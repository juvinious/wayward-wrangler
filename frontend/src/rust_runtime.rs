//! Rust-release startup and project routing. The service owns workflow state;
//! this launcher owns only project setup, unit bootstrap and UI attachment.
use super::{phase,request,route_bound_release,selected_project,session,session_resume,run_ui};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{env,fs,io::{self,Write},os::unix::ffi::OsStrExt,path::{Component,Path,PathBuf},
    process::{Command,Stdio},thread,time::{Duration,Instant}};
use waywr_backend::execution::{FileLock,random_token,read,write,write_bytes};

#[derive(Clone)]
struct Ready {project:PathBuf,state:PathBuf,key:String,release:String,ipc:PathBuf,
    herdr:PathBuf,unit:String}
fn err(e:impl std::fmt::Display)->io::Error{io::Error::other(e.to_string())}
fn value(args:&[String],flag:&str)->Option<String>{args.windows(2).find(|v|v[0]==flag).map(|v|v[1].clone())}
fn selected(input:Option<String>,project:&Path,default:&Path)->io::Result<PathBuf>{
    let path=input.map(PathBuf::from).unwrap_or_else(||default.to_path_buf());
    let path=if path.is_absolute(){path}else{project.join(path)};
    if path.exists(){path.canonicalize()}else{
        let parent=path.parent().ok_or_else(||err("Path has no parent"))?;
        fs::create_dir_all(parent)?;Ok(parent.canonicalize()?.join(path.file_name().ok_or_else(||err("Path has no name"))?))
    }
}
fn init_project(project:&Path)->io::Result<()> {
    waywr_backend::project_layout::check(project).map_err(err)?;
    let root=project.join(".waywr");fs::create_dir_all(&root)?;
    let key=random_token().map_err(err)?;
    write_bytes(&root.join("project.toml"),format!("schema_version = 2\nproject_key = \"{key}\"\n[execution]\nhost = \"systemd-user-v1\"\n[conversation]\nschema_version = 6\n").as_bytes(),true).map_err(err)?;
    let auth_home=std::env::var("HOME").unwrap_or_default();
    let binary=waywr_backend::development::find_codex().unwrap_or_else(||PathBuf::from(&auth_home).join(".local/bin/codex"));
    let legacy=json!({"schema_version":1,"profile":"codex-development-v1","mode":"native",
        "provider":"codex","model":"gpt-6.1-sol","effort":"medium","binary":binary,
        "concurrency":2,"fallback":[],"main_policy":"checked-fast-forward","dependency_network":true});
    let preferences=waywr_backend::provider_selection::imported(&legacy).map_err(err)?;
    write(&root.join("coordinator.json"),&preferences,true).map_err(err)?;

    waywr_backend::project_workflow::initialize(project).map_err(err)?;
    Ok(())
}
fn registry(args:&[String])->io::Result<PathBuf>{
    let chosen=value(args,"--registry-dir").map(PathBuf::from).unwrap_or_else(||{
        env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(||
            PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"))
            .join("wayward-wrangler")
    });
    if !chosen.is_absolute(){return Err(err("Registry directory must be absolute"));}
    Ok(chosen)
}
pub(super) fn relocate_registration(project:&Path,old:&Path,new:&Path,key:&str,args:&[String])->io::Result<()> {
    let root=registry(args)?;if !root.exists(){return Ok(())}
    let _lock=FileLock::exclusive(&root.join("registry.lock")).map_err(err)?;
    let path=root.join("locations.json");if !path.exists(){return Ok(())}
    let mut document=read(&path).map_err(err)?;
    if document["schema_version"]!=1||!document["projects"].is_object(){return Err(err("Unsupported location registry; migration retained for reconciliation"))}
    let expected=json!({"project":project,"state":old});let destination=json!({"project":project,"state":new});
    if document["projects"].as_object().unwrap().iter().any(|(other,v)|other!=key&&v["state"]==destination["state"]){return Err(err("Canonical state is registered to another project"))}
    if let Some(current)=document["projects"].get(key){
        if current!=&expected&&current!=&destination{return Err(err("Registered project location differs; preserve and reconcile"))}
        document["projects"][key]=destination;write(&path,&document,false).map_err(err)?;
    }
    Ok(())
}
fn retired_owner_is_gone(key:&str,args:&[String])->io::Result<()> {
    let unit=format!("waywr-{key}.service");
    let loaded=Command::new("systemctl").args(["--user","show","--property=LoadState","--value",&unit]).output()?;
    if !loaded.status.success(){return Err(err(format!(
        "Cannot verify whether previous project service {unit} is gone; registration preserved")));}
    if String::from_utf8_lossy(&loaded.stdout).trim()!="not-found"{
        return Err(err(format!("Previous project service {unit} still exists; registration preserved")));
    }
    let runtime=value(args,"--ipc-dir").map(PathBuf::from).or_else(||env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
        .unwrap_or_else(||PathBuf::from(format!("/run/user/{}",unsafe{libc::geteuid()})));
    let socket=runtime.join("waywr").join(format!("{key}.sock"));
    match fs::symlink_metadata(&socket){
        Ok(_)=>Err(err(format!("Previous project socket {} still exists; registration preserved",socket.display()))),
        Err(error) if error.kind()==io::ErrorKind::NotFound=>Ok(()),
        Err(error)=>Err(error),
    }
}
fn register_project(project:&Path,state:&Path,key:&str,args:&[String])->io::Result<()> {
    register_project_checked(project,state,key,args,|old|retired_owner_is_gone(old,args))
}
fn replaceable_state(state:&Path,key:&str)->io::Result<bool>{
    let metadata=match fs::symlink_metadata(state){
        Ok(metadata)=>metadata,
        Err(error) if error.kind()==io::ErrorKind::NotFound=>return Ok(true),
        Err(error)=>return Err(error),
    };
    if !metadata.file_type().is_dir(){return Ok(false);}
    let mut entries=fs::read_dir(state)?;
    let Some(entry)=entries.next().transpose()? else{return Ok(false)};
    if entry.file_name()!="project-identity.json" || entries.next().transpose()?.is_some() ||
        !fs::symlink_metadata(entry.path())?.file_type().is_file(){return Ok(false);}
    let marker=read(&entry.path()).map_err(err)?;
    Ok(marker==json!({"schema_version":1,"project_key":key}))
}
fn register_project_checked<F>(project:&Path,state:&Path,key:&str,args:&[String],owner_gone:F)->io::Result<()>
where F:Fn(&str)->io::Result<()> {
    let root=registry(args)?;fs::create_dir_all(&root)?;
    let deadline=Instant::now()+Duration::from_secs(10);
    let _lock=loop{match FileLock::exclusive(&root.join("registry.lock")){
        Ok(lock)=>break lock,
        Err(message) if message.starts_with("Already owned:")=>{
            if Instant::now()>=deadline{return Err(err("Project location registry lock did not clear"));}
            thread::sleep(Duration::from_millis(100));
        },Err(message)=>return Err(err(message))
    }};
    let index=root.join("locations.json");
    let mut document=if index.exists(){read(&index).map_err(err)?}
        else{json!({"schema_version":1,"projects":{}})};
    if document["schema_version"]!=1 || !document["projects"].is_object(){return Err(err("Unsupported project location index"));}
    let entry=json!({"project":project,"state":state});
    let projects=document["projects"].as_object_mut().ok_or_else(||err("Invalid location index"))?;
    let conflicts:Vec<String>=projects.iter().filter(|(other,item)|other.as_str()!=key && item["state"]==entry["state"])
        .map(|(old,_)|old.clone()).collect();
    for old in &conflicts{
        if old.len()!=32 || !old.bytes().all(|byte|byte.is_ascii_hexdigit()){
            return Err(err("Previous project identity is malformed; registration preserved"));
        }
        if projects[old]["project"]!=entry["project"]{
            return Err(err("State directory is registered to another project"));
        }
        if !replaceable_state(state,key)?{
            return Err(err("Previous project state or execution records still exist; registration preserved"));
        }
        owner_gone(old)?;
    }
    if let Some(old)=projects.get(key){
        if old!=&entry{return Err(err("Project key has a different registered location; explicit rebind required"));}
    }
    if !projects.contains_key(key) || !conflicts.is_empty(){
        for old in conflicts{projects.remove(&old);}
        projects.insert(key.into(),entry);
        write(&index,&document,false).map_err(err)?;
    }
    let marker=state.join("project-identity.json");
    write(&marker,&json!({"schema_version":1,"project_key":key}),true).map_err(err)?;
    Ok(())
}
fn request_service(ready:&Ready,kind:&str,body:Value)->io::Result<Value>{
    let mut peer=session(ready.ipc.to_str().ok_or_else(||err("Non-UTF8 IPC path"))?,&ready.key,&ready.release,"command")?;
    let result=request(&mut peer,kind,"bootstrap",body)?;
    if result.kind=="error"{return Err(err(result.body));}
    Ok(result.body)
}
fn unit_state(unit:&str)->io::Result<String>{
    let output=Command::new("systemctl").args(["--user","show","--property=ActiveState","--value",unit]).output()?;
    if !output.status.success(){return Err(err(String::from_utf8_lossy(&output.stderr)));}
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn service_command(root:&Path,ready:&Ready)->Vec<String>{
    let mut command=vec![root.join("bin/waywr-service").to_string_lossy().into_owned(),
        "--project".into(),ready.project.to_string_lossy().into_owned(),
        "--state-dir".into(),ready.state.to_string_lossy().into_owned(),
        "--project-key".into(),ready.key.clone(),"--release-id".into(),ready.release.clone(),
        "--ipc".into(),ready.ipc.to_string_lossy().into_owned(),
        "--runner".into(),root.join("bin/waywr-runner").to_string_lossy().into_owned(),
        "--frontend".into(),root.join("bin/waywr").to_string_lossy().into_owned()];
    if !ready.herdr.as_os_str().is_empty(){command.extend(["--herdr-socket".into(),ready.herdr.to_string_lossy().into_owned()]);}
    command
}
fn ensure_service(root:&Path,ready:&Ready)->io::Result<()> {
    let lock_path=ready.state.join("session/start.lock");
    let lock_deadline=Instant::now()+Duration::from_secs(10);
    let _start_lock=loop{
        match FileLock::exclusive(&lock_path){Ok(lock)=>break lock,Err(error) if error.starts_with("Already owned:")=>{
            if Instant::now()>=lock_deadline{return Err(err("Project service startup lock did not clear"));}
            thread::sleep(Duration::from_millis(100));
        },Err(error)=>return Err(err(error))}
    };
    if request_service(ready,"session.status",json!({})).is_ok(){return Ok(());}
    let deadline=Instant::now()+Duration::from_secs(10);
    loop{
        let state=unit_state(&ready.unit)?;
        if matches!(state.as_str(),"inactive"|"failed"){break;}
        if state=="active" && request_service(ready,"session.status",json!({})).is_ok(){return Ok(());}
        if Instant::now()>=deadline{return Err(io::Error::new(io::ErrorKind::TimedOut,format!("Project service {} did not settle; inspect its journal",ready.unit)));}
        thread::sleep(Duration::from_millis(100));
    }
    let command=service_command(root,ready);
    let loaded=Command::new("systemctl").args(["--user","show","--property=LoadState","--value",&ready.unit]).output()?;
    if !loaded.status.success(){return Err(err("Could not inspect project unit definition"));}
    let state=String::from_utf8_lossy(&loaded.stdout);
    let started=if state.trim()=="loaded"{
        let actual=Command::new("systemctl").args(["--user","show","--property=ExecStart","--value",&ready.unit]).output()?;
        let text=String::from_utf8_lossy(&actual.stdout);
        if !actual.status.success() || !command.iter().all(|arg|text.contains(arg)){
            return Err(err("Loaded project unit differs from this bound release/project"));
        }
        Command::new("systemctl").args(["--user","start",&ready.unit]).output()?
    }else if state.trim()=="not-found"{
        Command::new("systemd-run").args(["--user","--collect",&format!("--unit={}",ready.unit),
            "--property=Restart=on-failure","--property=RestartSec=1s",
            "--property=StartLimitBurst=3","--property=StartLimitIntervalSec=30"])
            .args(&command).output()?
    }else{return Err(err("Could not determine project unit load state"));};
    if !started.status.success(){return Err(err(format!("Project service failed to start: {}",String::from_utf8_lossy(&started.stderr))));}
    let deadline=Instant::now()+Duration::from_secs(10);
    while Instant::now()<deadline{
        if request_service(ready,"session.status",json!({})).is_ok(){return Ok(());}
        thread::sleep(Duration::from_millis(100));
    }
    Err(io::Error::new(io::ErrorKind::TimedOut,format!("Project service {} did not become ready; inspect its journal",ready.unit)))
}
fn prepare(root:&Path,manifest:&Value,args:&[String],setup:bool)->io::Result<Option<Ready>> {
    let project=selected_project(args)?;
    let configuration=project.join(".waywr/project.toml");
    if !configuration.exists(){
        if !setup{return Ok(None);}
        init_project(&project)?;
    }
    let config:toml::Value=toml::from_str(&fs::read_to_string(&configuration)?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?.to_owned();
    let host=config["schema_version"].as_integer()==Some(2) &&
        config.get("execution").and_then(|v|v.get("host")).and_then(toml::Value::as_str)==Some("systemd-user-v1");
    let state_schema=if config.get("conversation").and_then(|v|v.get("schema_version"))
        .and_then(toml::Value::as_integer)==Some(6){9}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(5){8}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(4){7}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(3){6}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(2){5}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(1){4}else{3};
    if !(host || config["schema_version"].as_integer()==Some(1)) || key.len()!=32 || !key.bytes().all(|c|c.is_ascii_hexdigit()){
        return Err(err("Unsupported project identity"));
    }
    let release=manifest["release_id"].as_str().ok_or_else(||err("Release ID missing"))?.to_owned();
    let state=selected(value(args,"--state-dir"),&project,&project.join(".waywr/state"))?;
    register_project(&project,&state,&key,args)?;
    if host{
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":state_schema,"execution_host":"systemd-user-v1","project_key":key}),true).map_err(err)?;
    }
    let endpoint=project.join(".waywr/herdr.json");
    let selected_socket=if host{String::new()}else if endpoint.exists(){
        let existing=read(&endpoint).map_err(err)?;
        if existing["schema_version"]!=1 || existing["project_key"]!=key{
            return Err(err("Herdr endpoint belongs to another project"));
        }
        let recorded=existing["socket"].as_str().ok_or_else(||err("Recorded Herdr socket missing"))?;
        if let Some(requested)=value(args,"--herdr-socket"){
            if requested!=recorded{return Err(err("Herdr endpoint differs from saved project configuration"));}
        }
        recorded.to_owned()
    }else{
        value(args,"--herdr-socket").or_else(||env::var("HERDR_SOCKET_PATH").ok())
            .unwrap_or_else(||format!("{}/.config/herdr/herdr.sock",env::var("HOME").unwrap_or_default()))
    };
    // The supported `herdr` client starts its server on a cold attach. Freeze
    // the explicit endpoint before that socket exists; never infer a focused
    // or unrelated server from the absence of the configured path.
    let socket=PathBuf::from(selected_socket);
    if !host && !socket.is_absolute(){return Err(err("Herdr socket must be absolute"));}
    if !host && !endpoint.exists(){write(&endpoint,&json!({"schema_version":1,"project_key":key,"socket":socket}),true).map_err(err)?;}
    let binding_path=project.join(".waywr/release.json");
    let manifest_hash=format!("{:x}",Sha256::digest(fs::read(root.join("manifest.json"))?));
    if binding_path.exists(){
        let binding=read(&binding_path).map_err(err)?;
        if binding["project_key"]!=key || binding["release_id"]!=release || binding["manifest_sha256"]!=manifest_hash{
            return Err(err("Project is bound to another or changed release; route explicitly"));
        }
    }else{
        if !setup{return Err(err("Existing project has no release binding; use explicit project binding"));}
        write(&binding_path,&json!({"schema_version":1,"project_key":key,"release_id":release,
            "manifest_sha256":manifest_hash,"binding_generation":1}),true).map_err(err)?;
    }
    let runtime=value(args,"--ipc-dir").map(PathBuf::from).or_else(||env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
        .unwrap_or_else(||PathBuf::from(format!("/run/user/{}",unsafe{libc::geteuid()})));
    if !runtime.is_absolute(){return Err(err("IPC directory must be absolute"));}
    let ipc=runtime.join("waywr").join(format!("{key}.sock"));
    if ipc.as_os_str().as_bytes().len()>=108{
        return Err(err("Project IPC socket path exceeds Linux SUN_LEN; select a shorter --ipc-dir before starting the service"));
    }
    let ready=Ready{project,state,key:key.clone(),release,ipc,herdr:socket,unit:format!("waywr-{key}.service")};
    ensure_service(root,&ready)?;
    Ok(Some(ready))
}
pub fn service_call(root:&Path,manifest:&Value,args:&[String],kind:&str,body:Value)->io::Result<Value>{
    let ready=prepare(root,manifest,args,false)?.ok_or_else(||err("Project is not initialized"))?;
    request_service(&ready,kind,body)
}
pub(super) fn service_socket(_project:&Path,args:&[String],key:&str)->io::Result<PathBuf>{
    let runtime=value(args,"--ipc-dir").map(PathBuf::from)
        .or_else(||env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
        .unwrap_or_else(||PathBuf::from(format!("/run/user/{}",unsafe{libc::geteuid()})));
    if !runtime.is_absolute(){return Err(err("IPC directory must be absolute"));}
    Ok(runtime.join("waywr").join(format!("{key}.sock")))
}
pub(super) fn existing_service_call(manifest:&Value,args:&[String],kind:&str,body:Value)->io::Result<Value>{
    let project=selected_project(args)?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    let socket=service_socket(&project,args,key)?;
    let release=manifest["release_id"].as_str().ok_or_else(||err("Release ID missing"))?;
    let mut peer=super::session(socket.to_str().ok_or_else(||err("Non-UTF8 IPC socket"))?,key,release,"command")
        .map_err(|_|err("Project service unavailable; opening a view will not start execution"))?;
    let reply=super::request(&mut peer,kind,"view-control",body)?;
    if reply.kind=="error"{return Err(err(reply.body));}
    Ok(reply.body)
}
pub fn initialize(root:&Path,manifest:&Value,args:&[String])->io::Result<()> {
    let project=init_selection(args)?;
    route_bound_release(root,manifest,&project,args)?;
    if !project.join(".waywr/project.toml").exists(){init_project(&project)?;}
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    let state=selected(value(args,"--state-dir"),&project,&project.join(".waywr/state"))?;
    // Initialization is project-local. Runtime review-link routing registers a
    // location only when the project is actually attached to its service.
    write(&state.join("project-identity.json"),
        &json!({"schema_version":1,"project_key":key}),true).map_err(err)?;
    let host=config["schema_version"].as_integer()==Some(2) &&
        config.get("execution").and_then(|v|v.get("host")).and_then(toml::Value::as_str)==Some("systemd-user-v1");
    let state_schema=if config.get("conversation").and_then(|v|v.get("schema_version"))
        .and_then(toml::Value::as_integer)==Some(6){9}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(5){8}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(4){7}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(3){6}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(2){5}else if config.get("conversation").and_then(|v|v.get("schema_version")).and_then(toml::Value::as_integer)==Some(1){4}else{3};
    if !host && config["schema_version"].as_integer()!=Some(1){return Err(err("Unsupported project execution contract"));}
    if host{
        write(&state.join("state-contract.json"),&json!({"schema_version":1,
            "state_schema":state_schema,"execution_host":"systemd-user-v1","project_key":key}),true).map_err(err)?;
    }
    let existing_endpoint=project.join(".waywr/herdr.json");
    let socket=PathBuf::from(if host{String::new()}else if existing_endpoint.exists(){
        let existing=read(&existing_endpoint).map_err(err)?;
        if existing["project_key"]!=key{return Err(err("Existing Herdr endpoint belongs to another project"));}
        let recorded=existing["socket"].as_str().ok_or_else(||err("Existing Herdr endpoint lacks socket"))?;
        if value(args,"--herdr-socket").as_deref().is_some_and(|requested|requested!=recorded){
            return Err(err("Existing project Herdr endpoint differs; initialization preserved it"));
        }
        recorded.to_owned()
    }else{value(args,"--herdr-socket").or_else(||env::var("HERDR_SOCKET_PATH").ok())
        .unwrap_or_else(||format!("{}/.config/herdr/herdr.sock",env::var("HOME").unwrap_or_default()))});
    if !host && !socket.is_absolute(){return Err(err("Herdr socket must be absolute"));}
    let endpoint=project.join(".waywr/herdr.json");
    if host{}else if endpoint.exists(){
        let existing=read(&endpoint).map_err(err)?;
        if existing["project_key"]!=key || existing["socket"]!=socket.to_string_lossy().as_ref(){
            return Err(err("Existing project Herdr endpoint differs; initialization preserved it"));
        }
    }else{write(&endpoint,&json!({"schema_version":1,"project_key":key,"socket":socket}),true).map_err(err)?;}
    let release=manifest["release_id"].as_str().ok_or_else(||err("Release ID missing"))?;
    let digest=format!("{:x}",Sha256::digest(fs::read(root.join("manifest.json"))?));
    let release_path=project.join(".waywr/release.json");
    if release_path.exists(){
        let existing=read(&release_path).map_err(err)?;
        if existing["project_key"]!=key || existing["release_id"]!=release || existing["manifest_sha256"]!=digest{
            return Err(err("Existing release binding differs; initialization preserved it"));
        }
    }else{write(&release_path,&json!({"schema_version":1,"project_key":key,"release_id":release,
        "manifest_sha256":digest,"binding_generation":1}),true).map_err(err)?;}
    println!("{}",json!({"project":project,"project_key":key,"state":state,
        "release_id":release}));
    Ok(())
}

fn init_selection(args:&[String])->io::Result<PathBuf>{
    let explicit=value(args,"--project").map(PathBuf::from);
    let positional=args.iter().position(|part|part=="init")
        .and_then(|index|args.get(index+1)).filter(|part|!part.starts_with('-')).map(PathBuf::from);
    let current=env::current_dir()?;
    let resolve=|path:PathBuf|->io::Result<PathBuf>{
        let path=if path.is_absolute(){path}else{current.join(path)};
        let mut normalized=PathBuf::new();
        for part in path.components(){match part{
            Component::RootDir=>normalized.push(Path::new("/")),
            Component::CurDir=>{},
            Component::ParentDir=>{normalized.pop();},
            Component::Normal(value)=>normalized.push(value),
            Component::Prefix(_)=>return Err(err("Unsupported init path prefix")),
        }}
        if normalized.exists(){normalized.canonicalize()}else{
            let mut base=normalized.as_path();let mut missing=Vec::new();
            while !base.exists(){
                missing.push(base.file_name().ok_or_else(||err("Init path has no existing ancestor"))?.to_os_string());
                base=base.parent().ok_or_else(||err("Init path has no parent"))?;
            }
            let mut resolved=base.canonicalize()?;
            for part in missing.into_iter().rev(){resolved.push(part);}
            Ok(resolved)
        }
    };
    let named=positional.map(resolve).transpose()?;
    let flagged=explicit.map(resolve).transpose()?;
    if let (Some(a),Some(b))=(&named,&flagged){if a!=b{return Err(err("init directory conflicts with --project"));}}
    let chosen=named.or(flagged).unwrap_or(current);
    fs::create_dir_all(&chosen)?;waywr_backend::project_workflow::canonical_project(&chosen).map_err(err)
}

fn ensure_herdr(ready:&Ready)->io::Result<()> {
    let herdr=waywr_backend::herdr::Herdr::new(&ready.herdr);
    if herdr.server_identity().is_ok(){return Ok(());}
    if ready.herdr.exists(){return Err(err("Configured Herdr socket exists but its server identity is unavailable"));}
    if env::var("HERDR_ENV").ok().as_deref()==Some("1"){
        return Err(err("Invoking Herdr pane has no reachable configured server"));
    }
    Command::new("herdr").arg("server").env("HERDR_SOCKET_PATH",&ready.herdr)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    let until=Instant::now()+Duration::from_secs(10);
    while Instant::now()<until{
        if herdr.server_identity().is_ok(){return Ok(());}
        thread::sleep(Duration::from_millis(100));
    }
    Err(err("Headless Herdr server did not become ready; no UI tab was created"))
}

pub fn launch(root:&Path,manifest:&Value,args:&[String])->io::Result<()> {
    let project=selected_project(args)?;
    route_bound_release(root,manifest,&project,args)?;
    let ready=if let Some(ready)=prepare(root,manifest,args,false)?{ready}else{
        let approved=if args.iter().any(|arg|arg=="--yes"){true}else{
            eprint!("Initialize Wayward Wrangler for {}? [y/N] ",project.display());
            io::stderr().flush()?;let mut response=String::new();io::stdin().read_line(&mut response)?;
            matches!(response.trim().to_ascii_lowercase().as_str(),"y"|"yes")
        };
        if !approved{return Err(err("Project setup declined"));}
        prepare(root,manifest,args,true)?.ok_or_else(||err("Project setup did not complete"))?
    };
    if session_resume(args){
        let until=Instant::now()+Duration::from_secs(40);
        loop{
            // A previous exit may close its socket after the readiness read.
            // Retry only read-only preparation, within the existing bound. A
            // possibly delivered mutation below still retains its exact ID and
            // reports uncertainty; it is never replaced by this retry loop.
            let snapshot=match ensure_service(root,&ready).and_then(|()|
                phase(&mut session(ready.ipc.to_str().ok_or_else(||err("Non-UTF8 IPC"))?,&ready.key,&ready.release,"command")?)){
                Ok(snapshot)=>snapshot,
                Err(error) if matches!(error.kind(),io::ErrorKind::ConnectionReset|io::ErrorKind::ConnectionRefused|io::ErrorKind::NotFound|io::ErrorKind::UnexpectedEof|io::ErrorKind::TimedOut|io::ErrorKind::WouldBlock|io::ErrorKind::Interrupted) && Instant::now()<until=>{
                    thread::sleep(Duration::from_millis(100));continue;
                },Err(error)=>return Err(error),
            };
            let id=format!("resume-{}",random_token().map_err(err)?);
            let command=json!({"command_id":id,"action":"resume_session",
                "expected_state_revision":snapshot["state_revision"],"payload":{}});
            let result=request_service(&ready,"command",command).map_err(|error|err(format!(
                "Resume delivery uncertain for command {id}: {error}. Inspect this command ID before another attempt")))?;
            if result["status"]=="accepted" || result["code"]=="already_running"{break;}
            if (result["code"]=="stale_state" || result["code"]=="service_stopping") && Instant::now()<until{
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            return Err(err(format!("Resume requires attention: {result}")));
        }
    }
    if !ready.herdr.as_os_str().is_empty(){ensure_herdr(&ready)?;}
    let pane=if ready.herdr.as_os_str().is_empty(){None}
        else if env::var("HERDR_ENV").ok().as_deref()==Some("1"){
        let inherited=env::var("HERDR_SOCKET_PATH").map_err(|_|err("Herdr pane lacks socket identity"))?;
        if Path::new(&inherited)!=ready.herdr{return Err(err("Invoking Herdr pane belongs to a different server"));}
        Some(json!({"pane_id":env::var("HERDR_PANE_ID").map_err(|_|err("Herdr pane lacks pane ID"))?,
            "tab_id":env::var("HERDR_TAB_ID").map_err(|_|err("Herdr pane lacks tab ID"))?,
            "workspace_id":env::var("HERDR_WORKSPACE_ID").map_err(|_|err("Herdr pane lacks workspace ID"))?}))
    }else{None};
    let attached=request_service(&ready,"ui.attach_direct",json!({"pane":pane}))?;
    if attached["status"]!="attached"{return Err(err(format!("UI attachment refused: {attached}")));}
    let outbox=ready.state.join("session/outbox");
    run_ui(ready.ipc.to_string_lossy().into_owned(),ready.key,ready.release,
        ready.project.file_name().unwrap_or_default().to_string_lossy().into_owned(),outbox,None,Some(ready.project))
}

pub fn backend_info(root:&Path,manifest:&Value)->io::Result<()> {
    if manifest["backend_kind"]!="rust" || manifest["protocol_major"]!=1 ||
        manifest["frontend"]!=env!("CARGO_PKG_VERSION") || manifest["backend"]!="0.1.0"{
        return Err(err("Rust release manifest/version mismatch"));
    }
    for binary in ["waywr","waywr-service","waywr-runner"]{
        if !root.join("bin").join(binary).is_file(){return Err(err(format!("Missing Rust release binary: {binary}")));}
    }
    println!("{}",json!({"release_id":manifest["release_id"],"frontend":env!("CARGO_PKG_VERSION"),
        "backend":"0.1.0","backend_kind":"rust","protocol_major":1}));
    Ok(())
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    #[test]fn normal_init_has_live_profile_without_seeded_requests_or_fake_workflow(){
        let d=tempfile::tempdir().unwrap();init_project(d.path()).unwrap();let root=d.path().join(".waywr");let p=read(&root.join("coordinator.json")).unwrap();assert_eq!(p["schema_version"],2);assert_eq!(p["coordinator"],"codex");assert!(!root.join("workflow.toml").exists());assert!(!root.join("state/state.sqlite").exists());
        let conflict=tempfile::tempdir().unwrap();fs::create_dir(conflict.path().join(".waywr")).unwrap();fs::write(conflict.path().join(".waywr/unrelated"),"preserve").unwrap();assert!(init_project(conflict.path()).is_err());assert!(!conflict.path().join(".waywr/project.toml").exists());
    }
    #[test]fn canonical_registry_relocation_preserves_other_projects_and_refuses_divergence(){
        let d=tempfile::tempdir().unwrap();let registry=d.path().join("registry");fs::create_dir(&registry).unwrap();let key="a".repeat(32);let project=d.path().join("project");let old=project.join(".wayward-wrangler/state");let new=project.join(".waywr/state");let args=vec!["--registry-dir".into(),registry.to_string_lossy().into_owned()];let file=registry.join("locations.json");let other=json!({"project":"/unrelated","state":"/unrelated/state"});write(&file,&json!({"schema_version":1,"projects":{key.clone():{"project":project,"state":old},"other":other}}),false).unwrap();relocate_registration(&project,&old,&new,&key,&args).unwrap();let document=read(&file).unwrap();assert_eq!(document["projects"]["other"],other);assert_eq!(document["projects"][&key]["state"],json!(new));assert!(relocate_registration(&project,&old,&new,&key,&args).is_ok());assert!(relocate_registration(&project,&old,&project.join("different"),&key,&args).is_err());assert_eq!(read(&file).unwrap(),document);
    }

    fn fixture() -> (tempfile::TempDir,PathBuf,PathBuf,PathBuf,Vec<String>) {
        let temp=tempfile::tempdir().unwrap();
        let project=temp.path().join("project");fs::create_dir(&project).unwrap();
        let state=project.join(".waywr/state");
        let registry=temp.path().join("registry");fs::create_dir(&registry).unwrap();
        let args=vec!["--registry-dir".into(),registry.to_string_lossy().into_owned()];
        (temp,project,state,registry,args)
    }
    fn old_entry(registry:&Path,state:&Path,old_project:&Path) -> Vec<u8> {
        let contents=serde_json::to_vec(&json!({"schema_version":1,"projects":{
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":{"project":old_project,"state":state}
        }})).unwrap();
        fs::write(registry.join("locations.json"),&contents).unwrap();
        contents
    }

    #[test]
    fn missing_same_project_state_can_be_reinitialized_without_reusing_old_identity() {
        let (_temp,project,state,registry,args)=fixture();
        old_entry(&registry,&state,&project);
        let new="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        register_project_checked(&project,&state,new,&args,|_|Ok(())).unwrap();
        let index=read(&registry.join("locations.json")).unwrap();
        assert!(index["projects"].get("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_none());
        assert_eq!(index["projects"][new]["project"],json!(project));
        assert_eq!(read(&state.join("project-identity.json")).unwrap()["project_key"],new);
        register_project_checked(&project,&state,new,&args,|_|panic!("No stale owner on repeat")).unwrap();
    }

    #[test]
    fn fresh_local_init_marker_can_be_registered_on_first_attachment() {
        let (_temp,project,state,registry,args)=fixture();
        old_entry(&registry,&state,&project);
        let new="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        write(&state.join("project-identity.json"),
            &json!({"schema_version":1,"project_key":new}),true).unwrap();
        register_project_checked(&project,&state,new,&args,|_|Ok(())).unwrap();
        let index=read(&registry.join("locations.json")).unwrap();
        assert!(index["projects"].get("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_none());
        assert_eq!(index["projects"][new]["state"],json!(state));
    }

    #[test]
    fn existing_state_or_unverified_owner_preserves_old_registration() {
        let (_temp,project,state,registry,args)=fixture();
        let before=old_entry(&registry,&state,&project);
        fs::create_dir_all(&state).unwrap();
        assert!(register_project_checked(&project,&state,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",&args,
            |_|panic!("State exists")).is_err());
        assert_eq!(fs::read(registry.join("locations.json")).unwrap(),before);
        fs::remove_dir(&state).unwrap();
        assert!(register_project_checked(&project,&state,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",&args,
            |_|Err(err("Service identity unresolved"))).is_err());
        assert_eq!(fs::read(registry.join("locations.json")).unwrap(),before);
        assert!(!state.exists());
    }

    #[test]
    fn local_marker_plus_execution_evidence_is_not_reclaimed() {
        let (_temp,project,state,registry,args)=fixture();
        let before=old_entry(&registry,&state,&project);
        let new="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        write(&state.join("project-identity.json"),
            &json!({"schema_version":1,"project_key":new}),true).unwrap();
        fs::write(state.join("state.sqlite"),b"retained evidence").unwrap();
        assert!(register_project_checked(&project,&state,new,&args,
            |_|panic!("Execution evidence exists")).is_err());
        assert_eq!(fs::read(registry.join("locations.json")).unwrap(),before);
    }

    #[test]
    fn another_projects_registration_cannot_be_reclaimed() {
        let (_temp,project,state,registry,args)=fixture();
        let before=old_entry(&registry,&state,&registry.join("other-project"));
        assert!(register_project_checked(&project,&state,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",&args,
            |_|panic!("Different project")).is_err());
        assert_eq!(fs::read(registry.join("locations.json")).unwrap(),before);
        assert!(!state.exists());
    }

    #[test]
    fn malformed_previous_identity_is_not_used_for_service_or_socket_lookup() {
        let (_temp,project,state,registry,args)=fixture();
        let before=serde_json::to_vec(&json!({"schema_version":1,"projects":{
            "../unknown":{"project":project,"state":state}
        }})).unwrap();
        fs::write(registry.join("locations.json"),&before).unwrap();
        assert!(register_project_checked(&project,&state,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",&args,
            |_|panic!("Malformed identity")).is_err());
        assert_eq!(fs::read(registry.join("locations.json")).unwrap(),before);
    }
}
