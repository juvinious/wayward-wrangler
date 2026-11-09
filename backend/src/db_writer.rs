//! One bounded writer actor per live project service. Read-only snapshots use
//! independent WAL connections; accepted mutations execute on this thread.
use crate::{execution::{now,write},session_store::Session};
use serde_json::json;
use std::{panic::{AssertUnwindSafe,catch_unwind},path::PathBuf,
    sync::{Arc,Mutex,mpsc::{self,SyncSender,Receiver,TrySendError}},thread::{self,JoinHandle},
    time::{Duration,Instant}};

const CAPACITY:usize=64;
type Job=Box<dyn FnOnce(&Session)+Send+'static>;

pub struct DbWriter {
    state:PathBuf,
    sender:Mutex<Option<SyncSender<Job>>>,
    handle:Mutex<Option<JoinHandle<()>>>,
}
impl DbWriter {
    pub fn start(project:PathBuf,state:PathBuf,key:String,release:String)->Result<Arc<Self>,String>{
        let (sender,receiver):(SyncSender<Job>,Receiver<Job>)=mpsc::sync_channel(CAPACITY);
        let (ready_tx,ready_rx)=mpsc::sync_channel(1);
        let writer_state=state.clone();
        let handle=thread::Builder::new().name(format!("waywr-sqlite-{}",&key[..key.len().min(8)]))
            .spawn(move||{
                let session=Session::open(&project,&state,&key,&release);
                let ready=session.as_ref().map(|_|()).map_err(Clone::clone);
                let _=ready_tx.send(ready);
                let Ok(session)=session else{return;};
                for job in receiver {
                    if catch_unwind(AssertUnwindSafe(||job(&session))).is_err(){
                        let path=state.join("session/writer-failure.json");
                        let _=write(&path,&json!({"schema_version":1,"at":now(),
                            "reason":"SQLite writer panicked; command outcome requires lookup",
                            "operator_action":"Inspect the command ledger and project service errors; do not retry with a new command ID"}),false);
                        break;
                    }
                }
            }).map_err(|e|e.to_string())?;
        ready_rx.recv().map_err(|_|"SQLite writer initialization failed".to_owned())??;
        Ok(Arc::new(Self{state:writer_state,sender:Mutex::new(Some(sender)),handle:Mutex::new(Some(handle))}))
    }
    pub fn call<T:Send+'static>(&self,action:impl FnOnce(&Session)->Result<T,String>+Send+'static)->Result<T,String>{
        let (reply_tx,reply_rx)=mpsc::sync_channel(1);
        let job:Job=Box::new(move|session|{let _=reply_tx.send(action(session));});
        let guard=self.sender.lock().map_err(|e|e.to_string())?;
        let sender=guard.as_ref().ok_or("writer_unavailable: writer is stopping")?;
        match sender.try_send(job){
            Ok(())=>{},
            Err(TrySendError::Full(_))=>return Err("busy: SQLite writer queue full before acceptance".into()),
            Err(TrySendError::Disconnected(_))=>return Err("writer_unavailable: SQLite writer stopped; reconcile command ID".into()),
        }
        drop(guard);
        reply_rx.recv().map_err(|_|"writer_unavailable: SQLite writer stopped; reconcile command ID".to_owned())?
    }
    pub fn shutdown(&self)->Result<(),String>{
        if let Ok(mut sender)=self.sender.lock(){sender.take();}
        let mut guard=self.handle.lock().map_err(|e|e.to_string())?;
        let deadline=Instant::now()+Duration::from_secs(5);
        while guard.as_ref().is_some_and(|handle|!handle.is_finished()) && Instant::now()<deadline{
            thread::sleep(Duration::from_millis(50));
        }
        if guard.as_ref().is_some_and(|handle|!handle.is_finished()){
            let notice=self.state.join("session/writer-shutdown-attention.json");
            write(&notice,&json!({"schema_version":1,"at":now(),
                "reason":"SQLite writer did not stop within five seconds; command outcomes require lookup",
                "operator_action":"Inspect command IDs and SQLite state before restarting this project service"}),false)?;
            guard.take();
            return Err("SQLite writer shutdown timed out; durable attention recorded".into());
        }
        if let Some(handle)=guard.take(){handle.join().map_err(|_|"SQLite writer panicked during shutdown".to_owned())?;}
        Ok(())
    }
}
impl Drop for DbWriter {
    fn drop(&mut self){
        if let Ok(sender)=self.sender.get_mut(){sender.take();}
        if let Ok(handle)=self.handle.get_mut() && let Some(handle)=handle.take(){
            if handle.is_finished(){let _=handle.join();}
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use tempfile::TempDir;
    use std::sync::Barrier;
    #[test]
    fn full_writer_queue_refuses_before_acceptance_and_recovers(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let writer=DbWriter::start(project.clone(),project.join("state"),"a".repeat(32),"test".into()).unwrap();
        let (entered_tx,entered_rx)=mpsc::sync_channel(1);
        let (release_tx,release_rx)=mpsc::sync_channel(1);
        let first=writer.clone();
        let held=thread::spawn(move||first.call(move|_|{
            entered_tx.send(()).unwrap();release_rx.recv().unwrap();Ok(())
        }));
        entered_rx.recv().unwrap();
        let gate=Arc::new(Barrier::new(81));
        let mut jobs=Vec::new();
        for _ in 0..80 {
            let writer=writer.clone();let gate=gate.clone();
            jobs.push(thread::spawn(move||{gate.wait();writer.call(|_|Ok(()))}));
        }
        gate.wait();thread::sleep(std::time::Duration::from_millis(100));
        release_tx.send(()).unwrap();held.join().unwrap().unwrap();
        let results=jobs.into_iter().map(|job|job.join().unwrap()).collect::<Vec<_>>();
        assert!(results.iter().any(|result|result.as_ref().is_err_and(|e|e.starts_with("busy:"))));
        assert!(results.iter().all(|result|result.is_ok() || result.as_ref().is_err_and(|e|e.starts_with("busy:"))));
        writer.call(|_|Ok(())).unwrap();writer.shutdown().unwrap();
    }
    #[test]
    fn stalled_writer_shutdown_records_attention_instead_of_hanging(){
        let root=TempDir::new().unwrap();let project=root.path().join("project");
        std::fs::create_dir_all(&project).unwrap();let state=project.join("state");
        let writer=DbWriter::start(project,state.clone(),"b".repeat(32),"test".into()).unwrap();
        let (entered_tx,entered_rx)=mpsc::sync_channel(1);
        let (release_tx,release_rx)=mpsc::sync_channel(1);
        let active=writer.clone();
        let running=thread::spawn(move||active.call(move|_|{
            entered_tx.send(()).unwrap();release_rx.recv().unwrap();Ok(())
        }));
        entered_rx.recv().unwrap();
        let started=Instant::now();
        assert!(writer.shutdown().unwrap_err().contains("timed out"));
        assert!(started.elapsed()<Duration::from_secs(6));
        assert!(state.join("session/writer-shutdown-attention.json").is_file());
        release_tx.send(()).unwrap();running.join().unwrap().unwrap();
    }
}
