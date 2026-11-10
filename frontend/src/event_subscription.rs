//! Read-only event subscription and verified snapshot recovery.
use crate::{protocol::Frame,request,reconnect_snapshot,next_id};
use serde_json::json;
use std::{os::unix::net::UnixStream,sync::{Arc,atomic::{AtomicBool,AtomicU64,Ordering},mpsc::SyncSender},thread,time::Duration};

pub fn run(mut event_stream:UnixStream,event_socket:String,event_key:String,event_release:String,
    initial_cursor:u64,event_revision:Arc<AtomicU64>,event_mutations:Arc<AtomicBool>,updates_tx:SyncSender<Frame>){
    let mut cursor=initial_cursor;let mut counter=0;let mut resync=false;
    loop{
        let id=next_id(&mut counter);
        if resync{
            match reconnect_snapshot(&event_socket,&event_key,&event_release,cursor){
                Ok((replacement,snapshot))=>{
                    cursor=snapshot.body["cursor"].as_u64().unwrap();
                    if let Some(value)=snapshot.body["state_revision"].as_u64(){event_revision.fetch_max(value,Ordering::SeqCst);}
                    if updates_tx.send(snapshot).is_err(){break;}
                    event_stream=replacement;event_mutations.store(true,Ordering::SeqCst);resync=false;
                },
                Err(_)=>{
                    if updates_tx.send(Frame::new("error",&id,json!({"code":"service_unavailable","message":"Still reconnecting to this project's verified service; showing last received state. Draft preserved; Ctrl-Q leaves."}))).is_err(){break;}
                    thread::sleep(Duration::from_millis(500));
                }
            }
            continue;
        }
        let response=request(&mut event_stream,"events_read",&id,json!({"after_seq":cursor,"limit":64,"wait_ms":1000}));
        let page=response.ok().filter(|frame|frame.kind=="events_result");
        let mut expected=cursor+1;
        let valid=page.as_ref().is_some_and(|frame|{
            let Some(events)=frame.body["events"].as_array()else{return false;};
            for item in events{if item["seq"].as_u64()!=Some(expected){return false;}expected+=1;}
            frame.body["next_seq"].as_u64()==Some(expected-1)
        });
        if !valid{
            // A busy/error reply is not an event page. Gaps and interrupted reads
            // also require a verified snapshot; none may strand the subscriber.
            event_mutations.store(false,Ordering::SeqCst);resync=true;
            if updates_tx.send(Frame::new("error",&id,json!({"code":"service_unavailable","message":"Live updates interrupted; reconnecting and verifying current state. Draft preserved; no call is resubmitted."}))).is_err(){break;}
            continue;
        }
        let frame=page.unwrap();cursor=expected-1;
        if let Some(value)=frame.body["state_revision"].as_u64(){event_revision.fetch_max(value,Ordering::SeqCst);}
        let changed=frame.body["events"].as_array().is_some_and(|events|!events.is_empty());
        if updates_tx.send(frame).is_err(){break;}
        if changed{
            let id=next_id(&mut counter);
            match request(&mut event_stream,"snapshot",&id,json!({})){
                Ok(snapshot) if snapshot.kind=="snapshot"&&snapshot.body["cursor"].as_u64().is_some_and(|latest|latest>=cursor)=>{
                    if updates_tx.send(snapshot).is_err(){break;}
                },
                _=>{
                    event_mutations.store(false,Ordering::SeqCst);resync=true;
                    if updates_tx.send(Frame::new("error",&id,json!({"code":"service_unavailable","message":"Snapshot refresh interrupted; reconnecting with draft preserved."}))).is_err(){break;}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use crate::{protocol,session};
    use serde_json::Value;
    use std::{os::unix::net::UnixListener,sync::{Mutex,mpsc},thread,time::{Duration,Instant}};
    #[test]
    fn event_subscription_recovers_busy_reply_and_exhausted_snapshot_batch_without_mutation(){
        let root=tempfile::tempdir().unwrap();let socket=root.path().join("peer.sock");
        let server=UnixListener::bind(&socket).unwrap();server.set_nonblocking(true).unwrap();
        let seen=Arc::new(Mutex::new(Vec::<String>::new()));let log=seen.clone();
        let owner=thread::spawn(move||{
            let deadline=Instant::now()+Duration::from_secs(4);let mut connections=0;
            while Instant::now()<deadline{
                let (mut peer,_)=match server.accept(){Ok(v)=>v,Err(_)=>{thread::sleep(Duration::from_millis(5));continue}};
                peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                let Ok(hello)=protocol::read_frame(&mut peer)else{continue};assert_eq!(hello.kind,"hello");
                protocol::write_frame(&mut peer,&Frame::new("hello_result",&hello.request_id,json!({"accepted":true}))).unwrap();
                let Ok(query)=protocol::read_frame(&mut peer)else{continue};log.lock().unwrap().push(query.kind.clone());
                connections+=1;
                if connections<=7{
                    protocol::write_frame(&mut peer,&Frame::new("error",&query.request_id,json!({"code":"busy","message":"Fixture service capacity reached"}))).unwrap();
                }else{
                    assert_eq!(query.kind,"snapshot");protocol::write_frame(&mut peer,&Frame::new("snapshot",&query.request_id,json!({"cursor":3,"state_revision":3,"fixture_completed":true}))).unwrap();
                    let Ok(query)=protocol::read_frame(&mut peer)else{continue};log.lock().unwrap().push(query.kind.clone());assert_eq!(query.kind,"events_read");assert_eq!(query.body["after_seq"],3);
                    let _=protocol::write_frame(&mut peer,&Frame::new("events_result",&query.request_id,json!({"events":[],"next_seq":3,"state_revision":3})));break;
                }
            }
        });
        let initial=session(socket.to_str().unwrap(),"project","release","events").unwrap();
        let(tx,rx)=mpsc::sync_channel(16);let allowed=Arc::new(AtomicBool::new(true));let revision=Arc::new(AtomicU64::new(1));
        let flag=allowed.clone();let rev=revision.clone();
        let watcher=thread::spawn(move||run(initial,socket.to_string_lossy().into_owned(),"project".into(),"release".into(),1,rev,flag,tx));
        let deadline=Instant::now()+Duration::from_secs(3);let mut recovered=false;let mut fenced=false;
        while Instant::now()<deadline{
            match rx.recv_timeout(Duration::from_millis(100)){
                Ok(frame) if frame.kind=="error"=>{fenced|=!allowed.load(Ordering::SeqCst);},
                Ok(frame) if frame.kind=="snapshot"&&frame.body["fixture_completed"]==Value::Bool(true)=>{recovered=true;break;},
                Err(mpsc::RecvTimeoutError::Disconnected)=>break,
                _=>{}
            }
        }
        drop(rx);watcher.join().unwrap();owner.join().unwrap();
        assert!(fenced);assert!(recovered,"A transient busy reply permanently stopped live updates");
        assert_eq!(revision.load(Ordering::SeqCst),3);
        assert!(seen.lock().unwrap().iter().all(|kind|matches!(kind.as_str(),"snapshot"|"events_read")),"Recovery must not resend commands or model calls");
    }
}
