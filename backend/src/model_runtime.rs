//! Provider-neutral process boundary. Models and protocols belong to the runtime.
use crate::execution::{hash_file, read, write, write_bytes};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Write, path::{Path,PathBuf}, process::{Command,Stdio}};

pub fn entry() -> Result<PathBuf,String> {
    if let Some(path)=std::env::var_os("WAYWR_MODEL_RUNTIME") { return PathBuf::from(path).canonicalize().map_err(|e|e.to_string()); }
    let executable=std::env::current_exe().map_err(|e|e.to_string())?;
    let installed=executable.parent().and_then(Path::parent).ok_or("Application directory missing")?
        .join("resources/model-runtime/bridge.mjs");
    if installed.is_file(){return Ok(installed)}
    // Development discovery is relative to the unbundled executable; never
    // compile a builder's CARGO_MANIFEST_DIR into distributable application data.
    for root in executable.ancestors().skip(1).take(4){
        let path=root.join("model-runtime/dist/bridge.mjs");
        if path.is_file(){return path.canonicalize().map_err(|e|e.to_string())}
    }
    Err("Model runtime missing; build model-runtime first".into())
}
pub fn default_transport() -> Result<Value,String> {
    let config=std::env::var_os("WAYWR_MODELS_CONFIG").map(PathBuf::from).or_else(||{
        let root=std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(||std::env::var_os("HOME").map(|p|PathBuf::from(p).join(".config")))?;
        let path=root.join("waywr/models.json");path.is_file().then_some(path)
    });
    Ok(json!({"entry":"bundled","config":config}))
}
pub fn destination(provider:&str,transport:&Value)->Result<Value,String>{
    if let Some(config)=transport["config"].as_str(){
        let config=read(Path::new(config))?;
        if let Some(p)=config["providers"].as_array().into_iter().flatten().find(|p|p["id"]==provider){
            return Ok(json!({"provider":provider,"api":p["api"],"base_url":p["base_url"],"auth_env":p["auth_env"]}));
        }
    }
    Ok(json!({"provider":provider,"kind":"runtime_builtin"}))
}
pub fn node() -> Result<PathBuf,String>{crate::provider_selection::find_client("node").ok_or("Node >=22.19 is required by the shared model runtime".into())}
fn path<'a>(value:&'a Value,key:&str)->Result<&'a Path,String>{
    let path=Path::new(value[key].as_str().ok_or_else(||format!("Runtime {key} missing"))?);
    if !path.is_absolute(){return Err(format!("Runtime {key} must be absolute"))} Ok(path)
}
pub fn freeze(mut value:Value)->Result<Value,String>{
    let binary=path(&value,"binary")?.canonicalize().map_err(|e|e.to_string())?;
    let entry=if value["transport"]["entry"]=="bundled"{entry()?}else{path(&value["transport"],"entry")?.to_path_buf()}
        .canonicalize().map_err(|e|e.to_string())?;
    let config=value["transport"]["config"].as_str().map(PathBuf::from);
    if config.as_ref().is_some_and(|p|!p.is_absolute()){return Err("Model configuration must be absolute".into())}
    let config_hash=config.as_ref().map(|p|hash_file(p)).transpose()?;
    let route=destination(value["provider"].as_str().ok_or("Provider missing")?,&value["transport"])?;
    if value["transport"]["destination"]!=route{return Err("Provider destination changed or is not saved; save the profile in F3 before new work".into())}
    if config_hash!=config.as_ref().map(|p|hash_file(p)).transpose()?{return Err("Model configuration changed during preparation".into())}
    value["client"]=json!({"provider":value["provider"],"locator":value["binary"],"binary":binary,"sha256":hash_file(&binary)?});
    value["runtime"]=json!({"protocol":1,"entry":entry,"sha256":hash_file(&entry)?,
        "config_sha256":config_hash,"config":config});
    Ok(value)
}
pub fn preflight(value:&Value)->Result<(),String>{
    if let Some(reason)=value["client_error"].as_str(){return Err(reason.into())}
    for (v,key) in [(&value["client"],"binary"),(&value["runtime"],"entry")] {
        if hash_file(path(v,key)?)? != v["sha256"] {return Err("Runtime executable changed after preparation; retain this attempt and prepare new work".into())}
    }
    if value["runtime"]["protocol"]!=1{return Err("Missing shared model-runtime contract; reconfigure the old native profile".into())}
    if let Some(config)=value["runtime"]["config"].as_str(){
        if hash_file(Path::new(config))?!=value["runtime"]["config_sha256"]{return Err("Model configuration changed after preparation".into())}
    }
    Ok(())
}
pub fn describe(binary:&Path,transport:&Value)->Result<Value,String>{
    let entry=if transport["entry"]=="bundled"{entry()?}else{path(transport,"entry")?.to_path_buf()};
    let mut child=Command::new("timeout").arg("8s").arg(binary).arg(entry).arg("describe")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e|e.to_string())?;
    let input=serde_json::to_vec(&json!({"config":transport["config"]})).map_err(|e|e.to_string())?;
    child.stdin.take().ok_or("Runtime stdin missing")?.write_all(&input).map_err(|e|e.to_string())?;
    let output=child.wait_with_output().map_err(|e|e.to_string())?;
    if !output.status.success(){return Err(format!("Model catalog unavailable: {}",String::from_utf8_lossy(&output.stderr).chars().take(500).collect::<String>()))}
    if output.stdout.len()>1024*1024{return Err("Runtime catalog exceeds 1 MiB".into())}
    let v:Value=serde_json::from_slice(&output.stdout).map_err(|e|e.to_string())?;
    if v["protocol"]!=1||!v["models"].is_array(){return Err("Unsupported runtime catalog".into())} Ok(v)
}
pub fn initial_preferences()->Result<Value,String>{
    let binary=node()?;let mut transport=default_transport()?;
    let catalog=describe(&binary,&transport)?;
    let selected=&catalog["default"];
    transport["destination"]=destination(selected["provider"].as_str().ok_or("Catalog has no default provider")?,&transport)?;
    let profile=json!({"provider":selected["provider"],"model":selected["model"],"binary":binary,
        "effort":"provider_default","options":{},"transport":transport,"context_bytes":98304});
    Ok(json!({"schema_version":3,"profile":"development-profiles-v1","profiles":{"default":profile},
        "coordinator":"default","worker_default":"default","roles":{
            "coordinator":{"default":"default","allowed":["default"]},"implementation":{"default":"default","allowed":["default"]},
            "review":{"default":"default","allowed":["default"]}},"fallback":{"coordinator":[],"implementation":[],"review":[]},
        "concurrency":2,"dependency_network":true,"main_policy":"checked-fast-forward","quality_preference":""}))
}
pub fn prepare(directory:&Path,packet:&Value,instruction:&str)->Result<(Vec<String>,BTreeMap<String,String>),String>{
    let p=&packet["execution"];preflight(p)?;
    if packet["operation_kind"]=="development-worker"{crate::development::worker_write_roots(packet)?;}
    if packet["context_delivery"]["version"]==1{crate::development_context::diagnose(packet,directory)?;}
    let snapshot=if let Some(config)=p["runtime"]["config"].as_str(){
        let bytes=std::fs::read(config).map_err(|e|e.to_string())?;
        if crate::codec::sha256(&bytes)!=p["runtime"]["config_sha256"]{return Err("Model configuration changed before launch".into())}
        let path=directory.join("model-config.json");write_bytes(&path,&bytes,true)?;Some(path)
    }else{None};
    let request=json!({"protocol":1,"packet":packet,"config":snapshot,"instructions":instruction,
        "schema":crate::development::result_schema(packet),"directory":directory});
    write_bytes(&directory.join("prompt.txt"),&serde_json::to_vec(&request).map_err(|e|e.to_string())?,true)?;
    let argv=vec![p["client"]["binary"].as_str().ok_or("Runtime binary missing")?.to_owned(),
        p["runtime"]["entry"].as_str().ok_or("Runtime entry missing")?.to_owned(),"run".into()];
    // Authentication stays in the runtime process. Values never enter packets/evidence.
    let environment=std::env::vars().filter(|(k,_)|crate::adapters::ENV_KEYS.contains(&k.as_str())
        ||k.starts_with("XDG_")||k.ends_with("_API_KEY")||k.ends_with("_AUTH_TOKEN")||k=="GOOGLE_APPLICATION_CREDENTIALS")
        .collect();
    write(&directory.join("execution.json"),&json!({"protocol":1,"argv":argv,"provider":p["provider"],"model":p["model"],"options":p["options"],"runtime":p["runtime"],"boundary":"trusted same-UID runtime; worktrees are not OS isolation"}),true)?;
    Ok((argv,environment))
}
pub fn inspect(v:&Value)->Result<(),String>{
    if !matches!(v["type"].as_str(),Some("runtime.started"|"runtime.text"|"runtime.activity"|"runtime.usage"|"runtime.completed")){
        return Err("Unexpected shared-runtime event".into())
    } Ok(())
}
pub fn finish(events:&[Value],packet:&Value,code:Option<i32>)->Result<(Value,Value),String>{
    let finals=events.iter().filter(|e|e["type"]=="runtime.completed").collect::<Vec<_>>();
    if code!=Some(0)||finals.len()!=1||finals[0]["outcome"]!="completed"{return Err(finals.first().and_then(|v|v["reason"].as_str()).unwrap_or("Model runtime did not return one completed result").into())}
    let final_=finals[0];
    if final_["provider"]!=packet["provider"]||final_["model"]!=packet["model"]{return Err("Model runtime changed the selected destination".into())}
    crate::output::validate(&final_["result"],packet)?;
    Ok((final_["result"].clone(),final_["runtime"].clone()))
}
pub fn progress(directory:&Path,event:&Value)->Result<(),String>{
    if matches!(event["type"].as_str(),Some("runtime.activity"|"runtime.usage")){
        let file=directory.join("runtime-progress.json");
        let mut value=read(&file).unwrap_or(json!({}));
        if event["type"]=="runtime.activity"{value["activity"]=event.clone()}else{value["usage"]=event["usage"].clone()}
        write(&file,&value,false)?;
    } Ok(())
}
