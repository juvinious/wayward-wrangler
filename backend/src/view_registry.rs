//! Display-only grants for explicitly requested Herdr output mirrors.
//! A grant has no execution authority and is never put in a model packet.
use crate::execution::{FileLock,random_token,read,write};
use serde_json::{Value,json};
use std::{fs,path::{Path,PathBuf}};

fn valid_id(value:&str)->bool{value.len()==32 && value.bytes().all(|b|b.is_ascii_hexdigit() && !b.is_ascii_uppercase())}
pub fn intent(state:&Path,id:&str)->Result<Value,String>{
    if !valid_id(id){return Err("Invalid view identity".into());}
    read(&state.join("views/intents").join(format!("{id}.json")))
}
pub fn prepare(state:&Path,project_key:&str,task:&str,stage:&str,attempt:&str,
    server:&Value,socket:&str,frontend:&Path,reopen_after:Option<&str>)->Result<Value,String>{
    let base=state.join("views");
    let _lock=FileLock::exclusive(&base.join("registry.lock"))?;
    let target=crate::codec::sha256(format!("{task}\0{stage}\0{attempt}\0{socket}\0{server}").as_bytes());
    let pointer=base.join("targets").join(format!("{target}.json"));
    if let Some(old)=reopen_after{
        let current=read(&pointer)?;
        if current["view_id"]!=old{return Err("View changed before explicit reopen".into());}
        if !base.join("bindings").join(format!("{old}.json")).is_file(){
            return Err("Unresolved view creation cannot be reopened automatically".into());
        }
    }else if pointer.exists(){
        let current=read(&pointer)?;
        return intent(state,current["view_id"].as_str().ok_or("View pointer missing identity")?);
    }
    let id=random_token()?;let token=random_token()?;
    let record=json!({"schema_version":1,"view_id":id,"token":token,
        "project_key":project_key,"task_id":task,"stage":stage,"attempt_id":attempt,
        "server":server,"socket":socket,"frontend":frontend,"purpose":"read-only operation output"});
    write(&base.join("intents").join(format!("{id}.json")),&record,true)?;
    write(&pointer,&json!({"schema_version":1,"view_id":id}),false)?;
    Ok(record)
}
pub fn mark_sent(state:&Path,id:&str)->Result<Value,String>{
    let grant=intent(state,id)?;
    write(&state.join("views/sent").join(format!("{id}.json")),
        &json!({"schema_version":1,"view_id":id,"status":"may-have-been-sent"}),true)?;
    Ok(grant)
}
pub fn was_sent(state:&Path,id:&str)->bool{state.join("views/sent").join(format!("{id}.json")).is_file()}
pub fn binding(state:&Path,id:&str)->Option<Value>{read(&state.join("views/bindings").join(format!("{id}.json"))).ok()}
pub fn bind(state:&Path,id:&str,binding:&Value)->Result<(),String>{
    let _=intent(state,id)?;
    if binding["view_id"]!=id || binding["pane_id"].as_str().is_none() ||
        binding["tab_id"].as_str().is_none() || binding["workspace_id"].as_str().is_none(){
        return Err("Invalid verified view binding".into());
    }
    write(&state.join("views/bindings").join(format!("{id}.json")),binding,true)
}
pub fn authorize(state:&Path,key:&str,id:&str,token:&str)->Result<Value,String>{
    let grant=intent(state,id)?;
    if grant["project_key"]!=key || grant["token"]!=token ||
        !was_sent(state,id){return Err("Read-only view grant refused".into());}
    Ok(grant)
}
pub fn workspace_record(state:&Path,server:&Value)->Result<PathBuf,String>{
    let digest=crate::codec::sha256(serde_json::to_string(server).map_err(|e|e.to_string())?.as_bytes());
    Ok(state.join("views/workspaces").join(format!("{digest}.json")))
}
pub fn record_workspace_intent(path:&Path,label:&str,server:&Value)->Result<(),String>{
    fs::create_dir_all(path.parent().ok_or("View workspace path has no parent")?).map_err(|e|e.to_string())?;
    write(&path.with_extension("intent.json"),&json!({"schema_version":1,"label":label,"server":server}),true)
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn view_grant_is_read_only_and_scoped(){
        let root=tempfile::tempdir().unwrap();let state=root.path();
        let server=json!({"pid":9,"starttime":1});
        let a=prepare(state,"project-a","task-a","draft","attempt-a",&server,
            "/tmp/private.sock",Path::new("/tmp/waywr"),None).unwrap();
        let id=a["view_id"].as_str().unwrap();let token=a["token"].as_str().unwrap();
        assert_eq!(prepare(state,"project-a","task-a","draft","attempt-a",&server,
            "/tmp/private.sock",Path::new("/tmp/waywr"),None).unwrap(),a);
        assert!(authorize(state,"project-a",id,token).is_err());
        mark_sent(state,id).unwrap();
        assert!(authorize(state,"project-b",id,token).is_err());
        assert!(authorize(state,"project-a",id,"wrong").is_err());
        assert_eq!(authorize(state,"project-a",id,token).unwrap()["stage"],"draft");
        assert!(prepare(state,"project-a","task-a","draft","attempt-a",&server,
            "/tmp/private.sock",Path::new("/tmp/waywr"),Some(id)).is_err());
        bind(state,id,&json!({"view_id":id,"pane_id":"p","tab_id":"t","workspace_id":"w"})).unwrap();
        let next=prepare(state,"project-a","task-a","draft","attempt-a",&server,
            "/tmp/private.sock",Path::new("/tmp/waywr"),Some(id)).unwrap();
        assert_ne!(next["view_id"],a["view_id"]);
    }
}
