//! Test-only deterministic command trace for the installed-r9 differential.
use serde_json::{Value,json};
use std::{env,path::Path};
use waywr_backend::session_store::Session;

fn run(project:&Path,state:&Path)->Result<Value,String>{
    let store=Session::open(project,state,&"a".repeat(32),"comparison")?;
    let first=json!({"command_id":"first","action":"submit_message","expected_state_revision":0,
        "payload":{"text":"Specify a local project decision log."}});
    let accepted=store.submit(&first)?;
    let replay=store.submit(&first)?;
    let mut conflict=first.clone();conflict["payload"]["text"]="changed".into();
    let conflict=store.submit(&conflict)?;
    let stale=store.submit(&json!({"command_id":"stale","action":"submit_message",
        "expected_state_revision":0,"payload":{"text":"Second task"}}))?;
    let id=accepted["task_id"].as_str().ok_or("missing task ID")?;
    let task=store.task(id)?.ok_or("missing task")?;
    let steer=json!({"command_id":"steer","action":"steer_queued","expected_task_version":1,
        "expected_state_revision":task["state_revision"],"payload":{"task_id":id,"text":"Revised brief"}});
    let steered=store.steer_queued(&steer)?;
    let steer_replay=store.steer_queued(&steer)?;
    let cancel=json!({"command_id":"cancel","action":"cancel_task","expected_task_version":2,
        "expected_state_revision":store.task(id)?.ok_or("missing task")?["state_revision"],
        "payload":{"task_id":id}});
    let cancelled=store.cancel_undispatched(&cancel)?;
    let stop=json!({"command_id":"exit","action":"stop_session",
        "expected_state_revision":store.snapshot()?["state_revision"],"payload":{}});
    let exiting=store.lifecycle_command(&stop)?;
    let stop_replay=store.lifecycle_command(&stop)?;
    let resume=json!({"command_id":"resume","action":"resume_session",
        "expected_state_revision":store.snapshot()?["state_revision"],"payload":{}});
    let resumed=store.lifecycle_command(&resume)?;
    let stale_exit=store.update_exit("exit",1,"paused",None)?;
    Ok(json!({"accepted":accepted,"replay":replay,"conflict":conflict,"stale":stale,
        "steered":steered,"steer_replay":steer_replay,"cancelled":cancelled,
        "exiting":exiting,"stop_replay":stop_replay,"resumed":resumed,
        "stale_exit":stale_exit,"snapshot":store.snapshot()?}))
}
fn main(){
    let args:Vec<_>=env::args().collect();
    if args.len()!=3{eprintln!("usage: session-probe PROJECT STATE");std::process::exit(2);}
    match run(Path::new(&args[1]),Path::new(&args[2])){
        Ok(value)=>println!("{value}"),
        Err(error)=>{eprintln!("session-probe: {error}");std::process::exit(2);}
    }
}
