use serde_json::{Value,json};
use std::{path::Path, sync::atomic::{AtomicUsize,Ordering}};
use tempfile::tempdir;
use waywr_backend::{execution::{read,write},herdr::CreateError,operation::{self,Control,Ensure,ErrorKind,OwnerFence}};

struct FaultControl {create_count:AtomicUsize,create_failure:bool,pre_send:bool}
impl Control for FaultControl {
    fn server_identity(&self)->Result<Value,String>{Ok(json!({"pid":1,"start":"fixed","boot":"test"}))}
    fn workspace(&self,_:&str)->Result<Value,String>{Ok(json!({"workspace_id":"w-test"}))}
    fn create(&self,_:&str,_:&str,_:&[String],_:&Path,_:Option<&str>)->Result<Value,CreateError>{
        self.create_count.fetch_add(1,Ordering::SeqCst);
        if self.pre_send{Err(CreateError::PreSend("busy bootstrap tab".into()))}
        else if self.create_failure{Err(CreateError::MaybeSent("fault after possible send".into()))}
        else{Ok(json!({}))}
    }
    fn find(&self,_:&str,_:&Path)->Result<Vec<Value>,String>{Ok(vec![])}
    fn pane(&self,_:&str)->Result<Value,String>{Err("no pane".into())}
}

#[test]
fn possible_send_is_durable_and_never_recreated() {
    let temp=tempdir().unwrap();let state=temp.path();let directory=state.join("probes/r1");
    operation::prepare(state,&directory,&json!({"schema_version":1,"operation_id":"r1",
        "run_id":"r1","stage":"draft","role":"author","model":"fake-v1"}),
        &json!({"workspace":"w-test","socket":"/tmp/isolated-test.sock"}),None).unwrap();
    let control=FaultControl{create_count:AtomicUsize::new(0),create_failure:true,pre_send:false};
    let fence=OwnerFence{state:state.into(),generation:None};
    let runner=Path::new("/tmp/test-runner");
    let run=||Ensure{state,directory:&directory,runner_binary:runner,control:&control,fence:&fence,
        launch_gate:None,before_launch:None}.run();
    let first=run().unwrap_err();
    assert_eq!(first.kind,ErrorKind::Uncertain);
    assert!(directory.join("launch.json").is_file());
    assert_eq!(read(&directory.join("launch.json")).unwrap()["status"],"may-have-been-sent");
    let second=run().unwrap_err();
    assert_eq!(second.kind,ErrorKind::Uncertain);
    assert_eq!(control.create_count.load(Ordering::SeqCst),1);
    assert!(!directory.join("permit.json").exists());
}

#[test]
fn stale_controller_cannot_create_a_pane() {
    let temp=tempdir().unwrap();let state=temp.path();let directory=state.join("probes/r2");
    operation::prepare(state,&directory,&json!({"schema_version":1,"operation_id":"r2",
        "run_id":"r2","stage":"draft","role":"author","model":"fake-v1"}),
        &json!({"workspace":"w-test","socket":"/tmp/isolated-test.sock"}),None).unwrap();
    write(&state.join("supervision/owner.json"),&json!({"generation":2}),true).unwrap();
    let control=FaultControl{create_count:AtomicUsize::new(0),create_failure:false,pre_send:false};
    let fence=OwnerFence{state:state.into(),generation:Some(1)};
    let error=Ensure{state,directory:&directory,runner_binary:Path::new("/tmp/test-runner"),control:&control,
        fence:&fence,launch_gate:None,before_launch:None}.run().unwrap_err();
    assert_eq!(error.kind,ErrorKind::Operational);
    assert!(error.detail.contains("Stale"));
    assert_eq!(control.create_count.load(Ordering::SeqCst),0);
    assert!(!directory.join("launch.json").exists());
}

#[test]
fn proven_pre_send_refusal_is_retained_and_not_redispatched() {
    let temp=tempdir().unwrap();let state=temp.path();let directory=state.join("probes/pre-send");
    operation::prepare(state,&directory,&json!({"schema_version":1,"operation_id":"pre-send",
        "run_id":"pre-send","stage":"coordinator","role":"coordinator","model":"fake-v1"}),
        &json!({"workspace":"w-test","socket":"/tmp/isolated-test.sock"}),None).unwrap();
    let control=FaultControl{create_count:AtomicUsize::new(0),create_failure:false,pre_send:true};
    let fence=OwnerFence{state:state.into(),generation:None};
    let run=||Ensure{state,directory:&directory,runner_binary:Path::new("/tmp/test-runner"),
        control:&control,fence:&fence,launch_gate:None,before_launch:None}.run();
    assert_eq!(run().unwrap_err().kind,ErrorKind::PreSendRefusal);
    assert_eq!(read(&directory.join("pre-send-refusal.json")).unwrap()["kind"],"pre-send-refusal");
    assert_eq!(operation::inspect(&directory,None)["status"],"blocked");
    assert_eq!(run().unwrap_err().kind,ErrorKind::PreSendRefusal);
    assert_eq!(control.create_count.load(Ordering::SeqCst),1);
    assert!(!directory.join("claim.json").exists());
    assert!(!directory.join("permit.json").exists());
}
