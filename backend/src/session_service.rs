//! One project-scoped service owner. The socket serves short authoritative
//! transactions; the scheduler reconciles external execution on another thread.
//! This module never inspects the global project registry or another project.
use crate::{codec::{canonical_sha256,sha256},execution::{self,FileLock,read,write},host_attempt::Attempt,
    output_capture,view_registry,
    systemd_host::UserManager,mock_coordinator,
    db_writer::DbWriter,herdr::Herdr,operation::{self,Ensure,ErrorKind,OwnerFence},session_store::Session};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{fs,io::{Read,Seek,SeekFrom},os::{fd::{AsRawFd,RawFd},unix::{fs::PermissionsExt,net::{UnixStream,UnixListener as StdUnixListener}}},
    path::{Path,PathBuf},sync::{Arc,Mutex,atomic::{AtomicBool,AtomicUsize,Ordering}},time::Duration};
use tokio::{io::{AsyncReadExt,AsyncWriteExt},net::{UnixListener,UnixStream as AsyncUnixStream},
    sync::{Semaphore,watch},task::JoinSet,time::{self,timeout}};
#[cfg(test)]use std::io::Write;

use crate::ipc_limits::{REQUEST_FRAME_MAX as FRAME_MAX,RESPONSE_FRAME_MAX,COMMAND_MAX};
const MAX_CONNECTIONS:usize=64;
const MAX_BLOCKING_DISPATCH:usize=16;
struct ReadPermit<'a>(&'a AtomicUsize);
impl Drop for ReadPermit<'_>{fn drop(&mut self){self.0.fetch_sub(1,Ordering::SeqCst);}}

pub struct Config {pub project:PathBuf,pub state:PathBuf,pub key:String,pub release:String,
    pub ipc:PathBuf,pub herdr_socket:PathBuf,pub runner:PathBuf,pub frontend:PathBuf}
pub struct Service {pub session:Session,pub config:Config,pub generation:u64,
    writer:Arc<DbWriter>,shutting_down:AtomicBool,shutdown_signal:watch::Sender<bool>,
    dispatch_gate:Mutex<()>,attach_gate:Mutex<()>,output_reads:AtomicUsize,host:bool,conversation:bool}

fn frame(kind:&str,id:&str,body:Value)->Value{json!({"v":1,"kind":kind,"request_id":id,"body":body})}
#[cfg(test)]
fn read_frame(peer:&mut UnixStream)->Result<Option<Value>,String>{
    let mut raw=Vec::new();let mut byte=[0u8];
    loop{
        match peer.read(&mut byte){Ok(0) if raw.is_empty()=>return Ok(None),Ok(0)=>return Err("Truncated frame".into()),
            Ok(_)=>{},Err(e)=>return Err(e.to_string())}
        if byte[0]==b'\n'{break;}
        if raw.len()>=FRAME_MAX{return Err("Frame exceeds 1 MiB".into());}
        raw.push(byte[0]);
    }
    let value:Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
    if value["v"]!=1 || value["request_id"].as_str().is_none_or(str::is_empty) ||
        !value["body"].is_object(){return Err("Invalid protocol envelope".into());}
    Ok(Some(value))
}
#[cfg(test)]
fn send(peer:&mut UnixStream,value:&Value)->Result<(),String>{
    let mut bytes=response_bytes(value)?;
    bytes.push(b'\n');peer.write_all(&bytes).map_err(|e|e.to_string())
}
fn response_bytes(value:&Value)->Result<Vec<u8>,String>{
    let bytes=serde_json::to_vec(value).map_err(|e|e.to_string())?;
    if bytes.len()<=RESPONSE_FRAME_MAX{return Ok(bytes);}
    // A bounded correlated error keeps the peer usable and never substitutes
    // a partial snapshot for complete state or authorizes a retry/mutation.
    serde_json::to_vec(&frame("error",value["request_id"].as_str().unwrap_or("response"),json!({
        "code":"response_too_large","message":"Service reply exceeds the 8 MiB response budget; use scoped task/history reads. Project state is preserved.",
        "response_kind":value["kind"],"response_bytes":bytes.len(),"limit_bytes":RESPONSE_FRAME_MAX
    }))).map_err(|e|e.to_string())
}
#[cfg(test)]
#[test]
fn oversize_response_is_correlated_and_keeps_connection_usable(){
    let (mut service,mut client)=UnixStream::pair().unwrap();
    send(&mut service,&frame("snapshot","large-read",json!({"history":"x".repeat(RESPONSE_FRAME_MAX)}))).unwrap();
    let reply=read_frame(&mut client).unwrap().unwrap();
    assert_eq!(reply["kind"],"error");assert_eq!(reply["request_id"],"large-read");
    assert_eq!(reply["body"]["code"],"response_too_large");assert_eq!(reply["body"]["limit_bytes"],RESPONSE_FRAME_MAX);
    send(&mut service,&frame("session.status_result","next-read",json!({"project_key":"fixture"}))).unwrap();
    assert_eq!(read_frame(&mut client).unwrap().unwrap()["request_id"],"next-read");
}
async fn read_frame_async(peer:&mut AsyncUnixStream)->Result<Option<Value>,String>{
    let mut raw=Vec::new();let mut byte=[0u8];
    loop{
        match peer.read(&mut byte).await{Ok(0) if raw.is_empty()=>return Ok(None),
            Ok(0)=>return Err("Truncated frame".into()),Ok(_)=>{},Err(e)=>return Err(e.to_string())}
        if byte[0]==b'\n'{break;}
        if raw.len()>=FRAME_MAX{return Err("Frame exceeds 1 MiB".into());}
        raw.push(byte[0]);
    }
    let value:Value=serde_json::from_slice(&raw).map_err(|e|e.to_string())?;
    if value["v"]!=1 || value["request_id"].as_str().is_none_or(str::is_empty) ||
        !value["body"].is_object(){return Err("Invalid protocol envelope".into());}
    Ok(Some(value))
}
async fn send_async(peer:&mut AsyncUnixStream,value:&Value)->Result<(),String>{
    let mut bytes=response_bytes(value)?;
    bytes.push(b'\n');peer.write_all(&bytes).await.map_err(|e|e.to_string())
}
fn current_uid(fd:RawFd)->Result<bool,String>{
    let mut creds:libc::ucred=unsafe{std::mem::zeroed()};
    let mut len=std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe{libc::getsockopt(fd,libc::SOL_SOCKET,libc::SO_PEERCRED,
        (&raw mut creds).cast(),&mut len)}!=0{return Err(std::io::Error::last_os_error().to_string());}
    Ok(creds.uid==unsafe{libc::geteuid()})
}
fn peer_pid(fd:RawFd)->Result<u32,String>{
    let mut creds:libc::ucred=unsafe{std::mem::zeroed()};
    let mut len=std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe{libc::getsockopt(fd,libc::SOL_SOCKET,libc::SO_PEERCRED,
        (&raw mut creds).cast(),&mut len)}!=0{return Err(std::io::Error::last_os_error().to_string());}
    Ok(creds.pid as u32)
}

// UI compatibility is a wire/storage contract, not an application-build pin.
// The socket already authenticates the same UID. Still verify the actual peer's
// executable against its paired manifest instead of trusting a payload path.
fn compatible_frontend(executable:&Path)->Result<bool,String>{
    if executable.file_name().is_none_or(|n|n!="waywr") {return Ok(false);}
    let Some(root)=executable.parent().filter(|p|p.file_name().is_some_and(|n|n=="bin")).and_then(Path::parent) else{return Ok(false)};
    let manifest=match read(&root.join("manifest.json")){Ok(v)=>v,Err(_)=>return Ok(false)};
    Ok(manifest["schema_version"]==1 && manifest["backend_kind"]=="rust" && manifest["protocol_major"]==1 &&
        manifest["interaction_contract"]=="conversation-v1" &&
        manifest["files"]["bin/waywr"]==execution::hash_file(executable)?)
}

impl Service {
    fn write_dispatch_phase(&self,phase:&Value)->Result<(),String>{
        if !self.host{return Ok(());}
        let path=self.session.workflow.state.join("supervision/fence.lock");
        let deadline=std::time::Instant::now()+Duration::from_millis(250);
        let _fence=loop{match FileLock::exclusive(&path){
            Ok(guard)=>break guard,
            Err(error) if error.starts_with("Already owned:") && std::time::Instant::now()<deadline=>
                std::thread::sleep(Duration::from_millis(2)),
            Err(error)=>return Err(error),
        }};
        write(&self.session.workflow.state.join("session/dispatch-gate.json"),
            &json!({"schema_version":1,"phase":phase["phase"],
                "owner_generation":self.generation,"lifecycle_generation":phase["generation"]}),false)
    }
    pub fn new(config:Config)->Result<Self,String>{
        if !config.runner.is_file() || !config.frontend.is_file(){return Err("Paired runner/frontend missing".into());}
        let contract=config.state.join("state-contract.json");
        let (host,conversation)=if contract.exists(){
            let value=read(&contract)?;
            if value["schema_version"]!=1 || !matches!(value["state_schema"].as_u64(),Some(3|4|5|6|7|8|9)) ||
                value["execution_host"]!="systemd-user-v1" || value["project_key"]!=config.key{
                return Err("Project execution contract does not match selected state".into());
            }
            (true,matches!(value["state_schema"].as_u64(),Some(4|5|6|7|8|9)))
        }else{(false,false)};
        if !host && config.herdr_socket.as_os_str().is_empty(){
            return Err("Legacy project service requires its bound Herdr socket".into());
        }
        let mut session=Session::open(&config.project,&config.state,&config.key,&config.release)?;
        let writer=DbWriter::start(config.project.clone(),config.state.clone(),config.key.clone(),config.release.clone())?;
        session.attach_writer(writer.clone());
        let owner=session.workflow.state.join("supervision/owner.json");
        let generation=if owner.exists(){read(&owner)?["generation"].as_u64().ok_or("invalid owner generation")?+1}else{1};
        let (shutdown_signal,_)=watch::channel(false);
        Ok(Self{session,config,generation,writer,shutting_down:AtomicBool::new(false),
            shutdown_signal,dispatch_gate:Mutex::new(()),attach_gate:Mutex::new(()),
            output_reads:AtomicUsize::new(0),host,conversation})
    }
    fn publish_owner(&self)->Result<(),String>{
        let phase=if self.host{Some(self.session.phase()?)}else{None};
        let _fence=FileLock::exclusive(&self.session.workflow.state.join("supervision/fence.lock"))?;
        write(&self.session.workflow.state.join("supervision/owner.json"),
            &json!({"schema_version":1,"generation":self.generation,"project_key":self.session.key,
                "release_id":self.session.release}),false)?;
        if self.host{
            let phase=phase.as_ref().ok_or("missing host lifecycle phase")?;
            write(&self.session.workflow.state.join("session/dispatch-gate.json"),
                &json!({"schema_version":1,"phase":phase["phase"],
                    "owner_generation":self.generation,"lifecycle_generation":phase["generation"]}),false)?;
        }
        Ok(())
    }
    pub fn serve(self:Arc<Self>)->Result<(),String>{
        let runtime=tokio::runtime::Builder::new_multi_thread().worker_threads(2)
            .max_blocking_threads(MAX_BLOCKING_DISPATCH+2).enable_all().build().map_err(|e|e.to_string())?;
        runtime.block_on(self.serve_async())
    }
    async fn serve_async(self:Arc<Self>)->Result<(),String>{
        let service=self.clone();
        let (_owner,std_listener)=tokio::task::spawn_blocking(move||->Result<_,String>{
            let owner=FileLock::exclusive(&service.session.workflow.state.join("session/service.lock"))?;
            service.publish_owner()?;
            if service.conversation{service.session.conversation_retire_stranded_mock()?;}
            let parent=service.config.ipc.parent().ok_or("IPC socket has no parent")?;
            fs::create_dir_all(parent).map_err(|e|e.to_string())?;
            fs::set_permissions(parent,fs::Permissions::from_mode(0o700)).map_err(|e|e.to_string())?;
            if service.config.ipc.exists(){
                match UnixStream::connect(&service.config.ipc){
                    Ok(_)=>return Err("Another project service answers at the configured socket".into()),
                    Err(_)=>fs::remove_file(&service.config.ipc).map_err(|e|e.to_string())?,
                }
            }
            let listener=StdUnixListener::bind(&service.config.ipc).map_err(|e|e.to_string())?;
            listener.set_nonblocking(true).map_err(|e|e.to_string())?;
            fs::set_permissions(&service.config.ipc,fs::Permissions::from_mode(0o600)).map_err(|e|e.to_string())?;
            Ok((owner,listener))
        }).await.map_err(|e|e.to_string())??;
        let listener=UnixListener::from_std(std_listener).map_err(|e|e.to_string())?;
        let scheduler=tokio::spawn(self.clone().schedule_async());
        let history=if self.session.development(){Some(tokio::spawn(self.clone().project_history_async()))}else{None};
        let dispatch_slots=Arc::new(Semaphore::new(MAX_BLOCKING_DISPATCH));
        let mut clients=JoinSet::new();
        let mut term=tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).map_err(|e|e.to_string())?;
        let mut interrupt=tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).map_err(|e|e.to_string())?;
        while !self.shutting_down.load(Ordering::SeqCst){
            tokio::select!{
                accepted=listener.accept()=>{
                    let (mut peer,_)=accepted.map_err(|e|e.to_string())?;
                    if clients.len()>=MAX_CONNECTIONS{
                        let request=timeout(Duration::from_millis(250),read_frame_async(&mut peer)).await;
                        let id=request.ok().and_then(Result::ok).flatten()
                            .and_then(|v|v["request_id"].as_str().map(str::to_owned))
                            .unwrap_or_else(||"overload".into());
                        let _=timeout(Duration::from_millis(250),send_async(&mut peer,&frame("error",&id,
                            json!({"code":"busy","message":"Project service client limit reached before acceptance"})))).await;
                        continue;
                    }
                    let service=self.clone();let slots=dispatch_slots.clone();
                    clients.spawn(async move{let _=service.handle_async(peer,slots).await;});
                }
                _=time::sleep(Duration::from_millis(50))=>{},
                _=term.recv()=>{self.shutting_down.store(true,Ordering::SeqCst);let _=self.shutdown_signal.send(true);},
                _=interrupt.recv()=>{self.shutting_down.store(true,Ordering::SeqCst);let _=self.shutdown_signal.send(true);},
                Some(joined)=clients.join_next(),if !clients.is_empty()=>{let _=joined;},
            }
        }
        let deadline=time::Instant::now()+Duration::from_secs(30);
        while !clients.is_empty(){
            if timeout(deadline.saturating_duration_since(time::Instant::now()),clients.join_next()).await.is_err(){
                let path=self.session.workflow.state.join("session/client-drain-attention.json");
                let _=tokio::task::spawn_blocking(move||write(&path,
                    &json!({"schema_version":1,"reason":"Client drain exceeded 30 seconds",
                        "operator_action":"Reconcile command IDs and operation evidence after service restart"}),false)).await;
                return Err("Client drain exceeded 30 seconds; retained IDs require reconciliation".into());
            }
        }
        scheduler.await.map_err(|e|e.to_string())?;
        if let Some(history)=history{history.await.map_err(|e|e.to_string())?;}
        let writer=self.writer.clone();let ipc=self.config.ipc.clone();
        tokio::task::spawn_blocking(move||->Result<(),String>{
            let outcome=writer.shutdown();
            if ipc.exists(){fs::remove_file(&ipc).map_err(|e|e.to_string())?;}
            outcome
        }).await.map_err(|e|e.to_string())??;
        Ok(())
    }
    async fn handle_async(self:Arc<Self>,mut peer:AsyncUnixStream,slots:Arc<Semaphore>)->Result<(),String>{
        let mut shutdown=self.shutdown_signal.subscribe();
        if !current_uid(peer.as_raw_fd())?{return Err("Project service peer UID mismatch".into());}
        let pid=peer_pid(peer.as_raw_fd())?;
        let hello=timeout(Duration::from_secs(10),read_frame_async(&mut peer)).await
            .map_err(|_|"Hello timed out".to_owned())??.ok_or("Missing hello")?;
        let body=&hello["body"];
        let view=if body["channel"]=="view" && self.host{
            let state=self.session.workflow.state.clone();let key=self.session.key.clone();
            let id=body["view_id"].as_str().unwrap_or("").to_owned();
            let token=body["view_token"].as_str().unwrap_or("").to_owned();
            tokio::task::spawn_blocking(move||view_registry::authorize(&state,&key,&id,&token))
                .await.map_err(|e|e.to_string())?.ok()
        }else{None};
        let allowed=hello["kind"]=="hello" && body["protocol"]["major"]==1 &&
            body["project_key"]==self.session.key && body["release_id"]==self.session.release &&
            (!self.session.conversation_native() || body["protocol"]["minor"].as_u64().unwrap_or(0)>=if crate::provider_selection::enabled(&self.session){10}else if self.session.development(){6}else if self.session.continuous(){5}else if self.session.scope_enabled(){4}else{3}) &&
            (["command","events"].iter().any(|s|body["channel"]==*s) || view.is_some());
        let id=hello["request_id"].as_str().ok_or("invalid request ID")?;
        timeout(Duration::from_secs(10),send_async(&mut peer,&frame(if allowed{"hello_result"}else{"error"},id,
            json!({"accepted":allowed,"owner_generation":self.generation,"state_schema":if crate::provider_selection::enabled(&self.session){9}else if self.session.development(){8}else if self.session.continuous(){7}else if self.session.scope_enabled(){6}else if self.session.conversation_chat(){5}else if self.conversation{4}else if self.host{3}else{2},
                "protocol":{"major":1,"minor":if crate::provider_selection::enabled(&self.session){10}else if self.session.development(){6}else if self.session.continuous(){5}else if self.session.scope_enabled(){4}else if self.session.conversation_chat(){3}else if self.conversation{2}else if self.host{1}else{0},
                    "capabilities":if crate::provider_selection::enabled(&self.session){json!(["host_operations_v1","output_pages_v1","conversation_v1","task_scope_v1","continuous_scope_v1","development_v1","request_history_v1","provider_selection_v1","conversational_coordination_v1","coordinator_recovery_v1","ordinary_project_workflow_v1"])}else if self.session.development(){json!(["host_operations_v1","output_pages_v1","conversation_v1","task_scope_v1","continuous_scope_v1","development_v1","request_history_v1"])}else if self.session.continuous(){json!(["host_operations_v1","output_pages_v1","conversation_v1","claude_chat_v1","task_scope_v1","continuous_scope_v1"])}else if self.session.scope_enabled(){json!(["host_operations_v1","output_pages_v1","conversation_v1","claude_chat_v1","task_scope_v1"])}else if self.session.conversation_chat(){json!(["host_operations_v1","output_pages_v1","conversation_v1","claude_chat_v1"])}else if self.conversation{json!(["host_operations_v1","output_pages_v1","artifact_pages_v1","herdr_views_v1","conversation_v1","file_candidates_v1"])}
                        else if self.host{json!(["host_operations_v1","output_pages_v1","artifact_pages_v1","herdr_views_v1"])}else{json!([])}},
                "code":if allowed{Value::Null}else{json!("protocol_mismatch")}})))).await
            .map_err(|_|"Hello reply timed out".to_owned())??;
        if !allowed{return Ok(());}
        loop {
            if *shutdown.borrow(){return Ok(());}
            let request=tokio::select!{
                value=timeout(Duration::from_secs(10),read_frame_async(&mut peer))=>
                    value.map_err(|_|"Client frame timed out".to_owned())??,
                _=shutdown.wait_for(|value|*value)=>return Ok(()),
            };
            let Some(request)=request else{return Ok(());};
            let id=request["request_id"].as_str().ok_or("missing request ID")?.to_owned();
            let wait=if request["kind"]=="events_read"{request["body"]["wait_ms"].as_u64().unwrap_or(0).min(1000)}else{0};
            let permit=match slots.clone().try_acquire_owned(){Ok(permit)=>permit,Err(_)=>{
                timeout(Duration::from_secs(10),send_async(&mut peer,&frame("error",&id,
                    json!({"code":"busy","message":"Project service dispatch capacity reached before acceptance"})))).await
                    .map_err(|_|"Busy reply timed out".to_owned())??;continue;
            }};
            let service=self.clone();let view_grant=view.clone();
            let (kind,mut body)=tokio::task::spawn_blocking(move||{let _permit=permit;
                if let Some(grant)=view_grant.as_ref(){service.dispatch_view(&request,grant)}
                else{service.dispatch(&request,pid)}
            })
                .await.map_err(|e|e.to_string())??;
            if wait>0 && kind=="events_result" && body["events"].as_array().is_some_and(Vec::is_empty){
                time::sleep(Duration::from_millis(wait)).await;
                let service=self.clone();let after=body["next_seq"].as_u64().unwrap_or(0);
                body=tokio::task::spawn_blocking(move||service.session.events(after,64))
                    .await.map_err(|e|e.to_string())??;
            }
            timeout(Duration::from_secs(10),send_async(&mut peer,&frame(&kind,&id,body))).await
                .map_err(|_|"Client reply timed out".to_owned())??;
        }
    }
    fn dispatch(&self,request:&Value,pid:u32)->Result<(String,Value),String>{
        let kind=request["kind"].as_str().ok_or("missing request kind")?;
        let body=&request["body"];
        let result=(||->Result<(&str,Value),String>{match kind{
            "snapshot"=>{
                if self.conversation{
                    let mut coherent=None;
                    for _ in 0..3{
                        let mut snapshot=self.session.snapshot()?;
                        let conversation=self.session.conversation_snapshot()?;
                        if snapshot["state_revision"]==conversation["state_revision"]{
                            snapshot["conversation"]=conversation;
                            coherent=Some(snapshot);break;
                        }
                    }
                    Ok(("snapshot",coherent.ok_or("busy: Conversation changed during snapshot; retry read")?))
                }else{Ok(("snapshot",self.session.snapshot()?))}
            },
            "conversation.read"=>if self.conversation{Ok(("conversation.result",self.session.conversation_snapshot()?))}
                else{Ok(("error",json!({"code":"capability_unavailable","message":"Conversation requires schema 4"})))},
            "conversation.activity"=>if self.conversation{Ok(("conversation.activity_result",self.session.conversation_activity(body["turn_id"].as_str().ok_or("missing turn ID")?)?))}
                else{Ok(("error",json!({"code":"capability_unavailable","message":"Conversation requires schema 4"})))},
            "events_read"=>{
                let after=body["after_seq"].as_u64().ok_or("invalid event cursor")?;
                let limit=body["limit"].as_u64().ok_or("invalid event limit")?;
                Ok(("events_result",self.session.events(after,limit)?))
            }
            "command_get"=>Ok(("command_result",self.session.command_get(body["command_id"].as_str().ok_or("missing command ID")?)?)),
            "session.status"=>Ok(("session.status_result",json!({"session":self.session.phase()?,
                "checkpoint":self.session.checkpoint()?,"project_key":self.session.key}))),
            "task.get"=>Ok(("task.result",self.session.task_detail(body["task_id"].as_str().ok_or("missing task ID")?)?)),
            "operation.list"=>Ok(("operation.list_result",self.operation_list(body)?)),
            "operation.inspect"=>Ok(("operation.inspect_result",self.operation_inspect(body)?)),
            "operation.output.read"=>Ok(("operation.output.page",self.output_page(body)?)),
            "artifact.read"=>Ok(("artifact.page",self.artifact_page(body)?)),
            "view.prepare"=>Ok(("view.prepare_result",self.view_prepare(body)?)),
            "view.sent"=>Ok(("view.sent_result",self.view_sent(body)?)),
            "view.bind"=>Ok(("view.bind_result",self.view_bind(body)?)),
            "task.plan"=>{
                if self.session.scope_enabled(){return Ok(("error",json!({"code":"outside_increment","message":"Scope review uses task.get; execution planning is unavailable"})));}
                let task=body["task_id"].as_str().ok_or("missing task ID")?;
                let profile=body["profile"].as_str().unwrap_or(".waywr/workflow.toml");
                let plan=if self.host{
                    self.session.plan_host(task,profile,body["context"].as_str())?
                }else{
                    let workspace=self.bound_workspace()?;
                    self.session.plan(task,profile,body["context"].as_str(),
                        &workspace,&self.config.herdr_socket.to_string_lossy())?
                };
                Ok(("task.plan_result",json!({"plan_digest":canonical_sha256(&plan).map_err(|e|e.to_string())?,"plan":plan})))
            }
            "ui.attach"=>Err("Service-created UI tabs are unavailable in this release; run waywr in the invoking terminal".into()),
            "ui.attach_direct"=>Ok(("ui.attach_direct_result",self.attach_direct(pid,body)?)),
            "ui.register"=>Err("Service-created UI registration is unavailable in this release".into()),
            "command"=>{
                if serde_json::to_vec(request).map_err(|e|e.to_string())?.len()>COMMAND_MAX{
                    return Ok(("error",json!({"code":"invalid_request","message":"Command exceeds 64 KiB"})));}
                Ok(("command_result",self.command(body)?))
            }
            _=>Ok(("error",json!({"code":"invalid_request","message":"Unknown request"}))),
        }})();
        Ok(match result{Ok((kind,body))=>(kind.into(),body),Err(error)=>{
            let code=if error.starts_with("busy:") || error.contains("database is locked"){"busy"}
                else if error.starts_with("writer_unavailable:"){"writer_unavailable"}
                else{"invalid_request"};
            ("error".into(),json!({"code":code,"message":error,
                "execution_state":if code=="busy"{"not_accepted"}
                    else if code=="writer_unavailable"{"lookup_command_id"}else{"not_applicable"}}))
        }})
    }
    fn dispatch_view(&self,request:&Value,grant:&Value)->Result<(String,Value),String>{
        let body=&request["body"];
        if request["kind"]!="operation.output.read" ||
            body["task_id"]!=grant["task_id"] || body["stage"]!=grant["stage"] ||
            body["attempt_id"]!=grant["attempt_id"]{
            return Ok(("error".into(),json!({"code":"read_only_scope",
                "message":"Herdr viewer may read only its granted operation output"})));
        }
        match self.output_page(body){Ok(page)=>Ok(("operation.output.page".into(),page)),
            Err(error)=>Ok(("error".into(),json!({"code":"view_read_failed","message":error})))}
    }
    fn view_prepare(&self,body:&Value)->Result<Value,String>{
        if !self.host{return Err("Optional views require host execution contract".into());}
        let (attempt,id,stage)=self.operation_target(body)?;
        let _=attempt;
        let socket=body["socket"].as_str().ok_or("Explicit Herdr socket required")?;
        if !Path::new(socket).is_absolute(){return Err("Herdr socket must be absolute".into());}
        let server=Herdr::new(socket).server_identity()?;
        if body["server"]!=server{return Err("Herdr server identity changed before view intent".into());}
        if let Some(old)=body["reopen_after"].as_str(){
            let prior=view_registry::intent(&self.session.workflow.state,old)?;
            if prior["server"]!=server || prior["socket"]!=socket ||
                prior["task_id"]!=body["task_id"] || prior["stage"]!=stage ||
                prior["attempt_id"]!=id{
                return Err("Reopened view target differs from its prior grant".into());
            }
            let matches=Herdr::new(socket).find(prior["token"].as_str().ok_or("Prior view token missing")?,
                &self.config.frontend)?;
            if !matches.is_empty(){return Err("Prior read-only view still exists; focus it instead".into());}
        }
        view_registry::prepare(&self.session.workflow.state,&self.session.key,
            body["task_id"].as_str().ok_or("missing task ID")?,&stage,&id,
            &server,socket,&self.config.frontend,body["reopen_after"].as_str())
    }
    fn view_sent(&self,body:&Value)->Result<Value,String>{
        let id=body["view_id"].as_str().ok_or("Missing view ID")?;
        let grant=view_registry::intent(&self.session.workflow.state,id)?;
        if grant["project_key"]!=self.session.key{return Err("View belongs to another project".into());}
        view_registry::mark_sent(&self.session.workflow.state,id)
    }
    fn view_bind(&self,body:&Value)->Result<Value,String>{
        let id=body["view_id"].as_str().ok_or("Missing view ID")?;
        let grant=view_registry::intent(&self.session.workflow.state,id)?;
        if grant["project_key"]!=self.session.key{return Err("View belongs to another project".into());}
        if !view_registry::was_sent(&self.session.workflow.state,id){return Err("View creation not marked maybe-sent".into());}
        let socket=grant["socket"].as_str().ok_or("View endpoint missing")?;
        let herdr=Herdr::new(socket);
        if herdr.server_identity()!=Ok(grant["server"].clone()){
            return Err("Herdr view server identity changed".into());
        }
        let token=grant["token"].as_str().ok_or("View token missing")?;
        let found=herdr.find(token,&self.config.frontend)?;
        if found.len()!=1 || found[0]["pane_id"]!=body["pane_id"] ||
            found[0]["tab_id"]!=body["tab_id"] || found[0]["workspace_id"]!=body["workspace_id"]{
            return Err("Read-only view pane identity is not unique and verified".into());
        }
        let binding=json!({"schema_version":1,"view_id":id,"server":grant["server"],
            "pane_id":body["pane_id"],"tab_id":body["tab_id"],"workspace_id":body["workspace_id"]});
        view_registry::bind(&self.session.workflow.state,id,&binding)?;
        Ok(binding)
    }
    fn reader_permit(&self)->Result<ReadPermit<'_>,String>{
        self.output_reads.try_update(Ordering::SeqCst,Ordering::SeqCst,
            |current|(current<4).then_some(current+1))
            .map_err(|_|"busy: four project output readers already active".to_owned())?;
        Ok(ReadPermit(&self.output_reads))
    }
    fn coordinator_directory(&self,task_id:&str)->Result<PathBuf,String>{
        let root=self.session.workflow.state.join("session/coordinator").join(task_id);
        if !root.is_dir(){return Err("Selected task has no coordinator operation".into());}
        let mut found=Vec::new();
        for item in fs::read_dir(&root).map_err(|e|e.to_string())?{
            let item=item.map_err(|e|e.to_string())?;
            let name=item.file_name().to_string_lossy().to_string();
            let Some(version)=name.strip_prefix("call-v").and_then(|rest|rest.split('-').next())
                .and_then(|digits|digits.parse::<u64>().ok()) else{continue};
            let path=item.path();
            if path.join("operation.json").is_file(){found.push((version,name,path));}
        }
        found.sort_by(|a,b|a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        found.pop().map(|(_,_,path)|path).ok_or("Selected task has no coordinator operation".into())
    }
    fn operation_target(&self,body:&Value)->Result<(PathBuf,String,String),String>{
        let task_id=body["task_id"].as_str().ok_or("missing task ID")?;
        let stage=body["stage"].as_str().ok_or("missing operation stage")?;
        let development=self.session.development()&&stage.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_')&&stage.len()<=100&&self.session.workflow.state.join("session/development").join(task_id).join(stage).join("operation.json").is_file();
        if !development && !matches!(stage,"coordinator"|"draft"|"critique-1"|"revision-1"|"critique-2"|"revision-2"|
            "implement"|"review-1"|"repair"|"review-2"){
            return Err("Unknown project operation stage".into());
        }
        let task=self.session.task(task_id)?.ok_or("Task not found in this project")?;
        let directory=if development{self.session.workflow.state.join("session/development").join(task_id).join(stage)}else if stage=="coordinator"{
            self.coordinator_directory(task_id)?
        }else{
            let run=task["run_id"].as_str().ok_or("Task has no workflow run")?;
            self.session.workflow.run_dir(run)?.join("operations").join(stage)
        };
        if !directory.join("operation.json").is_file(){return Err("Selected operation has no frozen execution".into());}
        let operation=read(&directory.join("operation.json"))?;
        if operation["project_key"]!=self.session.key{
            return Err("Operation does not belong to this project".into());
        }
        let requested=body["attempt_id"].as_str();
        let (attempt,id)=if operation["schema_version"]==2{
            if !directory.join("active-host-attempt.json").exists(){
                return Err("Operation prepared; host attempt not yet published".into());
            }
            let active=Attempt::load(&self.session.workflow.state,&directory)?;
            let id=requested.unwrap_or(&active.id);
            if id.len()!=32 || !id.bytes().all(|c|c.is_ascii_hexdigit() && !c.is_ascii_uppercase()){
                return Err("Invalid host attempt identity".into());
            }
            let path=directory.join("attempts").join(id);
            let intent=read(&path.join("intent.json"))?;
            if intent["attempt_id"]!=id || intent["project_key"]!=self.session.key ||
                intent["operation_id"]!=operation["operation_id"]{
                return Err("Host attempt identity differs from operation".into());
            }
            (path,id.to_owned())
        }else{
            if requested.is_some_and(|value|value!="legacy"){
                return Err("Legacy operation has no host attempt identity".into());
            }
            (directory,"legacy".into())
        };
        Ok((attempt,id,stage.into()))
    }
    fn operation_list(&self,body:&Value)->Result<Value,String>{
        let task_id=body["task_id"].as_str().ok_or("missing task ID")?;
        self.session.task(task_id)?.ok_or("Task not found in this project")?;
        let mut operations=Vec::new();
        if self.session.development(){
            let active_only=body["active_only"].as_bool().unwrap_or(false);
            let active:std::collections::BTreeSet<String>=self.session.development_records("unit")?.into_iter()
                .filter(|unit|unit["work_id"]==task_id && unit["status"]=="running")
                .filter_map(|unit|unit["operation_id"].as_str().map(str::to_owned)).collect();
            let root=self.session.workflow.state.join("session/development").join(task_id);
            if root.is_dir(){
                let mut dirs=fs::read_dir(&root).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
                dirs.sort_by_key(|e|e.file_name());
                for entry in dirs {
                    let stage=entry.file_name().to_string_lossy().into_owned();
                    if active_only && !active.contains(&stage){continue;}
                    if !entry.path().join("operation.json").is_file(){continue;}
                    let operation=read(&entry.path().join("operation.json"))?;
                    if !entry.path().join("active-host-attempt.json").exists(){
                        operations.push(json!({"stage":stage,"attempt_id":null,"status":"prepared",
                            "output_available":false,"terminal":false,
                            "activity":{"status":"starting","label":"Starting · waiting for host attempt"}}));
                        continue;
                    }
                    let attempt=Attempt::load(&self.session.workflow.state,&entry.path())?;
                    let activity=if active.contains(&stage){crate::worker_activity::current(&attempt.directory)}else{Value::Null};
                    operations.push(json!({"stage":stage,"attempt_id":attempt.id,"provider":operation["provider"],"model":operation["model"],
                        "terminal":attempt.directory.join("receipt.json").exists(),
                        "output_available":attempt.directory.join("output.index").exists()||attempt.directory.join("transcript.log").exists(),
                        "output_limit":attempt.directory.join("output-limit.json").exists(),
                        "output_invalid":attempt.directory.join("output-invalid.json").exists(),"activity":activity}));
                }
            }
            return Ok(json!({"task_id":task_id,"active_only":active_only,"operations":operations}));
        }
        for stage in ["coordinator","draft","critique-1","revision-1","critique-2","revision-2",
            "implement","review-1","repair","review-2"]{
            let query=json!({"task_id":task_id,"stage":stage});
            let operation_dir=if stage=="coordinator"{
                match self.coordinator_directory(task_id){Ok(path)=>path,Err(_)=>continue}
            }else{
                let task=self.session.task(task_id)?.ok_or("Task not found in this project")?;
                let Some(run)=task["run_id"].as_str() else{continue};
                self.session.workflow.run_dir(run)?.join("operations").join(stage)
            };
            if operation_dir.join("operation.json").exists(){
                let operation=read(&operation_dir.join("operation.json"))?;
                if operation["schema_version"]==2 &&
                    !operation_dir.join("active-host-attempt.json").exists(){
                    operations.push(json!({"stage":stage,"attempt_id":null,
                        "status":"prepared; host attempt not yet published",
                        "terminal":false,"output_available":false,
                        "output_limit":false,"output_invalid":false}));
                    continue;
                }
                let (directory,id,_)=self.operation_target(&query)?;
                operations.push(json!({"stage":stage,"attempt_id":id,
                    "terminal":directory.join("receipt.json").exists(),
                    "output_available":directory.join("output.index").exists() ||
                        directory.join("transcript.log").exists(),
                    "output_limit":directory.join("output-limit.json").exists(),
                    "output_invalid":directory.join("output-invalid.json").exists()}));
            }
        }
        Ok(json!({"schema_version":1,"task_id":task_id,"operations":operations}))
    }
    fn operation_inspect(&self,body:&Value)->Result<Value,String>{
        let (directory,id,stage)=self.operation_target(body)?;
        let mut evidence=Vec::new();
        for name in ["intent.json","start-maybe-sent.json","claim.json","permit.json",
            "spawn-committed.json","child.json","receipt.json","retired.json",
            "uncertain.json","output-limit.json","output-invalid.json","recovery.json"]{
            if directory.join(name).exists(){evidence.push(name);}
        }
        Ok(json!({"schema_version":1,"task_id":body["task_id"],"stage":stage,
            "attempt_id":id,"evidence":evidence,
            "receipt":if directory.join("receipt.json").exists(){Some(read(&directory.join("receipt.json"))?)}else{None},
            "output_limit":if directory.join("output-limit.json").exists(){Some(read(&directory.join("output-limit.json"))?)}else{None}}))
    }
    fn output_page(&self,body:&Value)->Result<Value,String>{
        let _permit=self.reader_permit()?;
        let (directory,id,stage)=self.operation_target(body)?;
        let limit=body["limit"].as_u64().unwrap_or(16*1024) as usize;
        let page=output_capture::page(&directory,&id,body.get("cursor").filter(|v|!v.is_null()),limit)?;
        Ok(json!({"task_id":body["task_id"],"stage":stage,"page":page}))
    }
    fn artifact_page(&self,body:&Value)->Result<Value,String>{
        let _permit=self.reader_permit()?;
        let task_id=body["task_id"].as_str().ok_or("missing task ID")?;
        let task=self.session.task(task_id)?.ok_or("Task not found in this project")?;
        let run=task["run_id"].as_str().ok_or("Task has no workflow run")?;
        if let Some(state)=self.session.static_snapshot(run)?{
            let selected=body["artifact"].as_str().unwrap_or("latest");
            if selected!="latest"{
                let (contents,digest,manifest)=self.session.static_file_preview(run,selected)?;
                return Ok(json!({"schema_version":1,"task_id":task_id,"run_id":run,"artifact":selected,
                    "sha256":digest,"manifest_sha256":manifest,"size":contents.len(),"offset":0,
                    "next_offset":contents.len(),"encoding":"hex","data":output_capture::hex_bytes(contents.as_bytes()),
                    "complete":true,"bundle_review_receipt":false}));
            }
            let (preview,digest)=self.session.static_preview(run)?.ok_or("Static candidate unavailable")?;
            let size=preview.len() as u64;
            let offset=body["offset"].as_u64().unwrap_or(0);
            let limit=body["limit"].as_u64().unwrap_or(16*1024) as usize;
            if offset>size||limit==0||limit>output_capture::PAGE_LIMIT{return Err("Artifact cursor outside bound".into());}
            let end=(offset+limit as u64).min(size);
            if state["pending"]["kind"]=="final-review"{
                self.session.static_read_progress(run,state["pending"]["request_id"].as_str().ok_or("Missing request")?,
                    &digest,offset,end,size)?;
            }
            return Ok(json!({"schema_version":1,"task_id":task_id,"run_id":run,"artifact":"latest",
                "sha256":digest,"size":size,"offset":offset,"next_offset":end,
                "encoding":"hex","data":output_capture::hex_bytes(&preview.as_bytes()[offset as usize..end as usize]),
                "complete":end==size}));
        }
        let root=self.session.workflow.run_dir(run)?.canonicalize().map_err(|e|e.to_string())?;
        let chosen=body["artifact"].as_str().unwrap_or("latest");
        let reference=if chosen=="latest"{
            self.session.workflow.snapshot(run)?["state"]["current"].clone()
        }else{
            if !matches!(chosen,"draft"|"revision-1"|"revision-2"){
                return Err("Unknown project artifact selection".into());
            }
            read(&root.join("artifacts").join(format!("{chosen}.json")))?["manifest"].clone()
        };
        let relative=reference["path"].as_str().ok_or("Selected artifact has no path")?;
        let path=root.join(relative).canonicalize().map_err(|e|e.to_string())?;
        if !path.starts_with(&root){return Err("Artifact path leaves selected project run".into());}
        let mut file=fs::File::open(&path).map_err(|e|e.to_string())?;
        let size=file.metadata().map_err(|e|e.to_string())?.len();
        let mut hasher=Sha256::new();let mut buffer=[0u8;64*1024];
        loop{let amount=file.read(&mut buffer).map_err(|e|e.to_string())?;
            if amount==0{break;}hasher.update(&buffer[..amount]);}
        let digest=format!("{:x}",hasher.finalize());
        if reference["sha256"]!=digest{return Err("Artifact bytes differ from frozen reference; approval refused".into());}
        let offset=body["offset"].as_u64().unwrap_or(0);
        let limit=body["limit"].as_u64().unwrap_or(16*1024) as usize;
        if offset>size || limit==0 || limit>output_capture::PAGE_LIMIT{
            return Err("Artifact page cursor or size is outside its 64 KiB bound".into());
        }
        let take=(size-offset).min(limit as u64) as usize;
        let mut content=vec![0;take];file.seek(SeekFrom::Start(offset)).map_err(|e|e.to_string())?;
        file.read_exact(&mut content).map_err(|e|e.to_string())?;
        Ok(json!({"schema_version":1,"task_id":task_id,"run_id":run,"artifact":chosen,
            "sha256":digest,"size":size,"offset":offset,"next_offset":offset+take as u64,
            "encoding":"hex","data":output_capture::hex_bytes(&content),
            "complete":offset+take as u64==size}))
    }
    fn command(&self,body:&Value)->Result<Value,String>{
        if body["action"].as_str().is_some_and(|a|a.starts_with("conversation.")){return crate::live_input::command(&self.session,body)}
        if body["action"].as_str().is_some_and(|a|a.starts_with("coordination.")){return crate::coordination::command(&self.session,body)}
        let _development_scope_fence=if self.session.development()&&matches!(body["action"].as_str(),Some("save_proposal_revision"|"authorize_scope"|"answer_conversation")){Some(FileLock::exclusive(&self.session.workflow.state.join("supervision/fence.lock")).map_err(|e|format!("busy: Scope decision waits for serialized Git effect: {e}"))?)}else{None};
        if self.session.conversation_chat() && !matches!(body["action"].as_str(),Some("submit_message"|"select_coordinator"|"cancel_conversation"|"stop_session"|"resume_session")){
            return Ok(json!({"status":"rejected","code":"outside_increment","message":"This release supports coordinator conversation only; worker tasks, consultation and steering are deferred"}));
        }
        if self.session.continuous()&&matches!(body["action"].as_str(),Some("withdraw_queued"|"continue_held"|"confirm_question_disposition")){return self.session.dialogue_command(body)}
        if self.session.scope_enabled() && !self.session.development() && !matches!(body["action"].as_str(),Some("submit_message"|"select_coordinator"|"cancel_conversation"|"stop_session"|"resume_session"|"answer_conversation"|"save_proposal_revision"|"authorize_scope")){return Ok(json!({"status":"rejected","code":"outside_increment","message":"Scope only: worker execution, admission and consultation are unavailable"}));}
        if self.session.development()&&body["action"].as_str().is_some_and(|a|a.starts_with("candidate.")){let _fence=FileLock::exclusive(&self.session.workflow.state.join("supervision/fence.lock"))?;return crate::candidate::command(&self.session,body)}
        if self.session.development()&&body["action"]=="project.setup" {return crate::project_workflow::command(&self.session,body)}
        if self.session.development() && matches!(body["action"].as_str(),Some("development.resume"|"development.steer"|"development.reply"|"development.cancel")) {let _fence=FileLock::exclusive(&self.session.workflow.state.join("supervision/fence.lock"))?;return self.session.development_command(body)}
        if crate::provider_selection::enabled(&self.session) && body["action"].as_str().is_some_and(|a|a.starts_with("providers.")){
            let _fence=FileLock::exclusive(&self.session.workflow.state.join("supervision/fence.lock"))?;
            return crate::provider_selection::command(&self.session,body);
        }
        match body["action"].as_str().unwrap_or(""){
            "save_proposal_revision"|"authorize_scope"=>self.session.scope_command(body),
            "cancel_conversation"=>self.session.chat_cancel(body),
            "submit_message"=>if self.conversation{self.session.conversation_submit(body)}else{self.session.submit(body)},
            "answer_conversation"=>if self.conversation{self.session.conversation_answer(body)}
                else{Ok(json!({"status":"rejected","code":"capability_unavailable"}))},
            "authorize_consultation"=>if self.conversation{self.session.conversation_authorize_consultation(body)}
                else{Ok(json!({"status":"rejected","code":"capability_unavailable"}))},
            "apply_conversation_steering"=>if self.conversation{self.session.conversation_apply_steering(body)}
                else{Ok(json!({"status":"rejected","code":"capability_unavailable"}))},
            "select_coordinator"=>if self.conversation{self.session.conversation_select_coordinator(body)}
                else{Ok(json!({"status":"rejected","code":"capability_unavailable"}))},
            "steer_queued"=>self.session.steer_queued(body),
            "admit_task"=>(||->Result<Value,String>{
                let task=body["payload"]["task_id"].as_str().ok_or("missing task ID")?;
                let profile=body["payload"]["profile"].as_str().unwrap_or(".waywr/workflow.toml");
                let plan=if self.host{
                    self.session.plan_host(task,profile,body["payload"]["context"].as_str())?
                }else{
                    let workspace=self.bound_workspace()?;
                    self.session.plan(task,profile,body["payload"]["context"].as_str(),
                        &workspace,&self.config.herdr_socket.to_string_lossy())?
                };
                self.session.admit(body,&plan)
            })().or_else(|e|Ok(json!({"status":"rejected","code":"invalid_request","message":e}))),
            "answer_task"=>{
                let _gate=self.dispatch_gate.lock().map_err(|e|e.to_string())?;
                self.answer_task(body).or_else(|e|Ok(json!({"status":"rejected","code":"invalid_request","message":e})))
            },
            "release_recorded_answer"=>{
                let _gate=self.dispatch_gate.lock().map_err(|e|e.to_string())?;
                self.release_recorded_answer(body)
                .or_else(|e|Ok(json!({"status":"rejected","code":"invalid_request","message":e})))
            },
            "rebind_run"=>self.rebind_run(body)
                .or_else(|e|Ok(json!({"status":"rejected","code":"invalid_request","message":e}))),
            "recover_coordinator"=>self.recover_coordinator(body)
                .or_else(|e|Ok(json!({"status":"rejected","code":"invalid_request","message":e}))),
            "operation.retry_start"=>{
                let _gate=self.dispatch_gate.lock().map_err(|e|e.to_string())?;
                self.retry_start(body).or_else(|e|Ok(json!({"status":"rejected",
                    "code":"retry_refused","message":e})))
            },
            "cancel_task"=>{
                let _gate=self.dispatch_gate.lock().map_err(|e|e.to_string())?;
                self.cancel_task(body).or_else(|e|Ok(json!({"status":"rejected","code":"invalid_request","message":e})))
            },
            "stop_session"|"resume_session"=>{
                if body["action"]=="resume_session" && let Err(issue)=self.reconcile_before_resume(){return Ok(json!({"status":"rejected",
                        "code":"operation_uncertain","message":issue,
                        "operator_action":"Inspect retained operation records and resolve execution before resume"}));}
                let _guard=self.dispatch_gate.lock().map_err(|e|e.to_string())?;
                if self.shutting_down.load(Ordering::SeqCst){return Ok(json!({"status":"rejected","code":"service_stopping",
                    "message":"Project service is closing; reconnect and replay this command ID"}));}
                if self.host{
                    if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
                    if body["action"]=="stop_session"{
                        let current=self.session.phase()?;
                        if current["phase"]=="running"{
                            // Fence before the SQL command. A crash here is
                            // safe: the next owner reconciles SQL and the
                            // old runner cannot commit under its generation.
                            self.write_dispatch_phase(&json!({"phase":"exit-requested",
                                "generation":current["generation"].as_u64().unwrap_or(0)+1}))?;
                        }
                    }
                }
                let result=self.session.lifecycle_command(body);
                if self.host{
                    let actual=self.session.phase()?;
                    self.write_dispatch_phase(&actual)?;
                }
                let result=result?;
                drop(_guard);
                if result["status"]=="accepted"{self.session.checkpoint()?;}
                Ok(result)
            }
            _=>Ok(json!({"status":"rejected","code":"invalid_request","message":"Unknown action"})),
        }
    }
    fn bound_workspace(&self)->Result<String,String>{
        let _guard=self.attach_gate.lock().map_err(|e|e.to_string())?;
        let herdr=Herdr::new(&self.config.herdr_socket);
        let server=herdr.server_identity()?;
        let root=self.session.workflow.state.join("session");
        let pointer=root.join("workspace-current.json");
        let notice=root.join("workspace-routing-notice.json");
        if notice.exists(){return Err(format!("Worker workspace recovery needs inspection: {}",read(&notice)?));}
        if pointer.exists(){
            let prior=read(&pointer)?;
            if prior["project_key"]!=self.session.key{return Err("Worker workspace belongs to another project".into());}
            if !execution::identity_matches(&prior["server"],&server){
                return Err("Herdr server identity changed; reconcile project workspace before dispatch".into());
            }
            let id=prior["workspace_id"].as_str().ok_or("Worker workspace binding lacks ID")?;
            match herdr.workspace(id){Ok(_)=>return Ok(id.into()),Err(error) if !error.contains("workspace_not_found")=>return Err(error),Err(_)=>{}}
            // Keep the missing workspace as immutable evidence. A replacement
            // is safe only when existing frozen operations are reconciled by
            // the same routing rules used for an old UI workspace.
        }
        let intent_root=root.join("workspace-intents");
        let mut pending=Vec::new();
        if intent_root.is_dir(){for entry in fs::read_dir(&intent_root).map_err(|e|e.to_string())?{
            let path=entry.map_err(|e|e.to_string())?.path();
            if !root.join("workspace-bindings").join(path.file_name().ok_or("Invalid workspace intent")?).exists(){pending.push(path);}
        }}
        pending.sort();
        if pending.len()>1{return Err("Multiple unresolved project workspace intents; inspect before creating another".into());}
        let (intent_id,label)=if let Some(path)=pending.pop(){
            let intent=read(&path)?;
            if intent["project_key"]!=self.session.key || !execution::identity_matches(&intent["server"],&server){
                return Err("Unresolved workspace intent has a different project/server identity".into());
            }
            (intent["intent_id"].as_str().ok_or("Intent lacks ID")?.to_owned(),
                intent["label"].as_str().ok_or("Intent lacks label")?.to_owned())
        }else{
            let id=execution::random_token()?;
            let label=format!("Wayward Wrangler {} [{}]",self.session.project.file_name().unwrap_or_default().to_string_lossy(),&id[..8]);
            write(&intent_root.join(format!("{id}.json")),&json!({"schema_version":1,"project_key":self.session.key,
                "server":server,"intent_id":id,"label":label,"status":"may-have-been-sent"}),true)?;
            (id,label)
        };
        let listed=herdr.call("workspace.list",json!({}))?;
        let matches=listed["workspaces"].as_array().ok_or("Herdr omitted workspace list")?.iter()
            .filter(|item|item["label"]==label).collect::<Vec<_>>();
        if matches.len()>1{return Err("Multiple workspaces match one project intent".into());}
        let created=if let Some(existing)=matches.first(){
            let id=existing["workspace_id"].as_str().ok_or("Workspace lacks ID")?;
            let detail=herdr.workspace(id)?;
            json!({"workspace":detail["workspace"],"tab":{"tab_id":detail["workspace"]["active_tab_id"]}})
        }else{herdr.call("workspace.create",json!({"label":label,"cwd":self.session.project,"focus":false}))?};
        let workspace=created["workspace"]["workspace_id"].as_str().ok_or("Herdr omitted workspace ID")?;
        let binding=json!({"schema_version":1,"project_key":self.session.key,"server":server,
            "workspace_id":workspace,"initial_tab_id":created["tab"]["tab_id"],"intent_id":intent_id});
        write(&root.join("workspace-bindings").join(format!("{intent_id}.json")),&binding,true)?;
        write(&pointer,&binding,false)?;
        let target=json!({"workspace_id":workspace});
        let routing=self.rebind_active_if_safe(&target).and_then(|_|self.recover_prelaunch_coordinator(&target));
        if let Err(error)=routing{
            write(&notice,&json!({"schema_version":1,"workspace_id":workspace,"reason":error,
                "action":"Inspect frozen operations and resolve workspace routing before dispatch"}),true)?;
            return Err(format!("Worker workspace recovery needs inspection: {error}"));
        }
        Ok(workspace.into())
    }
    fn worker_target(&self,workspace:&str)->Result<Value,String>{
        let mut target=json!({"socket":self.config.herdr_socket,"workspace":workspace});
        let binding=self.session.workflow.state.join("session/workspace-current.json");
        if binding.exists(){
            let binding=read(&binding)?;
            if binding["project_key"]!=self.session.key || binding["workspace_id"]!=workspace{
                return Err("Worker workspace binding differs from selected project".into());
            }
            target["initial_tab_id"]=binding["initial_tab_id"].clone();
        }
        Ok(target)
    }
    fn attach_direct(&self,pid:u32,payload:&Value)->Result<Value,String>{
        let _guard=self.attach_gate.lock().map_err(|e|e.to_string())?;
        // Serialize new UI claims with the launcher's quiescent service handover.
        let _execution=FileLock::exclusive(&self.session.workflow.state.join("supervision/fence.lock"))?;
        let executable=fs::read_link(format!("/proc/{pid}/exe")).map_err(|e|e.to_string())?;
        if fs::canonicalize(&self.config.frontend).ok().as_ref()!=Some(&executable) &&
            !(self.host && compatible_frontend(&executable)?){
            return Err("Invoking UI does not support this project's interaction contract".into());
        }
        let process=execution::process_identity(pid)?;
        let herdr=Herdr::new(&self.config.herdr_socket);
        let server=if self.host{Value::Null}else{
            let server=herdr.server_identity()?;
            if herdr.call("ping",json!({}))?["protocol"]!=22{return Err("Unsupported Herdr API protocol".into());}
            server
        };
        let pane=&payload["pane"];
        if !self.host && !pane.is_null(){
            let id=pane["pane_id"].as_str().ok_or("Herdr pane ID required")?;
            let actual=herdr.pane(id)?;
            if actual["workspace_id"]!=pane["workspace_id"] || actual["tab_id"]!=pane["tab_id"]{
                return Err("Invoking pane identity differs from Herdr".into());
            }
            let processes=herdr.call("pane.process_info",json!({"pane_id":id}))?;
            if !processes["process_info"]["foreground_processes"].as_array()
                .is_some_and(|items|items.iter().any(|item|item["pid"]==pid)){
                return Err("Waywr is not the invoking Herdr pane foreground process".into());
            }
        }
        let root=self.session.workflow.state.join("session");
        let pointer=root.join("ui-current.json");
        if pointer.exists(){
            let prior=read(&pointer)?;
            let token=prior["token"].as_str().ok_or("Invalid existing UI token")?;
            let claim=root.join("ui-claims").join(format!("{token}.json"));
            if claim.exists() && execution::alive(&read(&claim)?["process"]){
                if prior["project_key"]==self.session.key && read(&claim)?["process"]==process && prior["pane"]==*pane{
                    return Ok(json!({"status":"attached","token":token,"reconnected":true}));
                }
                return Err(format!("Project UI is already attached in {:?}; detach it before opening another",prior["pane"]));
            }
        }
        let token=execution::random_token()?;
        let binding=json!({"schema_version":if self.host{3}else{2},"project_key":self.session.key,"token":token,
            "server":server,"pane":pane,"mode":"direct-terminal","process":process});
        write(&root.join("ui-bindings").join(format!("{token}.json")),&binding,true)?;
        write(&root.join("ui-claims").join(format!("{token}.json")),
            &json!({"schema_version":if self.host{3}else{2},"project_key":self.session.key,"token":token,
                "process":process,"pane":pane}),true)?;
        write(&pointer,&binding,false)?;
        Ok(json!({"status":"attached","token":token,"reconnected":false}))
    }
    fn rebind_active_if_safe(&self,pane:&Value)->Result<(),String>{
        let Some(task)=self.session.active()? else{return Ok(());};
        let run=task["run_id"].as_str().ok_or("active task lacks run")?;
        let current=self.session.workflow.effective_target(run)?;
        if current["workspace"]==pane["workspace_id"]{return Ok(());}
        let old=current["workspace"].as_str().ok_or("frozen target lacks workspace")?;
        let herdr=Herdr::new(&self.config.herdr_socket);
        match herdr.workspace(old){
            Ok(_)=>return Err("Old worker workspace still exists; automatic routing change refused".into()),
            Err(message) if !message.contains("workspace_not_found")=>return Err(format!("Cannot verify old workspace loss: {message}")),
            Err(_)=>{},
        }
        let target=json!({"socket":self.config.herdr_socket,"workspace":pane["workspace_id"]});
        self.session.workflow.append_routing(run,&target,"verified missing owned workspace on UI reconnect",self.generation)?;
        self.session.checkpoint()?;
        Ok(())
    }
    fn recover_prelaunch_coordinator(&self,pane:&Value)->Result<(),String>{
        let Some(call)=self.session.pending_coordinator()? else{return Ok(());};
        let task=self.session.task(call["task_id"].as_str().ok_or("invalid pending coordinator task")?)?
            .ok_or("pending coordinator task missing")?;
        let task_id=task["task_id"].as_str().ok_or("invalid task ID")?;
        let version=task["version"].as_u64().ok_or("invalid task version")?;
        let original=self.session.workflow.state.join("session/coordinator").join(task_id).join(format!("call-v{version}"));
        if !original.join("operation.json").exists(){return Ok(());}
        let operation=read(&original.join("operation.json"))?;
        let old=operation["target"]["workspace"].as_str().ok_or("coordinator target lacks workspace")?;
        if operation["target"]["workspace"]==pane["workspace_id"]{return Ok(());}
        let record=original.join("recovery.json");
        if record.exists(){
            let existing=read(&record)?;
            if existing["target"]["workspace"]==pane["workspace_id"]{return Ok(());}
            return Err("Coordinator already has a different recovery target; inspect evidence".into());
        }
        if ["launch.json","claim.json","permit.json","dispatch.json","child.json","receipt.json"].iter()
            .any(|name|original.join(name).exists()){
            return Err("Coordinator may have launched; workspace change cannot authorize another call".into());
        }
        match Herdr::new(&self.config.herdr_socket).workspace(old){
            Ok(_)=>return Err("Original coordinator workspace still exists; automatic rebind refused".into()),
            Err(message) if !message.contains("workspace_not_found")=>return Err(format!("Original workspace loss unverified: {message}")),
            Err(_)=>{},
        }
        let replacement=format!("call-v{version}-rebind-1");
        let target=json!({"socket":self.config.herdr_socket,"workspace":pane["workspace_id"]});
        write(&record,&json!({"schema_version":1,"reason":"verified prelaunch missing workspace",
            "original_operation_sha256":execution::hash_file(&original.join("operation.json"))?,
            "packet_sha256":operation["packet_sha256"],"replacement":replacement,
            "target":target,"owner_generation":self.generation}),true)?;
        let directory=original.parent().ok_or("coordinator has no parent")?.join(replacement);
        operation::prepare(&self.session.workflow.state,&directory,&read(&original.join("packet.json"))?,
            &target,Some(&self.session.key)).map_err(|e|e.detail)?;
        self.session.checkpoint()?;
        Ok(())
    }
    fn answer_task(&self,body:&Value)->Result<Value,String>{
        if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
        let payload=&body["payload"];
        let task_id=payload["task_id"].as_str().ok_or("missing task ID")?;
        let request=payload["request_id"].as_str().ok_or("missing request ID")?;
        let task=self.session.task(task_id)?.ok_or("Task not found")?;
        let run=task["run_id"].as_str().ok_or("Task has no run")?;
        if self.session.static_snapshot(run)?.is_some(){
            let saved=self.session.static_answer(body)?;
            if saved["status"]=="accepted"{self.session.checkpoint()?;}
            return Ok(saved);
        }
        let response_exists=self.session.workflow.response_exists(run,request)?;
        if !response_exists && (task["version"]!=body["expected_task_version"] || task["status"]!="awaiting-input" ||
            task["state_revision"]!=body["expected_state_revision"]){
            return Ok(json!({"status":"rejected","code":"stale_state"}));
        }
        let response=self.session.workflow.record_answer_mode(run,request,
            payload["outcome"].as_str().ok_or("missing answer outcome")?,
            payload["answer"].as_str().ok_or("missing answer")?,payload["artifact_sha256"].as_str(),
            payload["record_only"]==true)?;
        let outcome=json!({"status":"accepted","task_id":task_id,"run_id":run,
            "request_id":request,"response":response});
        let saved=self.session.record_external(body,&outcome)?;
        self.session.checkpoint()?;
        Ok(saved)
    }
    fn release_recorded_answer(&self,body:&Value)->Result<Value,String>{
        if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
        let task_id=body["payload"]["task_id"].as_str().ok_or("missing task ID")?;
        let request=body["payload"]["request_id"].as_str().ok_or("missing request ID")?;
        let task=self.session.task(task_id)?.ok_or("Task not found")?;
        let run=task["run_id"].as_str().ok_or("Task has no run")?;
        let release=self.session.workflow.release_record_only(run,request)?;
        let saved=self.session.record_external(body,&json!({"status":"accepted","task_id":task_id,
            "run_id":run,"request_id":request,"release":release}))?;
        self.session.checkpoint()?;Ok(saved)
    }
    fn rebind_run(&self,body:&Value)->Result<Value,String>{
        if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
        let run=body["payload"]["run_id"].as_str().ok_or("missing run ID")?;
        let requested=body["payload"]["workspace"].as_str().ok_or("missing workspace")?;
        let owned=self.bound_workspace()?;
        if requested!=owned{return Err("Workspace is not the currently verified project worker workspace".into());}
        let snapshot=self.session.workflow.snapshot(run)?;
        let pending=&snapshot["state"]["pending"];
        if snapshot["status"]!="awaiting-input" || pending["request_id"].as_str().is_none() ||
            self.session.workflow.response_exists(run,pending["request_id"].as_str().unwrap())?{
            return Err("Rebind requires an unanswered, quiescent human gate".into());
        }
        let target=json!({"socket":self.config.herdr_socket,"workspace":owned});
        let bound=self.session.workflow.append_routing(run,&target,"operator CLI rebind at unanswered gate",self.generation)?;
        let saved=self.session.record_external(body,&json!({"status":"accepted","run_id":run,"binding":bound}))?;
        self.session.checkpoint()?;Ok(saved)
    }
    fn recover_coordinator(&self,body:&Value)->Result<Value,String>{
        if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
        let id=body["payload"]["task_id"].as_str().ok_or("missing task ID")?;
        let pending=self.session.pending_coordinator()?.ok_or("No pending coordinator operation")?;
        if pending["task_id"]!=id{return Err("Selected task is not the pending coordinator".into());}
        let task=self.session.task(id)?.ok_or("Task missing")?;
        let version=task["version"].as_u64().ok_or("Task version missing")?;
        let original=self.session.workflow.state.join("session/coordinator").join(id)
            .join(format!("call-v{version}/operation.json"));
        if !original.exists(){return Err("Coordinator has no frozen operation to recover".into());}
        let workspace=self.bound_workspace()?;
        self.recover_prelaunch_coordinator(&json!({"workspace_id":workspace}))?;
        let saved=self.session.record_external(body,&json!({"status":"accepted","task_id":id,
            "recovery":"verified-prelaunch-only","workspace":workspace}))?;
        self.session.checkpoint()?;Ok(saved)
    }
    fn retry_start(&self,body:&Value)->Result<Value,String>{
        if !self.host{return Err("Explicit host retry is unavailable for this legacy execution contract".into());}
        if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
        let command=body["command_id"].as_str().ok_or("missing command ID")?;
        let task_id=body["payload"]["task_id"].as_str().ok_or("missing task ID")?;
        let task=self.session.task(task_id)?.ok_or("Task not found")?;
        let run=task["run_id"].as_str();
        let directory=if let Some(run)=run{
            let stage=self.session.workflow.snapshot(run)?["state"]["next_stage"]
                .as_str().ok_or("No pending workflow stage")?.to_owned();
            self.session.workflow.run_dir(run)?.join("operations").join(stage)
        }else{
            let version=task["version"].as_u64().ok_or("Missing coordinator task version")?;
            self.session.workflow.state.join("session/coordinator").join(task_id)
                .join(format!("call-v{version}"))
        };
        let grant_dir=directory.join("recovery-grants");
        let grant=grant_dir.join(format!("{}.json",sha256(command.as_bytes())));
        if grant.exists(){
            let saved=read(&grant)?;
            if saved["command_id"]!=command || saved["task_id"]!=task_id ||
                saved["operation_id"]!=read(&directory.join("operation.json"))?["operation_id"]{
                return Err("Infrastructure retry grant identity differs".into());
            }
            // A crash can land after the append-only grant and before the SQL
            // reopen/command ledger. Replaying the same ID completes that
            // transition without issuing another grant or dispatching a call.
            if task["status"]=="blocked"{
                if let Some(run)=run{
                    let status=self.session.workflow.snapshot(run)?["status"].as_str()
                        .ok_or("Workflow status missing during retry replay")?.to_owned();
                    if status=="blocked"{self.session.workflow.reopen_prelaunch(run)?;}
                    else if status!="running"{return Err("Workflow changed during retry replay".into());}
                    self.session.transition(task_id,run,"active",Some("Proven-no-launch infrastructure retry authorized"))?;
                }else{self.session.reopen_prelaunch_coordinator(task_id)?;}
            }
            let result=json!({"status":"accepted","task_id":task_id,"run_id":run,
                "grant":saved,"meaning":"retry only after positive no-launch proof; model budget unchanged"});
            let committed=self.session.record_external(body,&result)?;
            self.session.checkpoint()?;
            return Ok(committed);
        }
        if self.session.phase()?["phase"]!="running"{
            return Err("Resume this project session before explicit infrastructure retry".into());
        }
        if task["status"]!="blocked" ||
            task["version"]!=body["expected_task_version"] ||
            task["state_revision"]!=body["expected_state_revision"]{
            return Err("Task changed; inspect its current status before retry".into());
        }
        if run.is_some() && self.session.workflow.snapshot(run.unwrap())?["status"]!="blocked"{
            return Err("Workflow is not stopped for a proven prelaunch failure".into());
        }
        let attempt=Attempt::load(&self.session.workflow.state,&directory)?;
        let entries=fs::read_dir(directory.join("attempts")).map_err(|e|e.to_string())?
            .filter_map(Result::ok).filter(|entry|entry.path().join("intent.json").exists())
            .collect::<Vec<_>>();
        let grants=if grant_dir.exists(){fs::read_dir(&grant_dir).map_err(|e|e.to_string())?
            .filter_map(Result::ok).filter(|entry|entry.path().extension().is_some_and(|ext|ext=="json"))
            .count()}else{0};
        if entries.len()!=3*(grants+1) || entries.iter().any(|entry|{
            !entry.path().join("retired.json").exists() ||
            entry.path().join("spawn-committed.json").exists() ||
            entry.path().join("receipt.json").exists()
        }) || !attempt.directory.join("retired.json").exists() ||
            directory.join("invocation-reservation.json").exists(){
            return Err("Retry requires an exhausted group of positively retired, never-executed attempts".into());
        }
        fs::create_dir_all(&grant_dir).map_err(|e|e.to_string())?;
        let record=json!({"schema_version":1,"command_id":command,"task_id":task_id,
            "operation_id":read(&directory.join("operation.json"))?["operation_id"],
            "previous_attempt":attempt.id,"retired_attempts":entries.len(),
            "additional_infrastructure_attempts":3,"model_invocations_added":0,
            "time_unix_ms":(execution::now()*1000.0) as u64});
        write(&grant,&record,true)?;
        if let Some(run)=run{
            self.session.workflow.reopen_prelaunch(run)?;
            self.session.transition(task_id,run,"active",Some("Proven-no-launch infrastructure retry authorized"))?;
        }else{self.session.reopen_prelaunch_coordinator(task_id)?;}
        let result=json!({"status":"accepted","task_id":task_id,"run_id":run,"grant":record,
            "meaning":"retry only after positive no-launch proof; model budget unchanged"});
        let committed=self.session.record_external(body,&result)?;
        self.session.checkpoint()?;
        Ok(committed)
    }
    fn cancel_task(&self,body:&Value)->Result<Value,String>{
        if let Some(prior)=self.session.command_prior(body)?{return Ok(prior);}
        let task_id=body["payload"]["task_id"].as_str().ok_or("missing task ID")?;
        let task=self.session.task(task_id)?.ok_or("Task not found")?;
        if task["version"]!=body["expected_task_version"] || task["state_revision"]!=body["expected_state_revision"]{
            return Ok(json!({"status":"rejected","code":"stale_state"}));
        }
        if ["queued","proposed","ready"].iter().any(|s|task["status"]==*s){
            return self.session.cancel_undispatched(body);
        }
        let run=task["run_id"].as_str().ok_or("Task has no run")?;
        let static_state=self.session.static_snapshot(run)?;
        if static_state.is_some() && task["status"]=="awaiting-input"{
            self.session.static_stop(run,"cancelled","Operator cancelled pending static-file review")?;
            self.session.transition(task_id,run,"cancelled",Some("Operator cancelled pending static-file review"))?;
            let saved=self.session.record_external(body,&json!({"status":"accepted","task_id":task_id,"task_status":"cancelled"}))?;
            self.session.checkpoint()?;return Ok(saved);
        }
        if task["status"]=="awaiting-input"{
            let pending=self.session.workflow.snapshot(run)?["state"]["pending"]["request_id"]
                .as_str().ok_or("Pending gate lacks request ID")?.to_owned();
            if self.session.workflow.response_exists(run,&pending)?{
                return Ok(json!({"status":"rejected","code":"answer_pending_consumption",
                    "message":"A durable answer already exists; inspect or resume this task before cancellation"}));
            }
            self.session.workflow.stop(run,"cancelled","Operator abandoned pending human gate")?;
            self.session.transition(task_id,run,"cancelled",Some("Abandoned pending human gate"))?;
            let saved=self.session.record_external(body,&json!({"status":"accepted","task_id":task_id,"task_status":"cancelled"}))?;
            self.session.checkpoint()?;return Ok(saved);
        }
        if task["status"]!="active"{return Ok(json!({"status":"rejected","code":"not_cancellable_now"}));}
        let legacy_snapshot=if static_state.is_none(){Some(self.session.workflow.snapshot(run)?)}else{None};
        let stage=static_state.as_ref().map_or_else(||legacy_snapshot.as_ref().unwrap()["state"]["next_stage"].as_str(),
            |state|state["next_stage"].as_str()).ok_or("No current worker stage")?;
        let directory=self.session.workflow.run_dir(run)?.join("operations").join(stage);
        if self.host{
            if !directory.join("active-host-attempt.json").exists(){return Ok(json!({"status":"rejected",
                "code":"not_cancellable_now","message":"Host attempt has not been allocated"}));}
            let attempt=Attempt::load(&self.session.workflow.state,&directory)?;
            if !attempt.directory.join("claim.json").exists(){return Ok(json!({"status":"rejected",
                "code":"not_cancellable_now","message":"No verified host runner claim yet"}));}
            let claim=read(&attempt.directory.join("claim.json"))?;
            if !execution::alive(&claim["runner"]){return Ok(json!({"status":"rejected",
                "code":"operation_uncertain","message":"Host runner is gone without a validated receipt; inspect exact unit and child before cancellation"}));}
            let result=attempt.request_cancel(body["command_id"].as_str().ok_or("missing command ID")?)?;
            if result["status"]!="cancellation-requested"{
                return Ok(json!({"status":"rejected","code":"completed_race",
                    "message":"Host worker completed before cancellation; inspect saved result"}));
            }
            let saved=self.session.record_external(body,&json!({"status":"accepted","task_id":task_id,
                "task_status":"active","cancellation":result}))?;
            self.session.checkpoint()?;return Ok(saved);
        }
        if !directory.join("claim.json").exists(){return Ok(json!({"status":"rejected","code":"not_cancellable_now",
            "message":"No verified worker claim exists; wait for dispatch or the next gate"}));}
        match operation::cancel(&directory,&format!("project-session:{}:{task_id}",self.session.key)){
            Ok(result) if result["status"]=="cancellation-requested"=>{
                let saved=self.session.record_external(body,&json!({"status":"accepted","task_id":task_id,
                    "task_status":"active","cancellation":result}))?;
                self.session.checkpoint()?;Ok(saved)
            }
            Ok(_)=>Ok(json!({"status":"rejected","code":"completed_race",
                "message":"Worker completed before cancellation; inspect the saved result"})),
            Err(e)=>Ok(json!({"status":"rejected","code":if e.kind==ErrorKind::Uncertain{"operation_uncertain"}else{"completed_race"},
                "message":e.detail})),
        }
    }
    fn reconcile_before_resume(&self)->Result<(),String>{
        for root in [self.session.workflow.state.join("session/coordinator"),self.session.workflow.state.join("runs"),self.session.workflow.state.join("session/development")]{
            if !root.exists(){continue;}
            let mut dirs=Vec::new();
            for entry in fs::read_dir(&root).map_err(|e|e.to_string())?{
                let entry=entry.map_err(|e|e.to_string())?;
                let folder=if root.file_name().is_some_and(|n|n=="runs"){entry.path().join("operations")}else{entry.path()};
                if folder.is_dir(){for item in fs::read_dir(folder).map_err(|e|e.to_string())?{
                    dirs.push(item.map_err(|e|e.to_string())?.path());}}
            }
            for dir in dirs{
                if !dir.join("operation.json").exists(){continue;}
                if read(&dir.join("operation.json"))?["schema_version"]==2{
                    if !dir.join("active-host-attempt.json").exists(){continue;}
                    let attempt=Attempt::load(&self.session.workflow.state,&dir)?;
                    if attempt.validated_receipt()?.is_some() ||
                        attempt.directory.join("retired.json").exists(){continue;}
                    if !attempt.directory.join("start-maybe-sent.json").exists(){continue;}
                    let claim=read(&attempt.directory.join("claim.json")).map_err(|_|format!(
                        "{}: host start possibly sent without verified claim; inspect exact unit before resume",dir.display()))?;
                    if !execution::alive(&claim["runner"]){return Err(format!(
                        "{}: host runner not live and no receipt; execution uncertain",dir.display()));}
                    let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build()
                        .map_err(|e|e.to_string())?;
                    let status=runtime.block_on(async{UserManager::connect().await?.status(&attempt.unit).await})
                        .map_err(|e|format!("{}: exact host identity unavailable: {e}",dir.display()))?;
                    if status.name!=attempt.unit || status.invocation_id!=claim["invocation_id"] ||
                        status.main_pid!=claim["runner"]["pid"]{
                        return Err(format!("{}: host claim/unit identity conflict",dir.display()));
                    }
                    continue;
                }
                if dir.join("receipt.json").exists(){operation::receipt(&dir).map_err(|e|format!("{}: {}",dir.display(),e.detail))?;}
                else if ["launch.json","claim.json","permit.json"].iter().any(|n|dir.join(n).exists()){
                    let report=operation::inspect(&dir,None);
                    if report["status"]=="uncertain"{return Err(format!("{}: {}; resolve execution before resume",dir.display(),report["detail"]));}
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn live_steering_targets_the_reserved_turn_and_survives_reopen(){
        let root=tempfile::tempdir().unwrap();let project=root.path().join("project");let state=project.join(".waywr/state");fs::create_dir_all(&state).unwrap();
        let key="a".repeat(32);write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":9,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        write(&project.join(".waywr/coordinator.json"),&crate::model_runtime::initial_preferences().unwrap(),true).unwrap();
        let exe=std::env::current_exe().unwrap();let service=Service::new(Config{project:project.clone(),state:state.clone(),key:key.clone(),release:"fixture".into(),ipc:root.path().join("ipc"),herdr_socket:PathBuf::new(),runner:exe.clone(),frontend:exe}).unwrap();
        let c=service.session.workflow.connection().unwrap();
        c.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status,packet_sha256) VALUES('active',1,?1,'original',1,'reserved',?2)",rusqlite::params![key,"b".repeat(64)]).unwrap();
        c.execute("INSERT INTO dialogue_inputs(turn_id,subject_id) VALUES('active','discussion')",[]).unwrap();drop(c);
        let command=json!({"command_id":"live-guidance","action":"conversation.steer","payload":{"turn_id":"active","subject_id":"discussion","text":"Use the revised constraint"}});
        let result=service.command(&command).unwrap();assert_eq!(result["status"],"accepted", "{result}");assert_eq!(result["delivery"],"pending");
        assert_eq!(service.session.workflow.connection().unwrap().query_row("SELECT count(*) FROM conversation_messages WHERE message_id=?1",[result["input_id"].as_str()],|r|r.get::<_,i64>(0)).unwrap(),0,"Pending guidance must not leak into another turn's context before it is applied");
        assert_eq!(service.command(&command).unwrap(),result,"Replay must not create another guidance input");
        let mut foreign=command.clone();foreign["command_id"]=json!("foreign-guidance");foreign["payload"]["subject_id"]=json!("other");assert_eq!(service.command(&foreign).unwrap()["status"],"rejected");
        let saved=Session::open_existing(&project,&state,&key,"fixture").unwrap();
        let snapshot=saved.dialogue_state().unwrap();assert!(snapshot["queue"].as_array().unwrap().iter().any(|r|r["input_id"]==result["input_id"]&&r["text"]=="Use the revised constraint"));
        let withdraw=json!({"command_id":"withdraw-live-guidance","action":"conversation.withdraw_input","payload":{"input_id":result["input_id"],"subject_id":"discussion"}});
        assert_eq!(service.command(&withdraw).unwrap()["status"],"accepted");
        assert!(!saved.dialogue_state().unwrap()["queue"].as_array().unwrap().iter().any(|r|r["input_id"]==result["input_id"]));
        let mut second=command.clone();second["command_id"]=json!("guidance-claimed");let second=service.command(&second).unwrap();
        let directory=state.join("session/fixture-attempt");let input=second["input_id"].as_str().unwrap();
        fs::create_dir_all(directory.join("live-input/claimed")).unwrap();fs::write(directory.join("live-input/claimed").join(format!("{input}.json")),"claimed by fixture").unwrap();
        fs::create_dir_all(directory.join("live-input/pending")).unwrap();fs::write(directory.join("live-input/pending").join(format!("{input}.json")),"republished duplicate").unwrap();
        service.session.workflow.connection().unwrap().execute("UPDATE conversation_live_inputs SET directory=?1 WHERE input_id=?2",rusqlite::params![directory.to_str(),input]).unwrap();
        let mut late=withdraw;late["command_id"]=json!("withdraw-claimed");late["payload"]["input_id"]=json!(input);
        assert_eq!(service.command(&late).unwrap()["code"],"already_dispatched","Claimed guidance cannot be edited after delivery begins");
    }
    #[test]
    fn compatible_ui_uses_manifest_and_peer_binary_instead_of_build_identity(){
        let root=tempfile::tempdir().unwrap();let bin=root.path().join("bin");std::fs::create_dir(&bin).unwrap();
        let executable=bin.join("waywr");std::fs::write(&executable,b"fixture binary bytes").unwrap();
        let mut manifest=serde_json::json!({"schema_version":1,"backend_kind":"rust","protocol_major":1,
            "interaction_contract":"conversation-v1","release_id":"another-build",
            "files":{"bin/waywr":crate::execution::hash_file(&executable).unwrap()}});
        let path=root.path().join("manifest.json");crate::execution::write(&path,&manifest,false).unwrap();
        assert!(super::compatible_frontend(&executable).unwrap());
        manifest["protocol_major"]=serde_json::json!(2);crate::execution::write(&path,&manifest,false).unwrap();
        assert!(!super::compatible_frontend(&executable).unwrap());
        manifest["protocol_major"]=serde_json::json!(1);crate::execution::write(&path,&manifest,false).unwrap();
        std::fs::write(&executable,b"changed binary bytes").unwrap();
        assert!(!super::compatible_frontend(&executable).unwrap());
    }
    use super::*;
    use std::thread;
    use rusqlite::Connection;
    use tempfile::TempDir;
    use crate::{fake,codec::canonical_sha256};

    #[test]
    fn scope_commands_and_scheduler_cannot_admit_workers(){
        let root=TempDir::new().unwrap();let project=root.path().join("scope");let state=project.join(".waywr/state");fs::create_dir_all(&state).unwrap();let key="cccccccccccccccccccccccccccccccc";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":6,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();let service=Service::new(Config{project,state,key:key.into(),release:"scope-test".into(),ipc:root.path().join("project.sock"),herdr_socket:PathBuf::new(),runner:binary.clone(),frontend:binary}).unwrap();
        for action in ["admit_task","authorize_consultation","apply_conversation_steering","answer_task","release_recorded_answer","rebind_run","recover_coordinator","operation.retry_start","cancel_task","steer_queued"]{
            let (_,r)=service.dispatch(&frame("command",action,json!({"command_id":action,"action":action,"payload":{}})),std::process::id()).unwrap();assert_eq!(r["code"],"outside_increment","{action}: {r}");
        }
        let (_,r)=service.dispatch(&frame("task.plan","plan",json!({"task_id":"bad"})),std::process::id()).unwrap();assert_eq!(r["code"],"outside_increment");
        service.session.workflow.connection().unwrap().execute("INSERT INTO session_tasks VALUES('t-forged',1,1,'ready','Forged','Forged',NULL)",[]).unwrap();
        service.tick().unwrap();assert!(service.session.task("t-forged").unwrap().unwrap()["run_id"].is_null());
        assert_eq!(service.session.workflow.connection().unwrap().query_row("SELECT count(*) FROM app_runs",[],|r|r.get::<_,i64>(0)).unwrap(),0);service.writer.shutdown().unwrap();
    }

    #[test]
    fn schema_four_routes_discussion_without_creating_task(){
        let root=TempDir::new().unwrap();
        let project=root.path().join("conversation");
        let state=project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        let key="eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":4,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),release:"schema4-fixture".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:PathBuf::new(),
            runner:binary.clone(),frontend:binary}).unwrap();
        let command=json!({"kind":"command","body":{"command_id":"hello","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"What should we discuss?"}}});
        let (kind,accepted)=service.dispatch(&command,std::process::id()).unwrap();
        assert_eq!(kind,"command_result");assert_eq!(accepted["status"],"accepted");
        assert!(accepted["task_id"].is_null());
        let (kind,snapshot)=service.dispatch(&json!({"kind":"snapshot","body":{}}),std::process::id()).unwrap();
        assert_eq!(kind,"snapshot");
        assert_eq!(snapshot["conversation"]["messages"][0]["text"],"What should we discuss?");
        assert!(snapshot["tasks"].as_array().unwrap().is_empty());
        assert_eq!(service.command(&json!({"command_id":"native","action":"select_coordinator",
            "expected_generation":1,"payload":{"provider":"codex","model":"gpt-6.1-sol","effort":"high"}})).unwrap()["code"],"native_admission_closed");
        service.writer.shutdown().unwrap();
    }

    #[test]
    fn schema_four_mock_discussion_question_consultation_and_proposal_are_separate(){
        let root=TempDir::new().unwrap();
        let project=root.path().join("conversation");
        let state=project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        let key="edededededededededededededededed";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":4,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),release:"schema4-fixture".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:PathBuf::new(),
            runner:binary.clone(),frontend:binary}).unwrap();
        let send=|id:&str,text:&str|{
            let rev=service.session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
            service.command(&json!({"command_id":id,"action":"submit_message",
                "expected_state_revision":rev,"payload":{"text":text,"intent":"discussion"}})).unwrap()
        };
        assert_eq!(send("talk","Hello")["status"],"accepted");
        service.tick_conversation().unwrap();
        assert!(service.session.snapshot().unwrap()["tasks"].as_array().unwrap().is_empty());
        assert_eq!(send("weather","Can we discuss a weather page?")["status"],"accepted");
        service.tick_conversation().unwrap();
        let snap=service.session.conversation_snapshot().unwrap();
        let question=snap["questions"][0]["question_id"].as_str().unwrap();
        let rev=service.session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        assert_eq!(service.command(&json!({"command_id":"answer","action":"answer_conversation",
            "expected_state_revision":rev,"payload":{"question_id":question,"answer":"Celsius only"}})).unwrap()["status"],"accepted");
        service.tick_conversation().unwrap();
        assert_eq!(send("consult","Consult a second agent")["status"],"accepted");
        service.tick_conversation().unwrap();
        let action=service.session.conversation_snapshot().unwrap()["actions"].as_array().unwrap()
            .iter().find(|a|a["kind"]=="request_consultation").unwrap()["action_id"].clone();
        let rev=service.session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        assert_eq!(service.command(&json!({"command_id":"consult-authorization","action":"authorize_consultation",
            "expected_state_revision":rev,"payload":{"action_id":action}})).unwrap()["status"],"accepted");
        service.tick_conversation().unwrap();
        service.tick_conversation().unwrap();
        let snapshot=service.session.snapshot().unwrap();
        assert_eq!(snapshot["tasks"].as_array().unwrap().len(),1);
        assert_eq!(snapshot["tasks"][0]["status"],"proposed");
        let conversation=service.session.conversation_snapshot().unwrap();
        assert_eq!(conversation["budgets"]["consultation"]["charged"],1);
        assert_eq!(conversation["budgets"]["conversation"]["charged"],5);
        service.writer.shutdown().unwrap();
    }

    #[test]
    fn host_task_plan_uses_project_execution_contract_without_herdr() {
        let root=TempDir::new().unwrap();
        let project=root.path().join("project");
        let state=project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        fs::write(project.join(".waywr/workflow.toml"),
            "review_limit = 2\n[roles.author]\nprovider = 'fake'\nmodel = 'deterministic-author'\n[roles.critic]\nprovider = 'fake'\nmodel = 'deterministic-critic'\n[fake]\nseconds = 0.0\n").unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":3,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),release:"isolated-rust".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:PathBuf::new(),
            runner:binary.clone(),frontend:binary}).unwrap();
        let accepted=service.session.submit(&json!({"command_id":"submit","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"Draft a decision log"}})).unwrap();
        let task=service.session.task(accepted["task_id"].as_str().unwrap()).unwrap().unwrap();
        let id=task["task_id"].as_str().unwrap();
        let packet=json!({"operation_kind":"coordinator","task_id":id,"task_version":task["version"],
            "message":task["body"],"message_sha256":crate::codec::sha256(task["body"].as_str().unwrap().as_bytes()),
            "operation_id":format!("c-{id}-v1"),"mode":"success"});
        service.session.coordinator_reserved(&task,&canonical_sha256(&packet).unwrap()).unwrap();
        let result=fake::result(&packet).unwrap();
        service.session.publish_proposal(&task,&result,&canonical_sha256(&result).unwrap()).unwrap();
        let (kind,body)=service.dispatch(&frame("task.plan","plan",json!({"task_id":id,
            "profile":".waywr/workflow.toml"})),std::process::id()).unwrap();
        assert_eq!(kind,"task.plan_result");
        assert_eq!(body["plan"]["execution"]["host"],"systemd-user-v1");
        assert!(body["plan"].get("workspace").is_none());
        assert!(body["plan"].get("herdr_socket").is_none());
        assert_eq!(body["plan_digest"],canonical_sha256(&body["plan"]).unwrap());
        service.writer.shutdown().unwrap();
    }

    #[test]
    fn explicit_retry_reopens_only_exhausted_proven_no_launch_coordinator() {
        let root=TempDir::new().unwrap();
        let project=root.path().join("project");
        let state=project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":3,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),release:"isolated-rust".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:PathBuf::new(),
            runner:binary.clone(),frontend:binary}).unwrap();
        service.publish_owner().unwrap();
        let accepted=service.session.submit(&json!({"command_id":"submit","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"A fake task"}})).unwrap();
        let id=accepted["task_id"].as_str().unwrap();
        let task=service.session.task(id).unwrap().unwrap();
        let directory=service.session.workflow.state.join("session/coordinator").join(id).join("call-v1");
        let packet=json!({"schema_version":1,"operation_kind":"coordinator",
            "operation_id":format!("coordinator-{id}"),"task_id":id,"task_version":1,
            "provider":"fake","role":"coordinator","model":"fake","stage":"coordinator-1",
            "run_id":id,"project_key":key,"message":"A fake task"});
        operation::prepare_host(&service.session.workflow.state,&directory,&packet,key).unwrap();
        service.session.coordinator_reserved(&task,&canonical_sha256(&packet).unwrap()).unwrap();
        for number in 0..3 {
            let attempt=Attempt::create(&service.session.workflow.state,&directory,key,
                &service.config.runner,&service.session.release,service.generation).unwrap();
            write(&attempt.directory.join("prelaunch-refusal.json"),&json!({"schema_version":1,
                "attempt_id":attempt.id,"proof":"isolated fixture: no D-Bus enqueue","number":number}),true).unwrap();
            attempt.retire_proven_no_launch("isolated pre-enqueue refusal").unwrap();
        }
        service.session.stop_coordinator(id,"pre-send-refusal","Three host starts refused before enqueue").unwrap();
        let task=service.session.task(id).unwrap().unwrap();
        let command=json!({"command_id":"retry-once","action":"operation.retry_start",
            "expected_task_version":task["version"],"expected_state_revision":task["state_revision"],
            "payload":{"task_id":id}});
        // Crash fixture: the immutable grant reached disk, but the writer had
        // not yet reopened the task or committed the command receipt.
        let old=Attempt::load(&service.session.workflow.state,&directory).unwrap();
        write(&directory.join("recovery-grants").join(format!("{}.json",sha256(b"retry-once"))),
            &json!({"schema_version":1,"command_id":"retry-once","task_id":id,
                "operation_id":packet["operation_id"],"previous_attempt":old.id,
                "retired_attempts":3,"additional_infrastructure_attempts":3,
                "model_invocations_added":0,"time_unix_ms":1}),true).unwrap();
        let first=service.command(&command).unwrap();
        assert_eq!(first["status"],"accepted");
        assert_eq!(service.command(&command).unwrap(),first);
        assert_eq!(service.session.task(id).unwrap().unwrap()["status"],"queued");
        assert_eq!(service.session.pending_coordinator().unwrap().unwrap()["task_id"],id);
        std::thread::sleep(Duration::from_millis(2100));
        let next=service.ensure_host(&directory).unwrap();
        assert!(next.is_none());
        let fourth=Attempt::load(&service.session.workflow.state,&directory).unwrap();
        assert_ne!(fourth.id,first["grant"]["previous_attempt"]);
        assert!(!fourth.directory.join("start-maybe-sent.json").exists());
        assert!(!directory.join("invocation-reservation.json").exists());
        service.writer.shutdown().unwrap();
    }

    #[test]
    fn explicit_retry_reopens_worker_stage_without_resetting_model_budget() {
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        let state=project.join(".waywr/state");fs::create_dir_all(&state).unwrap();
        let key="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":3,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),release:"isolated-rust".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:PathBuf::new(),
            runner:binary.clone(),frontend:binary}).unwrap();
        service.publish_owner().unwrap();
        let run="worker-retry";
        service.session.workflow.create(&json!({"schema_version":2,"workflow_version":1,
            "run_id":run,"thread_id":run,"brief":"Fake worker","instructions":[],
            "skills":{"draft":"fake","critique":"fake","revise":"fake"},
            "rules_sha256":"test","config_sha256":"test","target":{"host":"systemd-user-v1"},
            "project_key":key,"config":{"roles":{"author":{"provider":"fake","model":"test"},
                "critic":{"provider":"fake","model":"test"}},"fake":{"seconds":0},
                "review_limit":2,"max_invocations":5}})).unwrap();
        let reserved=service.session.workflow.reserve_stage(run).unwrap();
        let directory=PathBuf::from(reserved["directory"].as_str().unwrap());
        service.session.workflow.connection().unwrap().execute(
            "INSERT INTO session_tasks VALUES('t-worker',1,1,'active','worker','worker',?1)",[run]).unwrap();
        for _ in 0..3{
            let attempt=Attempt::create(&service.session.workflow.state,&directory,key,
                &service.config.runner,&service.session.release,service.generation).unwrap();
            write(&attempt.directory.join("prelaunch-refusal.json"),&json!({"schema_version":1,
                "attempt_id":attempt.id,"proof":"isolated fixture: no enqueue"}),true).unwrap();
            attempt.retire_proven_no_launch("isolated no-enqueue proof").unwrap();
        }
        service.session.workflow.stop(run,"blocked","Host starts refused before enqueue").unwrap();
        service.session.transition("t-worker",run,"blocked",Some("Host starts refused")).unwrap();
        let task=service.session.task("t-worker").unwrap().unwrap();
        let command=json!({"command_id":"worker-retry-once","action":"operation.retry_start",
            "expected_task_version":task["version"],"expected_state_revision":task["state_revision"],
            "payload":{"task_id":"t-worker"}});
        let first=service.command(&command).unwrap();
        assert_eq!(first["status"],"accepted");
        assert_eq!(service.command(&command).unwrap(),first);
        assert_eq!(service.session.workflow.snapshot(run).unwrap()["status"],"running");
        assert_eq!(service.session.task("t-worker").unwrap().unwrap()["status"],"active");
        assert!(!directory.join("invocation-reservation.json").exists());
        service.writer.shutdown().unwrap();
    }

    #[test]
    fn host_exit_fence_cannot_close_newer_resume_generation() {
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        let state=project.join(".waywr/state");fs::create_dir_all(&state).unwrap();
        let key="cccccccccccccccccccccccccccccccc";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":3,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),
            release:"isolated-rust".into(),ipc:root.path().join("runtime/project.sock"),
            herdr_socket:PathBuf::new(),runner:binary.clone(),frontend:binary}).unwrap();
        service.publish_owner().unwrap();
        let exit=service.command(&json!({"command_id":"exit","action":"stop_session",
            "expected_state_revision":0,"payload":{}})).unwrap();
        assert_eq!(exit["status"],"accepted");
        let old_id=exit["session"]["exit_id"].as_str().unwrap().to_owned();
        let old_generation=exit["session"]["generation"].as_u64().unwrap();
        assert_eq!(read(&service.session.workflow.state.join("session/dispatch-gate.json")).unwrap()["phase"],"exit-requested");
        let revision=service.session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        let resumed=service.command(&json!({"command_id":"resume","action":"resume_session",
            "expected_state_revision":revision,"payload":{}})).unwrap();
        assert_eq!(resumed["status"],"accepted");
        assert!(service.session.update_exit(&old_id,old_generation,"paused",None).unwrap().is_none());
        let gate=read(&service.session.workflow.state.join("session/dispatch-gate.json")).unwrap();
        assert_eq!(gate["phase"],"running");
        assert_eq!(gate["lifecycle_generation"],resumed["session"]["generation"]);
        service.writer.shutdown().unwrap();
    }
    #[test]
    fn worker_activity_list_is_task_scoped_active_only_and_uses_published_attempt() {
        // Isolated metadata fixture; no scheduler, subprocess or native call.
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        let state=project.join(".waywr/state");fs::create_dir_all(&state).unwrap();
        let key="abababababababababababababababab";
        let contract=state.join("state-contract.json");
        write(&contract,&json!({"schema_version":1,"state_schema":3,
            "execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state:state.clone(),key:key.into(),release:"fixture".into(),
            ipc:root.path().join("project.sock"),herdr_socket:PathBuf::new(),runner:binary.clone(),frontend:binary}).unwrap();
        service.publish_owner().unwrap();
        let accepted=service.session.submit(&json!({"command_id":"submit","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"Metadata fixture"}})).unwrap();
        let id=accepted["task_id"].as_str().unwrap();
        write(&contract,&json!({"schema_version":1,"state_schema":8,
            "execution_host":"systemd-user-v1","project_key":key}),false).unwrap();
        let db=service.session.workflow.connection().unwrap();
        db.execute_batch("CREATE TABLE development_records(kind TEXT,id TEXT,work_id TEXT,record_json TEXT)").unwrap();
        for (name,work,status) in [("one",id,"running"),("old",id,"integrated"),("foreign","other","running")] {
            let unit=json!({"work_id":work,"status":status,"operation_id":format!("worker-{name}"),"attempt_id":null});
            db.execute("INSERT INTO development_records VALUES('unit',?1,?2,?3)",
                rusqlite::params![name,work,unit.to_string()]).unwrap();
            let dir=state.join("session/development").join(id).join(format!("worker-{name}"));
            operation::prepare_host(&state,&dir,&json!({"schema_version":1,"operation_kind":"development-worker",
                "operation_id":format!("worker-{name}"),"task_id":id,"task_version":1,"run_id":id,
                "stage":format!("worker-{name}"),"role":"worker","provider":"fake","model":"fake","message":"fixture"}),key).unwrap();
            if name=="one" {
                let attempt=Attempt::create(&state,&dir,key,&service.config.runner,&service.session.release,service.generation).unwrap();
                fs::write(attempt.directory.join("native-events.jsonl"),format!("{}\n",json!({"type":"item.started",
                    "item":{"id":"cmd","type":"command_execution","command":"cargo test","status":"in_progress",
                        "aggregated_output":"never display output"}}))).unwrap();
            }
        }
        let list=service.operation_list(&json!({"task_id":id,"active_only":true})).unwrap();
        assert_eq!(list["active_only"],true);assert_eq!(list["operations"].as_array().unwrap().len(),1);
        assert_eq!(list["operations"][0]["stage"],"worker-one");
        assert!(list["operations"][0]["attempt_id"].is_string());
        assert_eq!(list["operations"][0]["activity"]["label"],"Running: cargo test");
        assert!(!list.to_string().contains("never display"));
        assert_eq!(service.operation_list(&json!({"task_id":id})).unwrap()["operations"].as_array().unwrap().len(),3);
        assert!(service.operation_list(&json!({"task_id":"other","active_only":true})).is_err());
        service.writer.shutdown().unwrap();
    }

    #[test]
    fn output_pages_are_project_scoped_labelled_and_reader_bounded(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        let state=project.join(".waywr/state");fs::create_dir_all(&state).unwrap();
        let key="dddddddddddddddddddddddddddddddd";
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":3,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Service::new(Config{project,state,key:key.into(),release:"isolated-rust".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:PathBuf::new(),
            runner:binary.clone(),frontend:binary}).unwrap();
        service.publish_owner().unwrap();
        let accepted=service.session.submit(&json!({"command_id":"submit","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"Fake task"}})).unwrap();
        let id=accepted["task_id"].as_str().unwrap();
        let dir=service.session.workflow.state.join("session/coordinator").join(id).join("call-v1");
        operation::prepare_host(&service.session.workflow.state,&dir,
            &json!({"schema_version":1,"operation_kind":"coordinator","operation_id":format!("c-{id}"),
                "task_id":id,"task_version":1,"run_id":id,"stage":"coordinator-1","role":"coordinator",
                "provider":"fake","model":"fake","message":"Fake task"}),key).unwrap();
        assert_eq!(service.operation_list(&json!({"task_id":id})).unwrap()["operations"][0]["status"],
            "prepared; host attempt not yet published");
        let attempt=Attempt::create(&service.session.workflow.state,&dir,key,
            &service.config.runner,&service.session.release,service.generation).unwrap();
        let mut capture=output_capture::Recorder::create(&attempt.directory).unwrap();
        capture.push("stdout",b"draft\n").unwrap();capture.push("stderr",b"warning\n").unwrap();
        capture.flush().unwrap();
        // A completed coordinator call remains inspectable after the task
        // version advances through proposal and later workflow transitions.
        service.session.workflow.connection().unwrap().execute(
            "UPDATE session_tasks SET version=3 WHERE task_id=?1",[id]).unwrap();
        let body=json!({"task_id":id,"stage":"coordinator","limit":5});
        let listing=service.operation_list(&json!({"task_id":id})).unwrap();
        assert_eq!(listing["operations"][0]["stage"],"coordinator");
        let first=service.output_page(&body).unwrap();
        assert_eq!(first["page"]["segments"][0]["stream"],"stdout");
        let mut replay=body.clone();replay["cursor"]=first["page"]["cursor"].clone();
        let second=service.output_page(&replay).unwrap();
        assert_eq!(second["page"]["segments"][0]["stream"],"stdout");
        assert_eq!(second["page"]["segments"][1]["stream"],"stderr");
        replay["task_id"]="another-project-task".into();
        assert!(service.output_page(&replay).is_err());
        let grant=json!({"task_id":id,"stage":"coordinator","attempt_id":attempt.id});
        let denied=service.dispatch_view(&frame("command","unauthorized",
            json!({"action":"cancel_task","task_id":id})),&grant).unwrap();
        assert_eq!(denied.1["code"],"read_only_scope");
        let denied=service.dispatch_view(&frame("operation.output.read","wrong",
            json!({"task_id":id,"stage":"coordinator","attempt_id":"other"})),&grant).unwrap();
        assert_eq!(denied.1["code"],"read_only_scope");
        assert_eq!(service.dispatch_view(&frame("operation.output.read","valid",
            json!({"task_id":id,"stage":"coordinator","attempt_id":attempt.id,"limit":5})),&grant)
            .unwrap().0,"operation.output.page");
        let permits=(0..4).map(|_|service.reader_permit().unwrap()).collect::<Vec<_>>();
        assert!(service.reader_permit().err().unwrap().starts_with("busy:"));
        drop(permits);
        service.writer.shutdown().unwrap();
    }

    fn saved_fake(store:&crate::workflow_store::Store,run:&str){
        let reserved=store.reserve_stage(run).unwrap();
        let directory=PathBuf::from(reserved["directory"].as_str().unwrap());
        let result=fake::result(&reserved["packet"]).unwrap();
        let op=read(&directory.join("operation.json")).unwrap();
        let runner=json!({"pid":0,"start":"synthetic","boot":"synthetic"});
        write(&directory.join("claim.json"),&json!({"runner":runner}),true).unwrap();
        write(&directory.join("result.json"),&result,true).unwrap();
        write(&directory.join("receipt.json"),&json!({"schema_version":1,
            "operation_id":op["operation_id"],"packet_sha256":op["packet_sha256"],
            "launch_token":op["launch_token"],"runner":runner,"status":"completed",
            "result_sha256":canonical_sha256(&result).unwrap()}),true).unwrap();
        store.apply_saved(run,reserved["stage"].as_str().unwrap()).unwrap();
    }

    #[test]
    fn simultaneous_answer_cancel_exit_and_reconnect_have_one_gate_outcome(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";let state=project.join("state");
        let session=Session::open(&project,&state,key,"isolated-rust").unwrap();
        let run="race-run";
        session.workflow.create(&json!({"schema_version":1,"workflow_version":1,"run_id":run,"thread_id":run,
            "brief":"Offline race fixture","instructions":[],"skills":{"draft":"fake","critique":"fake","revise":"fake"},
            "rules_sha256":"test","config_sha256":"test","target":{"socket":"/tmp/test-only.sock","workspace":"w-test"},
            "config":{"roles":{"author":{"provider":"fake","model":"test"},"critic":{"provider":"fake","model":"test"}},
                "fake":{"seconds":0},"review_limit":2,"max_invocations":5}})).unwrap();
        saved_fake(&session.workflow,run);saved_fake(&session.workflow,run);
        let gate=session.workflow.prepare_gate(run).unwrap();
        session.workflow.connection().unwrap().execute("INSERT INTO session_tasks VALUES('t-race',1,1,'awaiting-input','race','race',?1)",[run]).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project,state,key:key.into(),release:"isolated-rust".into(),
            ipc:root.path().join("runtime/project.sock"),herdr_socket:root.path().join("missing-herdr.sock"),
            runner:binary.clone(),frontend:binary}).unwrap());
        let barrier=Arc::new(std::sync::Barrier::new(4));
        let mut jobs=Vec::new();
        for command in [
            json!({"command_id":"answer","action":"answer_task","expected_task_version":1,"expected_state_revision":1,
                "payload":{"task_id":"t-race","request_id":gate["request_id"],"outcome":"answered","answer":"7 days",
                    "artifact_sha256":gate["artifact_sha256"]}}),
            json!({"command_id":"cancel","action":"cancel_task","expected_task_version":1,"expected_state_revision":1,
                "payload":{"task_id":"t-race"}}),
            json!({"command_id":"exit","action":"stop_session","expected_state_revision":0,"payload":{}}),
        ]{
            let service=service.clone();let barrier=barrier.clone();
            jobs.push(thread::spawn(move||{barrier.wait();service.command(&command).unwrap()}));
        }
        barrier.wait();let started=std::time::Instant::now();
        let snapshot=service.session.snapshot().unwrap();
        eprintln!("race_snapshot_observed_ms={}",started.elapsed().as_millis());
        assert!(started.elapsed()<Duration::from_secs(1));
        assert_eq!(snapshot["project_key"],key);
        let results=jobs.into_iter().map(|job|job.join().unwrap()).collect::<Vec<_>>();
        let accepted=results.iter().take(2).filter(|result|result["status"]=="accepted").count();
        assert_eq!(accepted,1,"Answer and cancellation must not both settle the gate: {results:?}");
        assert_eq!(results[2]["status"],"accepted");
        for (id,result) in [("answer",&results[0]),("cancel",&results[1])] {
            let ledger=service.session.command_get(id).unwrap();
            assert_eq!(ledger["status"],if result["status"]=="accepted"{"accepted"}else{"not_found"});
        }
        service.writer.shutdown().unwrap();
    }

    fn connected(socket:&Path,key:&str)->UnixStream{
        let mut peer=loop{match UnixStream::connect(socket){Ok(peer)=>break peer,
            Err(_)=>thread::sleep(Duration::from_millis(10))}};
        peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        send(&mut peer,&frame("hello","h",json!({"protocol":{"major":1,"minor":0},
            "project_key":key,"release_id":"isolated-rust","channel":"command"}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["accepted"],true);
        peer
    }

    #[test]
    fn slow_clients_do_not_exhaust_status_capacity(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";let socket=root.path().join("runtime/project.sock");
        let executable=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project:project.clone(),state:project.join("state"),
            key:key.into(),release:"isolated-rust".into(),ipc:socket.clone(),
            herdr_socket:root.path().join("missing-herdr.sock"),runner:executable.clone(),frontend:executable}).unwrap());
        let running=service.clone();let owner=thread::spawn(move||running.serve());
        let mut slow=Vec::new();
        for _ in 0..32{slow.push(loop{match UnixStream::connect(&socket){Ok(peer)=>break peer,
            Err(_)=>thread::sleep(Duration::from_millis(10))}});}
        let started=std::time::Instant::now();let mut peer=connected(&socket,key);
        send(&mut peer,&frame("snapshot","s",json!({}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["kind"],"snapshot");
        eprintln!("slow_clients_status_observed_max_ms={}",started.elapsed().as_millis());
        assert!(started.elapsed()<Duration::from_secs(1),"status blocked behind slow clients");
        drop(slow);
        send(&mut peer,&frame("command","stop",json!({"command_id":"stop","action":"stop_session",
            "expected_state_revision":0,"payload":{}}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["status"],"accepted");
        drop(peer);owner.join().unwrap().unwrap();
    }

    #[test]
    fn exit_closes_idle_clients_without_waiting_for_tui_detach(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";let socket=root.path().join("runtime/project.sock");
        let binary=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project:project.clone(),state:project.join("state"),
            key:key.into(),release:"isolated-rust".into(),ipc:socket.clone(),
            herdr_socket:root.path().join("missing-herdr.sock"),runner:binary.clone(),frontend:binary}).unwrap());
        let (done_tx,done_rx)=std::sync::mpsc::sync_channel(1);
        let running=service.clone();let owner=thread::spawn(move||{let _=done_tx.send(running.serve());});
        let idle=connected(&socket,key);
        let mut control=connected(&socket,key);
        send(&mut control,&frame("command","stop",json!({"command_id":"exit","action":"stop_session",
            "expected_state_revision":0,"payload":{}}))).unwrap();
        assert_eq!(read_frame(&mut control).unwrap().unwrap()["body"]["status"],"accepted");
        assert!(done_rx.recv_timeout(Duration::from_secs(3)).unwrap().is_ok(),
            "project service waited for an idle attached client");
        drop(idle);drop(control);owner.join().unwrap();
    }

    #[test]
    fn sqlite_lock_returns_busy_before_acceptance_and_same_id_can_replay(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";let state=project.join("state");
        let socket=root.path().join("runtime/project.sock");let executable=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project,state:state.clone(),key:key.into(),
            release:"isolated-rust".into(),ipc:socket.clone(),herdr_socket:root.path().join("missing-herdr.sock"),
            runner:executable.clone(),frontend:executable}).unwrap());
        let running=service.clone();let owner=thread::spawn(move||running.serve());
        let mut peer=connected(&socket,key);
        let blocker=Connection::open(state.join("state.sqlite")).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
        let request=json!({"command_id":"same","action":"submit_message","expected_state_revision":0,
            "payload":{"text":"Make a decision log"}});
        let started=std::time::Instant::now();send(&mut peer,&frame("command","first",request.clone())).unwrap();
        let first=read_frame(&mut peer).unwrap().unwrap();
        assert_eq!(first["body"]["code"],"busy");
        eprintln!("sqlite_busy_observed_max_ms={}",started.elapsed().as_millis());
        assert!(started.elapsed()<Duration::from_millis(500),"SQLite busy exceeded predeclared bound");
        blocker.execute_batch("ROLLBACK").unwrap();
        send(&mut peer,&frame("command_get","lookup",json!({"command_id":"same"}))).unwrap();
        assert_ne!(read_frame(&mut peer).unwrap().unwrap()["body"]["status"],"accepted");
        send(&mut peer,&frame("command","replay",request)).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["status"],"accepted");
        send(&mut peer,&frame("command","stop",json!({"command_id":"stop","action":"stop_session",
            "expected_state_revision":1,"payload":{}}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["status"],"accepted");
        drop(peer);owner.join().unwrap().unwrap();
    }

    #[test]
    fn stalled_herdr_call_does_not_block_status(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";let socket=root.path().join("runtime/project.sock");
        let herdr=root.path().join("stalled-herdr.sock");
        let listener=std::os::unix::net::UnixListener::bind(&herdr).unwrap();
        let stalled=thread::spawn(move||{
            let (first,_)=listener.accept().unwrap();drop(first);
            let (second,_)=listener.accept().unwrap();
            thread::sleep(Duration::from_secs(9));drop(second);
        });
        let executable=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project:project.clone(),state:project.join("state"),
            key:key.into(),release:"isolated-rust".into(),ipc:socket.clone(),herdr_socket:herdr,
            runner:executable.clone(),frontend:executable}).unwrap());
        let running=service.clone();let owner=thread::spawn(move||running.serve());
        let attach_socket=socket.clone();let attaching=thread::spawn(move||{
            let mut peer=connected(&attach_socket,key);
            peer.set_read_timeout(Some(Duration::from_secs(12))).unwrap();
            send(&mut peer,&frame("ui.attach_direct","attach",json!({"pane":null}))).unwrap();
            read_frame(&mut peer).unwrap().unwrap()
        });
        thread::sleep(Duration::from_millis(150));
        let started=std::time::Instant::now();let mut peer=connected(&socket,key);
        send(&mut peer,&frame("snapshot","s",json!({}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["kind"],"snapshot");
        eprintln!("stalled_herdr_status_observed_max_ms={}",started.elapsed().as_millis());
        assert!(started.elapsed()<Duration::from_secs(1),"status blocked behind Herdr I/O");
        assert_eq!(attaching.join().unwrap()["kind"],"error");
        send(&mut peer,&frame("command","stop",json!({"command_id":"stop","action":"stop_session",
            "expected_state_revision":0,"payload":{}}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["status"],"accepted");
        drop(peer);owner.join().unwrap().unwrap();stalled.join().unwrap();
    }

    #[test]
    fn ipc_replays_commands_and_exit_without_a_worker(){
        let root=TempDir::new().unwrap();
        let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let key="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let state=project.join(".waywr/state");
        let socket=root.path().join("runtime/project.sock");
        let executable=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project,state,key:key.into(),release:"isolated-rust".into(),
            ipc:socket.clone(),herdr_socket:root.path().join("missing-herdr.sock"),
            runner:executable.clone(),frontend:executable}).unwrap());
        let running=service.clone();let thread=thread::spawn(move||running.serve());
        let mut peer=loop{match UnixStream::connect(&socket){Ok(peer)=>break peer,Err(_)=>thread::sleep(Duration::from_millis(20))}};
        send(&mut peer,&frame("hello","h",json!({"protocol":{"major":1,"minor":0},
            "project_key":key,"release_id":"isolated-rust","channel":"command"}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["accepted"],true);
        send(&mut peer,&frame("snapshot","s",json!({}))).unwrap();
        let snapshot=read_frame(&mut peer).unwrap().unwrap();assert_eq!(snapshot["body"]["state_revision"],0);
        let command=json!({"command_id":"first","action":"submit_message","expected_state_revision":0,
            "payload":{"text":"Make a decision log"}});
        send(&mut peer,&frame("command","a",command.clone())).unwrap();
        let first=read_frame(&mut peer).unwrap().unwrap();assert_eq!(first["body"]["status"],"accepted");
        send(&mut peer,&frame("command","a-replay",command)).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"],first["body"]);
        send(&mut peer,&frame("events_read","e",json!({"after_seq":0,"limit":64}))).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["events"].as_array().unwrap().len(),1);
        let stop=json!({"command_id":"exit","action":"stop_session","expected_state_revision":1,"payload":{}});
        send(&mut peer,&frame("command","stop",stop)).unwrap();
        assert_eq!(read_frame(&mut peer).unwrap().unwrap()["body"]["status"],"accepted");
        drop(peer);
        thread.join().unwrap().unwrap();
        assert_eq!(service.session.phase().unwrap()["phase"],"paused");
        assert!(service.session.workflow.state.join("session/checkpoints/latest.json").exists());
    }

    #[test]
    fn unexpected_scheduler_fault_stops_after_three_and_keeps_ipc_state_readable(){
        let root=TempDir::new().unwrap();
        let project=root.path().join("project");fs::create_dir_all(&project).unwrap();
        let binary=std::env::current_exe().unwrap();
        let service=Arc::new(Service::new(Config{project:project.clone(),
            state:project.join(".waywr/state"),key:"a".repeat(32),release:"fixture".into(),
            ipc:root.path().join("runtime/session.sock"),herdr_socket:root.path().join("no-herdr.sock"),
            runner:binary.clone(),frontend:binary}).unwrap());
        // This inconsistent fixture forces an internal scheduler error before
        // any operation or external process can be prepared.
        service.session.workflow.connection().unwrap().execute(
            "INSERT INTO session_tasks VALUES('t-broken',1,1,'active','broken','broken','r-missing')",[]).unwrap();
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all()
            .build().unwrap().block_on(service.clone().schedule_async());
        let snapshot=service.session.snapshot().unwrap();
        assert_eq!(snapshot["service_attention"]["status"],"stopped");
        assert_eq!(snapshot["service_attention"]["attempts"],3);
        assert_eq!(snapshot["tasks"][0]["status"],"active");
        assert_eq!(fs::read_dir(service.session.workflow.state.join("session/service-errors"))
            .unwrap().count(),3);
        assert_eq!(snapshot["recent_events"].as_array().unwrap().iter()
            .filter(|event|event["type"]=="session.attention").count(),2);
    }
}

impl Service {
    async fn project_history_async(self:Arc<Self>){
        let mut projected_revision=None;
        while !self.shutting_down.load(Ordering::SeqCst){let service=self.clone();let previous=projected_revision.clone();
            let result=tokio::task::spawn_blocking(move||->Result<Option<u64>,String>{let revision=service.session.current_revision()?;if previous==Some(revision){return Ok(previous)}
                let pending=crate::request_history::project(&service.session)?;Ok(if pending{None}else{Some(revision)})
            }).await;
            match result {Ok(Ok(revision))=>projected_revision=revision,other=>{let path=self.session.workflow.state.join("history/projection-attention.json");let body=json!({"status":"attention","reason":format!("{other:?}"),"action":"Committed source events remain saved; preserve and reconcile the history projection"});let _=tokio::task::spawn_blocking(move||{if read(&path).ok().as_ref()!=Some(&body){let _=write(&path,&body,false);}}).await;}}
            time::sleep(Duration::from_millis(500)).await;
        }
        let service=self.clone();let _=tokio::task::spawn_blocking(move||crate::request_history::project(&service.session)).await;
    }
    async fn schedule_async(self:Arc<Self>){
        let mut failures=0u8;
        while !self.shutting_down.load(Ordering::SeqCst){
            let service=self.clone();
            let result=tokio::task::spawn_blocking(move||service.tick()).await
                .unwrap_or_else(|error|Err(format!("Project scheduler task failed: {error}")));
            match result{
                Ok(())=>{
                    if failures>0{let service=self.clone();let _=tokio::task::spawn_blocking(move||service.session.service_attention(None,0)).await;}
                    failures=0;
                }
                Err(error)=>{
                    failures=failures.saturating_add(1).min(3);
                    let service=self.clone();
                    let _=tokio::task::spawn_blocking(move||{
                        let path=service.session.workflow.state.join("session/service-errors")
                            .join(format!("generation-{}-attempt-{failures}.json",service.generation));
                        let _=write(&path,&json!({"schema_version":1,"at":execution::now(),
                            "service_generation":service.generation,"attempt":failures,"error":error,
                            "operator_action":"Inspect this project's task and operation records; never retry ambiguous execution"}),true);
                        let _=service.session.service_attention(Some(&error),failures);
                    }).await;
                    if failures>=3 && !self.session.development(){return;}
                    time::sleep(Duration::from_millis(250u64 * (1u64 << failures))).await;
                }
            }
            time::sleep(Duration::from_millis(150)).await;
        }
    }
    fn tick(&self)->Result<(),String>{
        if crate::provider_selection::enabled(&self.session){crate::provider_selection::reconcile(&self.session)?;}
        if crate::coordination::enabled(&self.session){crate::coordination::pump(&self.session)?;}
        if self.session.development(){self.tick_development()?;}
        let phase=self.session.phase()?;
        let running=phase["phase"]=="running";
        if self.conversation{
            if self.session.conversation_native(){self.tick_chat(running)?;}else if running{self.tick_conversation()?;}
        }else if running || self.session.pending_coordinator()?.is_some(){self.tick_coordinator(running)?;}
        if crate::provider_selection::enabled(&self.session){crate::provider_selection::advance_switch(&self.session)?;}
        if !self.session.conversation_native(){self.tick_workflow(running)?;}
        if running && !self.session.conversation_native(){self.promote_ready()?;}
        else{self.tick_drain(&phase)?;}
        Ok(())
    }
    fn tick_chat(&self,running:bool)->Result<(),String>{
        if self.session.continuous(){crate::live_input::reconcile(&self.session)?;}
        let active=self.session.chat_reserved()?;
        let pending=if active.is_some(){None}else if running{self.session.conversation_next_queued()?}else{None};
        let Some(id)=active.as_ref().or(pending.as_ref()).and_then(|v|v["turn_id"].as_str())else{return Ok(());};
        let directory=if self.session.development(){let Some(path)=self.session.development_chat_attempt(id)?else{return Ok(())};path}else{self.session.workflow.state.join("session/coordinator").join(id).join("call-v1")};
        if let Some(next)=pending.as_ref(){
            let readiness=self.session.chat_readiness()?;
            if readiness["status"]!="available"{self.session.conversation_block_queued(id,readiness["reason"].as_str().unwrap_or("Coordinator unavailable"))?;return Ok(());}
            let profile=&readiness["profile"];
            if let Err(e)=if self.session.development(){crate::development::preflight(profile)}else{crate::claude_chat::preflight(profile)}{self.session.conversation_block_queued(id,&e)?;return Ok(());}
            let mut fresh_packet=if self.session.scope_enabled(){match self.session.scope_packet(next,profile,&readiness["profile_sha256"]){Ok(v)=>v,Err(e) if e.starts_with("busy:")=>return Ok(()),Err(e)=>{self.session.conversation_block_queued(id,&e)?;return Ok(());}}}else{
            let snapshot=self.session.conversation_snapshot()?;
            let handoff=if self.session.scope_enabled(){json!({"state_revision":snapshot["state_revision"]})}else{self.session.conversation_handoff()?};
            if snapshot["state_revision"]!=handoff["state_revision"]{return Ok(());}
            let through=snapshot["messages"].as_array().ok_or("Missing conversation messages")?.iter().find(|m|m["message_id"]==next["message_id"]).and_then(|m|m["seq"].as_u64()).ok_or("Selected message not in bounded conversation snapshot")?;
            let eligible=snapshot["messages"].as_array().unwrap().iter().filter(|m|m["seq"].as_u64().unwrap_or(u64::MAX)<through).collect::<Vec<_>>();
            let selected=eligible.into_iter().rev().take(6).rev()
                .map(|m|json!({"message_id":m["message_id"],"seq":m["seq"],"role":m["role"],"text":m["text"],"text_sha256":sha256(m["text"].as_str().unwrap_or("").as_bytes())})).collect::<Vec<_>>();
            let context=json!({"messages":selected,"handoff":handoff,"project_key":self.session.key});
            let packet=json!({"schema_version":1,"operation_kind":"conversation","operation_id":id,"turn_id":id,
                "run_id":id,"stage":"conversation","role":"coordinator","provider":"claude","model":profile["model"],
                "project_key":self.session.key,"message":next["text"],"binding":snapshot["binding"],
                "execution":profile,"authorization_profile_sha256":readiness["profile_sha256"],
                "context_sha256":canonical_sha256(&context).map_err(|e|e.to_string())?,"selected_context":context});
                packet
            };
            if !self.session.development()&&fresh_packet["execution"]["mode"]=="native"{
                fresh_packet["execution"]=crate::provider_selection::with_client(fresh_packet["execution"].clone());
            }
            // Scope packet construction includes bounded development context before its final check.
            if fresh_packet["execution"]["profile"]=="development-profiles-v1"{if let Err(error)=crate::development_context::prepare(&self.session,&directory,&mut fresh_packet){self.session.conversation_block_queued(id,&error)?;return Ok(())}}
            let limit=if fresh_packet["execution"]["profile"]=="development-profiles-v1"{crate::development_context::MEMORY_LIMIT}else{crate::native_contract::PACKET_LIMIT};
            if serde_json::to_vec(&fresh_packet).map_err(|e|e.to_string())?.len()>limit{
                self.session.conversation_block_queued(id,"Final coordinator context exceeds application memory bound before freezing; no call reserved")?;return Ok(());
            }
            // Freeze input before reserving authority; a crash can never create a second packet for the same turn.
            let saved=self.session.workflow.state.join("session/conversation").join(id);
            let packet=if crate::provider_selection::enabled(&self.session){
                let hash=canonical_sha256(&fresh_packet).map_err(|e|e.to_string())?;
                write(&saved.join("packets").join(format!("{hash}.json")),&fresh_packet,true)?;fresh_packet
            }else if saved.join("packet.json").exists(){
                let frozen=read(&saved.join("packet.json"))?;
                if frozen["turn_id"]!=id || frozen["project_key"]!=self.session.key || frozen["message"]!=next["text"] || frozen["authorization_profile_sha256"]!=readiness["profile_sha256"]{
                    self.session.conversation_block_queued(id,"Prepared coordinator packet differs from frozen authorization; inspect without replay")?;return Ok(());
                }frozen
            }else{write(&saved.join("packet.json"),&fresh_packet,true)?;fresh_packet};
            let reservation=self.session.conversation_reserve(id,&packet,"conversation")?;
            if reservation["status"]!="accepted"{if reservation["code"]!="busy"{self.session.conversation_block_queued(id,reservation["message"].as_str().unwrap_or("Turn reservation refused; inspect context size and allowance"))?;}return Ok(());}
        }
        let saved=self.session.workflow.state.join("session/conversation").join(id);
        let reserved=self.session.chat_reserved()?.ok_or("Missing reserved chat")?;
        let packet=if crate::provider_selection::enabled(&self.session){read(&saved.join("packets").join(format!("{}.json",reserved["packet_sha256"].as_str().ok_or("Missing packet hash")?)))?}else{read(&saved.join("packet.json"))?};
        let digest=canonical_sha256(&packet).map_err(|e|e.to_string())?;
        if reserved["packet_sha256"]!=digest{return self.session.conversation_stop(id,reserved["packet_sha256"].as_str().unwrap_or(""),"uncertain","packet-changed","Frozen coordinator packet changed; no launch or replay").map(|_|());}
        operation::prepare_host(&self.session.workflow.state,&directory,&packet,&self.session.key).map_err(|e|e.detail)?;
        if let Some(cancel)=self.session.chat_cancel_request(id)?{
            if directory.join("active-host-attempt.json").exists(){let attempt=Attempt::load(&self.session.workflow.state,&directory)?;
                attempt.request_cancel(cancel["command_id"].as_str().ok_or("Missing cancellation command")?)?;
            }else{if self.session.continuous(){write(&directory.join("pre-launch-interruption.json"),&json!({"schema_version":1,"owner_generation":self.generation,"turn_id":id,"command_id":cancel["command_id"],"no_start_request_sent":true}),true)?;return self.session.dialogue_stop(id,&digest,&json!({"schema_version":1,"category":"interrupted","known_completion":true,"settled":true,"capabilities_valid":true,"no_start_request_sent":true}),"Interrupted before hosting; held debit retained").map(|_|());}return self.session.conversation_stop(id,&digest,"cancelled","no-host-attempt","Cancelled before hosting; held allowance remains recorded").map(|_|());}
        }
        let launched=directory.join("active-host-attempt.json").exists();
        if !running && !launched{return Ok(());}
        if launched{let attempt=Attempt::load(&self.session.workflow.state,&directory)?;
            let path=attempt.directory.join("conversation-stream.json");
            if path.is_file(){self.session.chat_progress(id,read(&path)?["text"].as_str().unwrap_or(""))?;}}
        match self.ensure(&directory){
            Ok(Some(done)) if done["status"]=="completed"=>{
                let result=read(&operation::result_path(&directory).map_err(|e|e.detail)?)?;
                match self.session.conversation_apply(id,&digest,&result){
                    Ok(outcome) if outcome["status"]=="accepted"=>{},
                    refused=>{self.session.conversation_stop(id,&digest,"output-invalid",&canonical_sha256(&result).map_err(|e|e.to_string())?,&format!("Saved coordinator result refused; inspect immutable receipt: {refused:?}"))?;}
                }
                self.session.checkpoint()?;
            },
            Ok(Some(done)) if self.session.development()&&done["status"]=="capacity-waiting"=>{self.session.development_chat_wait(id,&directory,&done)?;},
            Ok(Some(done)) if self.session.continuous()=>{let attempt=Attempt::load(&self.session.workflow.state,&directory)?;let proof=read(&attempt.directory.join("completion.json"))?;self.session.dialogue_stop(id,&digest,&proof,done["error"].as_str().unwrap_or("Coordinator interrupted; call counted; held messages need explicit continuation"))?;self.session.checkpoint()?;},
            Ok(Some(done))=>{self.session.conversation_stop(id,&digest,if done["status"]=="cancelled"{"cancelled"}else if done["status"]=="output-invalid"{"output-invalid"}else{"failed"},"validated-receipt",done["error"].as_str().unwrap_or("Coordinator stopped; inspect its retained receipt"))?;self.session.checkpoint()?;},
            Ok(None)=>{},
            Err(e) if self.session.continuous() && e.kind==ErrorKind::PreSendRefusal && self.session.chat_cancel_request(id)?.is_some()=>{
                let attempt=Attempt::load(&self.session.workflow.state,&directory)?;
                if attempt.retire_proven_no_launch("Human interruption with positive pre-launch refusal").is_ok(){
                    self.session.dialogue_stop(id,&digest,&json!({"schema_version":1,"category":"interrupted","known_completion":true,"settled":true,"capabilities_valid":true,"settlement":"positive-nonlaunch-retirement","attempt_id":attempt.id}),"Interrupted before invocation; attempt retired, debit retained")?;
                }else{self.session.conversation_stop(id,&digest,"uncertain","interruption-unresolved",&e.detail)?;}
                self.session.checkpoint()?;
            },
            Err(e) if e.kind==ErrorKind::Operational=>{},
            Err(e)=>{self.session.conversation_stop(id,&digest,if e.kind==ErrorKind::PreSendRefusal{"failed"}else{"uncertain"},"host-outcome",&e.detail)?;self.session.checkpoint()?;}
        }
        Ok(())
    }
    fn tick_conversation(&self)->Result<(),String>{
        let Some(next)=self.session.conversation_next_queued()? else{return Ok(());};
        let turn=next["turn_id"].as_str().ok_or("Queued turn lacks ID")?;
        let text=next["text"].as_str().ok_or("Queued turn lacks text")?;
        let role=next["role"].as_str().ok_or("Queued turn lacks role")?;
        let snapshot=self.session.conversation_snapshot()?;
        let handoff=self.session.conversation_handoff()?;
        if snapshot["state_revision"]!=handoff["state_revision"]{
            return Ok(()); // A concurrent answer or command changed context before freezing the packet.
        }
        let selected=snapshot["messages"].as_array().ok_or("Conversation snapshot lacks messages")?
            .iter().rev().take(6).rev().map(|item|json!({"message_id":item["message_id"],
                "seq":item["seq"],"role":item["role"],"text_sha256":sha256(item["text"].as_str().unwrap_or("").as_bytes()),
                "text":item["text"]})).collect::<Vec<_>>();
        let context=json!({"schema_version":1,"messages":selected,"questions":snapshot["questions"],
            "handoff":handoff,"selected_project_key":self.session.key});
        let packet=json!({"schema_version":1,"mode":"mock","project_key":self.session.key,
            "turn_id":turn,"message_id":next["message_id"],"role":role,"intent":next["intent"],
            "target_task_id":next["task_id"],"input_sha256":sha256(text.as_bytes()),
            "binding":snapshot["binding"],"context_sha256":canonical_sha256(&context).map_err(|e|e.to_string())?,
            "selected_context":context});
        let directory=self.session.workflow.state.join("session/conversation").join(turn);
        write(&directory.join("packet.json"),&packet,true)?;
        let bucket=if role=="consultation_request"{"consultation"}else{"conversation"};
        let reserve=self.session.conversation_reserve(turn,&packet,bucket)?;
        if reserve["status"]!="accepted"{
            match reserve["code"].as_str(){
                Some("busy")=>return Ok(()),
                Some("budget_exhausted"|"stale_binding"|"invalid_packet")=>{
                    self.session.conversation_block_queued(turn,reserve["message"].as_str().unwrap_or("Turn cannot be dispatched"))?;
                    return Ok(());
                }
                _=>return Err(format!("Conversation reservation refused: {reserve}")),
            }
        }
        self.session.conversation_provisional(turn,"Mock coordinator stream: selected project context received; preparing a scripted result")?;
        std::thread::sleep(Duration::from_millis(120));
        let result=mock_coordinator::response(role,next["intent"].as_str().unwrap_or("discussion"),text,
            self.session.conversation_has_answer()?,next["task_id"].as_str());
        write(&directory.join("result.json"),&result,true)?;
        let digest=reserve["packet_sha256"].as_str().ok_or("Reservation lacks packet hash")?;
        let applied=self.session.conversation_apply(turn,digest,&result);
        match applied{
            Ok(outcome) if outcome["status"]=="accepted"=>Ok(()),
            Ok(refused)=>{
                self.session.conversation_stop(turn,digest,"output-invalid",
                    &canonical_sha256(&result).map_err(|e|e.to_string())?,
                    &format!("Mock result rejected by broker: {refused}"))?;Ok(())
            }
            Err(error)=>{
                self.session.conversation_stop(turn,digest,"output-invalid",
                    &canonical_sha256(&result).map_err(|e|e.to_string())?,
                    &format!("Mock result could not be interpreted: {error}"))?;Ok(())
            }
        }
    }
    pub(crate) fn ensure(&self,directory:&Path)->Result<Option<Value>,operation::OpError>{
        let op=read(&directory.join("operation.json")).map_err(operation::OpError::from)?;
        if op["schema_version"]==2{return self.ensure_host(directory);}
        if let Some(done)=operation::receipt(directory)?{return Ok(Some(done));}
        let control=Herdr::new(&self.config.herdr_socket);
        let fence=OwnerFence{state:self.session.workflow.state.clone(),generation:Some(self.generation)};
        let before_launch=||->Result<(),operation::OpError>{
            if self.session.phase().map_err(operation::OpError::from)?["phase"]!="running"{
                return Err("Project exit fenced new launch".into());
            }
            Ok(())
        };
        let status=Ensure{state:&self.session.workflow.state,directory,runner_binary:&self.config.runner,
            control:&control,fence:&fence,launch_gate:Some(&self.dispatch_gate),before_launch:Some(&before_launch)}.run()?;
        if status["status"]=="completed" || status["status"]=="failed" || status["status"]=="cancelled"{
            Ok(Some(status))
        }else{Ok(None)}
    }
    fn ensure_host(&self,directory:&Path)->Result<Option<Value>,operation::OpError>{
        use operation::{OpError,ErrorKind};
        let state=&self.session.workflow.state;
        if let Some(done)=operation::receipt(directory)?{return Ok(Some(done));}
        let phase=self.session.phase().map_err(OpError::from)?;
        let running=phase["phase"]=="running";
        let attempt=if directory.join("active-host-attempt.json").exists(){
            Attempt::load(state,directory).map_err(OpError::from)?
        }else{
            if !running{return Ok(None);}
            Attempt::create(state,directory,&self.session.key,&self.config.runner,
                &self.session.release,self.generation).map_err(|detail|if detail=="Frozen packet hash mismatch"{OpError{kind:ErrorKind::PreSendRefusal,detail:format!("{detail}; no host attempt created. Preserve the packet and Inspect before recovery")}}else{OpError::from(detail)})?
        };
        attempt.reconcile_recovery_link().map_err(OpError::from)?;
        if attempt.directory.join("retired.json").exists(){
            if !running{return Ok(None);}
            let count=fs::read_dir(directory.join("attempts")).map_err(|e|OpError::from(e.to_string()))?
                .filter_map(Result::ok).filter(|item|item.path().join("intent.json").exists()).count();
            let grant_dir=directory.join("recovery-grants");
            let grants=if grant_dir.exists(){fs::read_dir(&grant_dir).map_err(|e|OpError::from(e.to_string()))?
                .filter_map(Result::ok).filter(|item|item.path().extension().is_some_and(|ext|ext=="json"))
                .count()}else{0};
            if !self.session.development() && count>=3*(grants+1){return Err(OpError{kind:ErrorKind::PreSendRefusal,
                detail:"Could not start this operation. No agent call was launched. Infrastructure recovery exhausted. Inspect attempts before an explicit retry.".into()});}
            let retired=read(&attempt.directory.join("retired.json")).map_err(OpError::from)?;
            let delay=if count==1{1.0}else{2.0};
            if execution::now()<retired["time"].as_f64().unwrap_or(f64::MAX)+delay{return Ok(None);}
            let replacement=Attempt::create(state,directory,&self.session.key,&self.config.runner,
                &self.session.release,self.generation).map_err(OpError::from)?;
            replacement.reconcile_recovery_link().map_err(OpError::from)?;
            return Ok(None);
        }
        if attempt.directory.join("prelaunch-refusal.json").exists() ||
            attempt.directory.join("pre-dispatch-refusal.json").exists(){
            if attempt.directory.join("cancel.json").exists(){
                return Ok(Some(json!({"status":"cancelled","attempt_id":attempt.id,
                    "error":"Cancelled before invocation; no model call launched"})));
            }
            attempt.retire_proven_no_launch("verified pre-launch refusal")
                .map_err(OpError::from)?;
            return Ok(None);
        }
        if attempt.directory.join("uncertain.json").exists(){
            return Err(OpError{kind:ErrorKind::Uncertain,
                detail:format!("Host execution uncertain: {}",read(&attempt.directory.join("uncertain.json")).map_err(OpError::from)?)});
        }
        if !attempt.directory.join("start-maybe-sent.json").exists(){
            if !running{return Ok(None);}
            if attempt.intent().map_err(OpError::from)?["owner_generation"]!=self.generation{
                attempt.retire_fenced_unsent_owner(self.generation).map_err(OpError::from)?;
                return Ok(None);
            }
            let attempt_copy=attempt.clone();
            let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build()
                .map_err(|e|OpError::from(e.to_string()))?;
            let started=runtime.block_on(attempt_copy.start());
            if let Err(error)=started{
                if attempt.directory.join("prelaunch-refusal.json").exists(){return Ok(None);}
                // Possibly sent, including a lost D-Bus reply: observe only
                // this exact attempt on a later scheduler tick.
                let _=write(&attempt.directory.join("start-observation-error.json"),
                    &json!({"schema_version":1,"attempt_id":attempt.id,"error":error,
                        "action":"reconcile exact unit; never repeat start"}),true);
            }
            return Ok(None);
        }
        let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build()
            .map_err(|e|OpError::from(e.to_string()))?;
        let observed=runtime.block_on(async {
            let manager=UserManager::connect().await?;
            manager.status(&attempt.unit).await
        });
        let status=match observed{
            Ok(status)=>status,
            Err(error)=>{
                let started=read(&attempt.directory.join("start-maybe-sent.json"))
                    .map_err(OpError::from)?["time"].as_f64().unwrap_or(0.0);
                if execution::now()-started<10.0{return Ok(None);}
                return Err(OpError{kind:ErrorKind::Uncertain,
                    detail:format!("Host start possibly sent but exact unit unresolved: {error}; inspect attempt {}",attempt.id)});
            }
        };
        if !attempt.directory.join("claim.json").exists(){
            let started=read(&attempt.directory.join("start-maybe-sent.json"))
                .map_err(OpError::from)?["time"].as_f64().unwrap_or(0.0);
            if execution::now()-started<10.0{return Ok(None);}
            return Err(OpError{kind:ErrorKind::Uncertain,
                detail:format!("Host unit exists without a verified runner claim: {}",attempt.unit)});
        }
        let claim=read(&attempt.directory.join("claim.json")).map_err(OpError::from)?;
        if !execution::alive(&claim["runner"]){
            // Receipt publication precedes normal runner exit. The runner may
            // have completed while systemd status was being read; absence at
            // the earlier observation must not override this positive evidence.
            if let Some(done)=attempt.validated_receipt().map_err(OpError::from)?{return Ok(Some(done));}
            return Err(OpError{kind:ErrorKind::Uncertain,
                detail:format!("Runner exited without validated receipt; inspect exact unit and child: {}",attempt.unit)});
        }
        if !attempt.directory.join("permit.json").exists(){
            if !running{return Ok(None);}
            if self.session.development(){let packet=read(&directory.join("packet.json")).map_err(OpError::from)?;if packet["development"]==true&&packet["operation_kind"]!="conversation"{
                let work=self.session.development_record("workflow",packet["run_id"].as_str().ok_or_else(||OpError::from("Missing development work ID"))?).map_err(OpError::from)?.ok_or_else(||OpError::from("Development workflow missing"))?;
                let scope=self.session.scope_detail(packet["run_id"].as_str().unwrap()).map_err(OpError::from)?.ok_or_else(||OpError::from("Development scope missing"))?;
                if work["status"]!="running"||scope["authorized"]!=true||scope["base"]!=packet["scope_base"]||work["requirement_generation"]!=packet["requirement_generation"]||!(if crate::provider_selection::enabled(&self.session){crate::provider_selection::packet_authorized(&self.session,&work,&packet).map_err(OpError::from)?}else{work["policy"]==packet["policy"]}){attempt.request_cancel("development-requirements-fenced").map_err(OpError::from)?;return Ok(None);}
            }}
            if self.generation!=attempt.intent().map_err(OpError::from)?["owner_generation"]{
                attempt.endorse(&status,self.generation).map_err(OpError::from)?;
            }
            attempt.permit(&status,self.generation).map_err(OpError::from)?;
        }else if self.generation!=attempt.intent().map_err(OpError::from)?["owner_generation"] &&
            !attempt.directory.join(format!("endorsement-{}.json",self.generation)).exists(){
            attempt.endorse(&status,self.generation).map_err(OpError::from)?;
        }
        Ok(None)
    }
    fn tick_coordinator(&self,running:bool)->Result<(),String>{
        let pending=self.session.pending_coordinator()?;
        let task=if let Some(call)=&pending{self.session.task(call["task_id"].as_str().ok_or("invalid pending task")?)?}
            else if running{self.session.queued()?}else{None};
        let Some(task)=task else{return Ok(());};
        let id=task["task_id"].as_str().ok_or("invalid task ID")?;
        let version=task["version"].as_u64().ok_or("invalid task version")?;
        let original=self.session.workflow.state.join("session/coordinator").join(id).join(format!("call-v{version}"));
        let directory=if original.join("recovery.json").exists(){
            let recovery=read(&original.join("recovery.json"))?;
            if execution::hash_file(&original.join("operation.json"))?!=recovery["original_operation_sha256"] ||
                recovery["packet_sha256"]!=read(&original.join("operation.json"))?["packet_sha256"]{
                return Err("Coordinator recovery evidence changed; no dispatch".into());
            }
            let replacement=recovery["replacement"].as_str().ok_or("missing recovery directory")?;
            if replacement.contains('/') || replacement.contains("..") || !replacement.starts_with("call-v"){
                return Err("Invalid coordinator recovery directory".into());
            }
            let directory=original.parent().ok_or("invalid coordinator path")?.join(replacement);
            operation::prepare(&self.session.workflow.state,&directory,&read(&original.join("packet.json"))?,
                &recovery["target"],Some(&self.session.key)).map_err(|e|e.detail)?;
            directory
        }else{original};
        let launched=if self.host{directory.join("active-host-attempt.json").exists()}
            else{directory.join("launch.json").exists()};
        if !launched && self.session.phase()?["phase"]!="running"{return Ok(());}
        if pending.is_none(){
            let workspace=if self.host{String::new()}else{match self.bound_workspace(){Ok(value)=>value,Err(_)=>return Ok(())}};
            let message=task["body"].as_str().ok_or("invalid task body")?;
            let packet=json!({"schema_version":1,"operation_kind":"coordinator",
                "operation_id":format!("coordinator-{id}"),"run_id":id,"stage":"coordinator-1",
                "role":"coordinator","provider":"fake","model":"deterministic-coordinator-v1",
                "task_id":id,"task_version":version,"project_key":self.session.key,
                "message":message,"message_sha256":sha256(message.as_bytes()),"seconds":1.0});
            if self.host{
                operation::prepare_host(&self.session.workflow.state,&directory,&packet,&self.session.key)
                    .map_err(|e|e.detail)?;
            }else{
                let target=self.worker_target(&workspace)?;
                operation::prepare(&self.session.workflow.state,&directory,&packet,&target,Some(&self.session.key))
                    .map_err(|e|e.detail)?;
            }
            self.session.coordinator_reserved(&task,&canonical_sha256(&packet).map_err(|e|e.to_string())?)?;
        }
        if !running && !launched{return Ok(());}
        match self.ensure(&directory){
            Ok(Some(done)) if done["status"]=="completed"=>{
                let result=read(&operation::result_path(&directory).map_err(|e|e.detail)?)?;
                self.session.publish_proposal(&task,&result,done["result_sha256"].as_str().ok_or("missing result digest")?)?;
                self.session.checkpoint()?;
            }
            Ok(Some(done))=>{
                self.session.stop_coordinator(id,done["status"].as_str().unwrap_or("failed"),
                    "Coordinator worker stopped; inspect its receipt. No automatic retry")?;
                self.session.checkpoint()?;
            }
            Ok(None)=>{},
            Err(e) if e.kind==ErrorKind::Operational=>{
                // Prelaunch workspace loss is an infrastructure pause. The same
                // frozen call is reconciled after a verified UI attachment.
            }
            Err(e) if e.kind==ErrorKind::PreSendRefusal=>{
                self.session.stop_coordinator(id,"pre-send-refusal",&e.detail)?;
                self.session.checkpoint()?;
            }
            Err(e)=>{
                self.session.stop_coordinator(id,if e.kind==ErrorKind::OutputInvalid{"output-invalid"}else{"uncertain"},
                    &format!("Coordinator execution unresolved: {}",e.detail))?;
                self.session.checkpoint()?;
            }
        }
        Ok(())
    }
    fn promote_ready(&self)->Result<(),String>{
        if self.session.phase()?["phase"]!="running"{return Ok(());}
        if self.session.active()?.is_some(){return Ok(());}
        let Some((task,plan))=self.session.ready_plan()? else{return Ok(());};
        let id=task["task_id"].as_str().ok_or("invalid task ID")?;
        let run=plan["run_id"].as_str().ok_or("invalid run ID")?;
        let current=if self.host{String::new()}else{match self.bound_workspace(){Ok(workspace)=>workspace,Err(_)=>return Ok(())}};
        let moved=!self.host && plan["workspace"]!=current;
        if moved{
            let old=plan["workspace"].as_str().ok_or("authorized workspace missing")?;
            match Herdr::new(&self.config.herdr_socket).workspace(old){
                Ok(_)=>return Err("Authorized workspace still exists elsewhere; automatic relocation refused".into()),
                Err(error) if !error.contains("workspace_not_found")=>return Err(format!("Cannot verify workspace loss: {error}")),
                Err(_)=>{},
            }
        }
        let profile=self.session.project.join(plan["profile"].as_str().ok_or("invalid profile")?);
        if sha256(&fs::read(profile).map_err(|e|e.to_string())?)!=plan["profile_sha256"]{
            self.session.block_unbound(id,"Frozen workflow profile changed before dispatch")?;return Ok(());
        }
        if let Some(imports)=plan["cross_imports"].as_array(){
            crate::context_access::GrantReader{state:self.session.workflow.state.clone(),recipient_key:self.session.key.clone()}
                .check_frozen(imports)?;
        }
        if self.conversation && plan["workflow_profile"]==crate::static_files::PROFILE{
            self.session.static_create(id,&plan)?;
            self.session.activate(id,run)?;
            self.session.checkpoint()?;
            return Ok(());
        }
        let envelope=self.session.envelope(&plan)?;
        let intent=self.session.workflow.state.join("session/admission-intents").join(format!("{}.json",id));
        write(&intent,&json!({"schema_version":2,"task_id":id,"run_id":run,"plan_sha256":canonical_sha256(&plan).map_err(|e|e.to_string())?,
            "status":"may-have-created"}),true)?;
        let envelope_path=self.session.workflow.run_dir(run)?.join("inputs/envelope.json");
        if envelope_path.exists(){
            if read(&envelope_path)?!=envelope{self.session.block_unbound(id,"Reserved run differs from authorized plan")?;return Ok(());}
            self.session.workflow.snapshot(run)?;
        }else{
            self.session.workflow.create(&envelope)?;
        }
        if moved{
            let target=self.worker_target(&current)?;
            self.session.workflow.append_routing(run,&target,"verified missing owned workspace before first worker",self.generation)?;
        }
        self.session.activate(id,run)?;
        self.session.checkpoint()?;
        Ok(())
    }
    fn tick_workflow(&self,running:bool)->Result<(),String>{
        let Some(task)=self.session.active()? else{return Ok(());};
        let id=task["task_id"].as_str().ok_or("invalid task ID")?;
        let run=task["run_id"].as_str().ok_or("invalid run ID")?;
        if self.session.static_snapshot(run)?.is_some(){return self.tick_static(&task,running);}
        let snapshot=self.session.workflow.snapshot(run)?;
        let status=snapshot["status"].as_str().ok_or("invalid workflow status")?;
        if ["approved","rejected","failed","cancelled","uncertain","blocked","output-invalid"].contains(&status){
            self.session.transition(id,run,status,None)?;self.session.checkpoint()?;return Ok(());
        }
        if status=="awaiting-input"{
            let after=self.session.workflow.consume_answer(run)?;
            if after["status"]=="awaiting-input"{
                self.session.transition(id,run,"awaiting-input",None)?;return Ok(());
            }
            self.session.transition(id,run,"active",None)?;
            self.session.checkpoint()?;
            return Ok(());
        }
        let state=&snapshot["state"];
        if state["questions"].as_array().is_some_and(|v|!v.is_empty()) || state["next_stage"]=="final"{
            self.session.workflow.prepare_gate(run)?;
            self.session.transition(id,run,"awaiting-input",None)?;
            self.session.checkpoint()?;return Ok(());
        }
        if !running{
            let stage=state["next_stage"].as_str().ok_or("invalid stage")?;
            let dir=self.session.workflow.run_dir(run)?.join("operations").join(stage);
            if !(if self.host{dir.join("active-host-attempt.json").exists()}
                else{dir.join("launch.json").exists()}){
                self.session.transition(id,run,"paused",Some("Session exit fences the next worker stage"))?;
                return Ok(());
            }
        }
        let stage=state["next_stage"].as_str().ok_or("invalid stage")?;
        let existing=self.session.workflow.run_dir(run)?.join("operations").join(stage)
            .join(if self.host{"active-host-attempt.json"}else{"launch.json"}).exists();
        if !existing && self.session.phase()?["phase"]!="running"{return Ok(());}
        let reserved=self.session.workflow.reserve_stage(run)?;
        let stage=reserved["stage"].as_str().ok_or("invalid reserved stage")?;
        let directory=Path::new(reserved["directory"].as_str().ok_or("invalid operation path")?);
        match self.ensure(directory){
            Ok(Some(done)) if done["status"]=="completed"=>{
                let applied=self.session.workflow.apply_saved(run,stage)?;
                let next=applied["status"].as_str().ok_or("invalid workflow status")?;
                if next!="running"{self.session.transition(id,run,next,None)?;}
                else{self.session.transition(id,run,"active",None)?;}
                self.session.checkpoint()?;
            }
            Ok(Some(done))=>{
                let reason=format!("Worker {} stopped; retained receipt requires inspection",done["status"]);
                let status=if done["status"]=="cancelled"{"cancelled"}else if done["status"]=="output-invalid"{"output-invalid"}else{"failed"};
                self.session.workflow.stop(run,status,&reason)?;
                self.session.transition(id,run,status,Some(&reason))?;self.session.checkpoint()?;
            }
            Ok(None)=>{self.session.transition(id,run,"active",None)?;}
            Err(e) if e.kind==ErrorKind::Operational=>{
                self.session.transition(id,run,"recovering",Some(&format!("Worker infrastructure unavailable: {}",e.detail)))?;
            }
            Err(e)=>{
                let status=if e.kind==ErrorKind::OutputInvalid{"output-invalid"}
                    else if e.kind==ErrorKind::PreSendRefusal{"blocked"}else{"uncertain"};
                self.session.workflow.stop(run,status,&e.detail)?;
                self.session.transition(id,run,status,Some(&e.detail))?;
                self.session.checkpoint()?;
            }
        }
        Ok(())
    }
    fn tick_static(&self,task:&Value,running:bool)->Result<(),String>{
        let id=task["task_id"].as_str().ok_or("Invalid static task")?;
        let run=task["run_id"].as_str().ok_or("Invalid static run")?;
        let state=self.session.static_snapshot(run)?.ok_or("Missing static state")?;
        let status=state["status"].as_str().ok_or("Invalid static status")?;
        if status!="running"{
            self.session.transition(id,run,status,None)?;
            self.session.checkpoint()?;
            return Ok(());
        }
        let plan=self.session.admitted_plan(id)?.ok_or("Missing frozen static plan")?;
        let packet=match self.session.static_packet(run,&plan){
            Ok(Some(packet))=>packet,
            Ok(None)=>return Ok(()),
            Err(error)=>{
                self.session.static_stop(run,"output-invalid",&error)?;
                self.session.transition(id,run,"output-invalid",Some(&error))?;
                return Ok(());
            }
        };
        let stage=packet["stage"].as_str().ok_or("Missing static stage")?;
        let directory=self.session.workflow.run_dir(run)?.join("operations").join(stage);
        let existing=directory.join("active-host-attempt.json").exists();
        if !running&&!existing{
            self.session.transition(id,run,"paused",Some("Session exit fences the next mock worker stage"))?;
            return Ok(());
        }
        operation::prepare_host(&self.session.workflow.state,&directory,&packet,&self.session.key)
            .map_err(|e|e.detail)?;
        let reserved=self.session.static_reserve(run,&packet)?;
        if reserved["status"]!="accepted"{
            let reason=format!("Static stage cannot launch: {}. Inspect the retained candidate and budget; no extra call is authorized.",
                reserved["message"].as_str().unwrap_or("frozen stage or budget refused"));
            let status=if reserved["code"]=="packet_conflict"{"output-invalid"}else{"blocked"};
            self.session.static_stop(run,status,&reason)?;
            self.session.transition(id,run,status,Some(&reason))?;
            self.session.checkpoint()?;
            return Ok(());
        }
        let execution=self.ensure(&directory);
        self.session.static_charge_if_started(run,&packet)?;
        match execution{
            Ok(Some(done)) if done["status"]=="completed"=>{
                let result=read(&operation::result_path(&directory).map_err(|e|e.detail)?)?;
                match self.session.static_apply_saved(run,&packet,&done,&result){
                    Ok(next)=>{
                        if next["status"]=="awaiting-input"{self.session.transition(id,run,"awaiting-input",None)?;}
                        else if next["status"]=="blocked"{
                            self.session.transition(id,run,"blocked",next["reason"].as_str())?;
                        }
                        else{self.session.transition(id,run,"active",None)?;}
                    }
                    Err(error)=>{
                        self.session.static_stop(run,"output-invalid",&error)?;
                        self.session.transition(id,run,"output-invalid",Some(&error))?;
                    }
                }
                self.session.checkpoint()?;
            }
            Ok(Some(done))=>{
                let status=if done["status"]=="cancelled"{"cancelled"}else if done["status"]=="output-invalid"{"output-invalid"}else{"failed"};
                let reason=format!("Mock static worker {status}; inspect retained receipt");
                self.session.static_stop(run,status,&reason)?;
                self.session.transition(id,run,status,Some(&reason))?;self.session.checkpoint()?;
            }
            Ok(None)=>{self.session.transition(id,run,"active",None)?;}
            Err(error) if error.kind==ErrorKind::Operational=>{
                self.session.transition(id,run,"recovering",Some(&error.detail))?;
            }
            Err(error)=>{
                let status=if error.kind==ErrorKind::OutputInvalid{"output-invalid"}
                    else if error.kind==ErrorKind::PreSendRefusal{"blocked"}else{"uncertain"};
                self.session.static_stop(run,status,&error.detail)?;
                self.session.transition(id,run,status,Some(&error.detail))?;self.session.checkpoint()?;
            }
        }
        Ok(())
    }
    fn tick_drain(&self,phase:&Value)->Result<(),String>{
        if !["exit-requested","draining"].iter().any(|s|phase["phase"]==*s){return Ok(());}
        let exit=phase["exit_id"].as_str().ok_or("invalid exit ID")?;
        let generation=phase["generation"].as_u64().ok_or("invalid lifecycle generation")?;
        let mut in_flight=Vec::new();
        for folder in [self.session.workflow.state.join("session/coordinator"),self.session.workflow.state.join("runs"),self.session.workflow.state.join("session/development")]{
            if !folder.exists(){continue;}
            for entry in fs::read_dir(folder).map_err(|e|e.to_string())?{
                let entry=entry.map_err(|e|e.to_string())?;
                let operations=if entry.path().join("operations").is_dir(){entry.path().join("operations")}else{entry.path()};
                for item in fs::read_dir(operations).map_err(|e|e.to_string())?{
                    let dir=item.map_err(|e|e.to_string())?.path();
                    if dir.join("operation.json").exists() && read(&dir.join("operation.json"))?["schema_version"]==2{
                        if !dir.join("active-host-attempt.json").exists(){continue;}
                        let attempt=Attempt::load(&self.session.workflow.state,&dir)?;
                        if attempt.validated_receipt()?.is_some() || attempt.directory.join("retired.json").exists() ||
                            attempt.directory.join("prelaunch-refusal.json").exists() ||
                            attempt.directory.join("pre-dispatch-refusal.json").exists(){continue;}
                        if attempt.directory.join("start-maybe-sent.json").exists(){
                            let claim=read(&attempt.directory.join("claim.json")).ok();
                            let known_running=claim.as_ref().is_some_and(|value|execution::alive(&value["runner"]));
                            in_flight.push(json!({"operation":dir,"attempt":attempt.id,"unit":attempt.unit,
                                "report":{"status":if known_running{"running"}else{"uncertain"},
                                "detail":if known_running{"Verified runner still active"}
                                    else{"Possibly sent host start lacks a live verified runner and receipt"}}}));
                        }
                        continue;
                    }
                    if dir.join("launch.json").exists() && !dir.join("receipt.json").exists(){
                        in_flight.push(json!({"operation":dir,"report":operation::inspect(&dir,None)}));
                    }
                }
            }
        }
        let notice=if in_flight.iter().any(|v|v["report"]["status"]=="uncertain"){
            Some(json!({"kind":"execution-uncertain","operations":in_flight,
                "operator_action":"Inspect retained evidence; no retry is authorized"}))
        }else if !in_flight.is_empty() && execution::now()>=phase["deadline"].as_f64().unwrap_or(f64::MAX){
            Some(json!({"kind":"attention-required","reason":"Foreground drain deadline passed; known call still running or stalled",
                "operations":in_flight,"operator_action":"Inspect this project's worker panes and operation evidence"}))
        }else{None};
        if !in_flight.is_empty(){
            self.session.update_exit(exit,generation,"draining",notice.as_ref())?;
            self.session.checkpoint()?;return Ok(());
        }
        let _guard=self.dispatch_gate.lock().map_err(|e|e.to_string())?;
        if self.session.update_exit(exit,generation,"paused",None)?.is_none(){return Ok(());}
        if self.host{self.write_dispatch_phase(&self.session.phase()?)?;}
        self.session.checkpoint()?;
        write(&self.session.workflow.state.join("session/shutdown-intents").join(format!("{generation}-{exit}.json")),
            &json!({"schema_version":2,"exit_id":exit,"generation":generation,"service_generation":self.generation}),true)?;
        self.shutting_down.store(true,Ordering::SeqCst);
        let _=self.shutdown_signal.send(true);
        Ok(())
    }
}
