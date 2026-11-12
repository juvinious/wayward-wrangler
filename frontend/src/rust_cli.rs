//! Rust-release diagnostics and storage compatibility boundary.
use super::{check_project_compatibility,rust_runtime,selected_project};
use serde_json::{Value,json};
use waywr_backend::codec::sha256;
use std::{fs,io,os::unix::fs::PermissionsExt,path::{Path,PathBuf}};
use waywr_backend::{execution::{read,write,FileLock,identity_matches},herdr::Herdr,
    operation,session_store::Session};
use std::{thread,time::{Duration,Instant},process::Command,io::Write};

fn err(message:impl std::fmt::Display)->io::Error{io::Error::other(message.to_string())}
fn command(args:&[String])->Option<(usize,&str)>{
    let mut index=0;
    while index<args.len(){
        if matches!(args[index].as_str(),"--project"|"--state-dir"|"--registry-dir"|
            "--legacy-origin"|"--color"|"--herdr-socket"|"--ipc-dir"|"--release"){
            index+=2;continue;
        }
        if args[index].starts_with('-'){index+=1;continue;}
        return Some((index,&args[index]));
    }
    None
}
fn next(args:&[String],index:usize)->Option<&str>{
    let mut cursor=index+1;
    while cursor<args.len(){
        let item=&args[cursor];
        if item.starts_with("--"){
            cursor+=if matches!(item.as_str(),"--watch"|"--json"|"--record-only"|"--once"|"--detach"|"--yes") {1}else{2};
        }else{return Some(item);}
    }
    None
}
fn value<'a>(args:&'a [String],name:&str)->Option<&'a str>{args.windows(2).find(|v|v[0]==name).map(|v|v[1].as_str())}
fn state_dir(project:&Path,args:&[String])->PathBuf{
    let chosen=value(args,"--state-dir").map(PathBuf::from).unwrap_or_else(||project.join(".waywr/state"));
    if chosen.is_absolute(){chosen}else{project.join(chosen)}
}
fn host_project(project:&Path)->io::Result<bool>{
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    Ok(config["schema_version"].as_integer()==Some(2) &&
        config.get("execution").and_then(|value|value.get("host"))
            .and_then(toml::Value::as_str)==Some("systemd-user-v1"))
}
pub(super) fn decode_hex(input:&str)->io::Result<Vec<u8>>{
    if input.len()%2!=0{return Err(err("Malformed output page encoding"));}
    input.as_bytes().chunks_exact(2).map(|pair|{
        let text=std::str::from_utf8(pair).map_err(err)?;
        u8::from_str_radix(text,16).map_err(err)
    }).collect()
}
pub(super) fn safe_terminal(bytes:&[u8])->String{
    String::from_utf8_lossy(bytes).chars().map(|character|{
        if character=='\n' || character=='\t' || !character.is_control(){character.to_string()}
        else{format!("\\u{{{:x}}}",character as u32)}
    }).collect()
}
fn output_pages(root:&Path,manifest:&Value,args:&[String],task:&str,stage:&str,follow:bool)->io::Result<()> {
    let mut cursor=Value::Null;
    let mut attempt:Option<String>=None;
    loop{
        if attempt.is_none(){
            let listed=rust_runtime::service_call(root,manifest,args,"operation.list",
                json!({"task_id":task}))?;
            attempt=listed["operations"].as_array().and_then(|items|items.iter()
                .find(|item|item["stage"]==stage)).and_then(|item|item["attempt_id"].as_str())
                .map(str::to_owned);
            if attempt.is_none(){
                if !follow{println!("No output published for {stage} yet.");return Ok(());}
                thread::sleep(Duration::from_millis(250));continue;
            }
        }
        let body=rust_runtime::service_call(root,manifest,args,"operation.output.read",
            json!({"task_id":task,"stage":stage,"attempt_id":attempt,
                "cursor":cursor,"limit":16*1024}))?;
        let page=&body["page"];
        let segments=page["segments"].as_array().ok_or_else(||err("Output page is missing segments"))?;
        for segment in segments{
            let bytes=decode_hex(segment["data"].as_str().ok_or_else(||err("Output segment is missing bytes"))?)?;
            print!("{}",safe_terminal(&bytes));
        }
        io::stdout().flush()?;
        let next=page["cursor"].clone();
        if next==cursor && !segments.is_empty(){return Err(err("Output cursor did not advance"));}
        cursor=next;
        let at_end=match page["format"].as_str(){
            Some("indexed-v1")=>cursor["sequence"].as_u64().unwrap_or(0)>page["high_water"].as_u64().unwrap_or(0),
            Some("combined-legacy-v1")=>cursor["offset"]==page["high_water"],
            _=>return Err(err("Unknown output page format")),
        };
        if !at_end{continue;}
        if !follow || page["terminal"]==true{break;}
        thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}
fn resolve_task(session:&Session,run:&str)->io::Result<Value>{
    session.task(run).map_err(err)?
        .or(session.task_by_run(run).map_err(err)?)
        .or(if run.starts_with("r-"){None}else{session.task_by_run(&format!("r-{run}")).map_err(err)?})
        .ok_or_else(||err("Run/task not found in this project"))
}
fn operation_dir(project:&Path,args:&[String],run:&str,stage:&str)->io::Result<PathBuf>{
    if !matches!(stage,"draft"|"critique-1"|"revision-1"|"critique-2"|"revision-2"){
        return Err(err("Invalid worker stage"));
    }
    let store=waywr_backend::workflow_store::Store::open(&state_dir(project,args)).map_err(err)?;
    let root=store.run_dir(run).map_err(err)?;
    if !root.is_dir(){return Err(err("Run is absent in this project"));}
    Ok(root.join("operations").join(stage))
}
fn workers(project:&Path,args:&[String],run:&str)->io::Result<Value>{
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let session=Session::open_existing(project,&state_dir(project,args),config["project_key"].as_str().ok_or_else(||err("Project key missing"))?,"cli-read").map_err(err)?;
    let task=resolve_task(&session,run)?;
    let run=task["run_id"].as_str().ok_or_else(||err("Task has no run yet"))?;
    let mut result=Vec::new();
    for stage in ["draft","critique-1","revision-1","critique-2","revision-2"]{
        let dir=operation_dir(project,args,run,stage)?;
        if dir.join("operation.json").exists(){result.push(json!({"stage":stage,"directory":dir,
            "inspection":operation::inspect(&dir,None),"binding":read(&dir.join("binding.json")).ok()}));}
    }
    Ok(json!({"run_id":run,"workers":result}))
}
fn command_id(action:&str,identity:&Value)->io::Result<String>{
    let bytes=serde_json::to_vec(&json!({"action":action,"identity":identity}))?;
    Ok(format!("cli-{}",&sha256(&bytes)[..40]))
}
fn saved_command_path(args:&[String],id:&str)->io::Result<PathBuf>{
    if id.is_empty() || id.len()>120 || !id.bytes().all(|byte|byte.is_ascii_alphanumeric() || byte==b'-' || byte==b'_'){
        return Err(err("Invalid command ID"));
    }
    let project=selected_project(args)?;
    Ok(state_dir(&project,args).join("session/cli-commands").join(format!("{id}.json")))
}
fn deliver_command(root:&Path,manifest:&Value,args:&[String],body:Value)->io::Result<Value>{
    let id=body["command_id"].as_str().ok_or_else(||err("Command ID required"))?.to_owned();
    let path=saved_command_path(args,&id)?;
    let directory=path.parent().ok_or_else(||err("Command path has no parent"))?;
    fs::create_dir_all(directory)?;
    fs::set_permissions(directory,fs::Permissions::from_mode(0o700))?;
    write(&path,&json!({"schema_version":1,"command":body.clone()}),true).map_err(err)?;
    rust_runtime::service_call(root,manifest,args,"command",body).map_err(|error|err(format!(
        "Command delivery uncertain for {id}: {error}. Saved input: {}. Inspect with session command-status {id}; replay the same ID only after inspection",
        path.display())))
}
fn submit_command(root:&Path,manifest:&Value,args:&[String],body:Value)->io::Result<Value>{
    let outcome=deliver_command(root,manifest,args,body)?;
    println!("{}",serde_json::to_string_pretty(&outcome)?);
    if outcome["status"]!="accepted"{return Err(err(format!("Command refused: {} · {}",
        outcome["code"].as_str().unwrap_or("unknown"),
        outcome["message"].as_str().unwrap_or("inspect project status"))));}
    Ok(outcome)
}
fn selected_task(root:&Path,manifest:&Value,args:&[String],run:&str)->io::Result<Value>{
    let (_,session)=offline(root,manifest,args)?;
    let session=session.ok_or_else(||err("No Rust session state yet"))?;
    let task=resolve_task(&session,run)?;
    session.task_detail(task["task_id"].as_str().unwrap()).map_err(err)
}
fn selected_text(project:&Path,raw:&str)->io::Result<String>{
    let path=PathBuf::from(raw);let path=if path.is_absolute(){path}else{project.join(path)};
    let path=path.canonicalize()?;
    if !path.starts_with(project){return Err(err("Selected input file must belong to this project"));}
    let bytes=fs::read(path)?;
    if bytes.len()>32*1024{return Err(err("Selected input file exceeds 32 KiB"));}
    Ok(String::from_utf8(bytes).map_err(err)?.trim().to_owned())
}
fn decide(root:&Path,manifest:&Value,args:&[String],index:usize,action:&str)->io::Result<()> {
    let run=next(args,index).ok_or_else(||err("Use decide/approve/reject RUN --request ID"))?;
    let request=value(args,"--request").ok_or_else(||err("--request required"))?;
    let detail=selected_task(root,manifest,args,run)?;
    let pending=&detail["pending"];
    if pending["request_id"]!=request{return Err(err("Request is not this task's current human gate"));}
    let project=selected_project(args)?;
    let (outcome,answer)=match action{
        "decide"=>("answered",selected_text(&project,value(args,"--answer-file").ok_or_else(||err("--answer-file required"))?)?),
        "approve"=>("approved","Approved".to_owned()),
        "reject"=>("rejected",selected_text(&project,value(args,"--reason-file").ok_or_else(||err("--reason-file required"))?)?),
        _=>return Err(err("Unknown decision action")),
    };
    if action=="approve" && value(args,"--sha256")!=pending["artifact_sha256"].as_str(){
        return Err(err("Approval SHA-256 does not match the exact pending candidate"));
    }
    let task=&detail["task"];
    let payload=json!({"task_id":task["task_id"],"request_id":request,"outcome":outcome,
        "answer":answer,"artifact_sha256":pending["artifact_sha256"],
        "record_only":args.iter().any(|v|v=="--record-only")});
    let body=json!({"command_id":command_id("answer_task",&payload)?,"action":"answer_task",
        "expected_task_version":task["version"],"expected_state_revision":task["state_revision"],"payload":payload});
    submit_command(root,manifest,args,body)?;Ok(())
}
fn resume_recorded(root:&Path,manifest:&Value,args:&[String],run:&str)->io::Result<()> {
    let detail=selected_task(root,manifest,args,run)?;
    let request=detail["pending"]["request_id"].as_str().ok_or_else(||err("No recorded human response is pending"))?;
    let payload=json!({"task_id":detail["task"]["task_id"],"request_id":request});
    submit_command(root,manifest,args,json!({"command_id":command_id("release_recorded_answer",&payload)?,
        "action":"release_recorded_answer","payload":payload}))?;Ok(())
}
fn cancel(root:&Path,manifest:&Value,args:&[String],index:usize)->io::Result<()> {
    let run=next(args,index).ok_or_else(||err("Use cancel RUN --operation STAGE"))?;
    let stage=value(args,"--operation").ok_or_else(||err("--operation required"))?;
    let detail=selected_task(root,manifest,args,run)?;
    if detail["task"]["status"]=="active" && detail["run"]["run_id"].as_str().is_some(){
        let project=selected_project(args)?;
        let store=waywr_backend::workflow_store::Store::open(&state_dir(&project,args)).map_err(err)?;
        let workflow=store.snapshot(detail["run"]["run_id"].as_str().unwrap()).map_err(err)?;
        if workflow["state"]["next_stage"]!=stage{return Err(err("Operation stage is not this task's current worker"));}
    }else{return Err(err("No active worker for the requested operation; inspect task status"));}
    let task=&detail["task"];
    let payload=json!({"task_id":task["task_id"]});
    submit_command(root,manifest,args,json!({"command_id":command_id("cancel_task",&json!({"task":task["task_id"],"stage":stage}))?,
        "action":"cancel_task","expected_task_version":task["version"],
        "expected_state_revision":task["state_revision"],"payload":payload}))?;Ok(())
}
fn rebind(root:&Path,manifest:&Value,args:&[String],index:usize)->io::Result<()> {
    let run=next(args,index).ok_or_else(||err("Use rebind RUN --workspace ID"))?;
    let requested=value(args,"--workspace").ok_or_else(||err("--workspace required"))?;
    let detail=selected_task(root,manifest,args,run)?;
    let actual=detail["run"]["run_id"].as_str().ok_or_else(||err("Task has no run"))?;
    let payload=json!({"run_id":actual,"workspace":requested});
    submit_command(root,manifest,args,json!({"command_id":command_id("rebind_run",&payload)?,
        "action":"rebind_run","payload":payload}))?;Ok(())
}
fn profile_path(project:&Path,args:&[String])->io::Result<PathBuf>{
    if value(args,"--profile").is_some() && value(args,"--config").is_some(){return Err(err("Choose --profile or --config"));}
    let candidate=value(args,"--profile").or_else(||value(args,"--config"))
        .unwrap_or(".waywr/workflow.toml");
    let path=PathBuf::from(candidate);let path=if path.is_absolute(){path}else{project.join(path)};
    let path=path.canonicalize()?;
    if !path.starts_with(project) || !path.is_file(){return Err(err("Workflow profile must be a project-local file"));}
    Ok(path)
}
fn status_only(binary:&str,args:&[&str])->io::Result<std::process::Output>{
    let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(15),
            tokio::process::Command::new(binary).args(args).kill_on_drop(true).output()).await
            .map_err(|_|err("CLI status check exceeded 15 seconds; child stopped, no model call"))?
    })
}
fn verify_auth_status(provider:&str,binary:&str)->io::Result<()> {
    match provider {
        "claude"=>{
            let output=status_only(binary,&["auth","status","--json"])?;
            let logged_in=output.status.success() &&
                serde_json::from_slice::<Value>(&output.stdout).ok()
                    .and_then(|value|value["loggedIn"].as_bool())==Some(true);
            if !logged_in{return Err(err("Claude authentication status unavailable or logged out; no model call"));}
        }
        "codex"=>{
            let output=status_only(binary,&["login","status"])?;
            let combined=format!("{}{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
            if !output.status.success() || !combined.to_ascii_lowercase().contains("logged in"){
                return Err(err("Codex authentication status unavailable or logged out; no model call"));
            }
        }
        _=>return Err(err("Unsupported native provider")),
    }
    Ok(())
}
fn preflight(args:&[String])->io::Result<Value>{
    let project=selected_project(args)?;
    let run=value(args,"--run").ok_or_else(||err("--run required"))?;
    if run.is_empty() || run.len()>64 || !run.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_'){
        return Err(err("Invalid run ID"));
    }
    let brief=selected_text(&project,value(args,"--brief").ok_or_else(||err("--brief required"))?)?;
    if brief.is_empty(){return Err(err("Brief is empty"));}
    let profile=profile_path(&project,args)?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(&profile)?).map_err(err)?;
    let context=value(args,"--context").map(|v|selected_text(&project,v)).transpose()?;
    let has_native=["author","critic"].iter().any(|name|config["roles"][name]["provider"].as_str()!=Some("fake"));
    if has_native && (config["roles"]["author"]["provider"].as_str()!=Some("claude") ||
        config["roles"]["critic"]["provider"].as_str()!=Some("codex")){
        return Err(err("Native workflow requires Claude author and Codex critic in this release"));
    }
    if config["review_limit"].as_integer()!=Some(2){return Err(err("Review limit must be two"));}
    let socket=value(args,"--socket").or_else(||value(args,"--herdr-socket")).map(PathBuf::from);
    let binding_path=project.join(".waywr/herdr.json");
    if binding_path.exists(){
        let bound=read(&binding_path).map_err(err)?;
        if socket.as_ref().is_some_and(|requested|bound["socket"]!=requested.to_string_lossy().as_ref()){
            return Err(err("Preflight Herdr socket differs from this project's bound endpoint"));
        }
    }
    let herdr_server=if let Some(socket)=&socket{
        if !socket.is_absolute(){return Err(err("Herdr socket must be absolute"));}
        Some(Herdr::new(socket).server_identity().map_err(err)?)
    }else{None};
    let workspace=if let Some(requested)=value(args,"--workspace"){
        let socket=socket.as_ref().ok_or_else(||err("--workspace requires a verified Herdr socket"))?;
        let binding=read(&state_dir(&project,args).join("session/ui-current.json")).map_err(|_|err(
            "--workspace requires this project's verified UI binding"))?;
        if binding["pane"]["workspace_id"]!=requested ||
            !identity_matches(&binding["server"],herdr_server.as_ref().unwrap()){
            return Err(err("--workspace is not this project's current verified Herdr workspace"));
        }
        Herdr::new(socket).workspace(requested).map_err(err)?;
        Some(requested)
    }else{None};
    let mut roles=Vec::new();let mut native=false;
    for name in ["author","critic"]{
        let role=&config["roles"][name];let provider=role["provider"].as_str().ok_or_else(||err("Role provider missing"))?;
        if provider!="fake"{
            native=true;
            let role_json=serde_json::to_value(role).map_err(err)?;
            let binary=role["binary"].as_str().ok_or_else(||err("Native role requires absolute CLI binary"))?;
            let output=status_only(binary,&["--version"])?;
            if !output.status.success(){return Err(err(format!("{name} CLI version check failed")));}
            let version_text=format!("{}{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
            let version=waywr_backend::adapters::verify_role(&role_json,&version_text).map_err(err)?;
            verify_auth_status(provider,binary)?;
            roles.push(json!({"role":name,"provider":provider,"model":role["model"],
                "reasoning":role["reasoning"],"version":version}));
        }else{roles.push(json!({"role":name,"provider":"fake","model":role["model"]}));}
    }
    Ok(json!({"project":project,"run_id":run,"brief_sha256":sha256(brief.as_bytes()),
        "profile":profile,"profile_sha256":sha256(&fs::read(&profile)?),
        "context_selected":context.is_some(),
        "context_sha256":context.as_ref().map(|text|sha256(text.as_bytes())),
        "roles":roles,"review_limit":2,"max_worker_invocations":5,
        "native_clis_authenticated":native,
        "admission":if native{"blocked-native-isolation"}else{"simulated-ready"},
        "herdr_socket":socket,"herdr_server":herdr_server,"workspace":workspace}))
}
fn location_rebind(args:&[String])->io::Result<()> {
    let project=selected_project(args)?;
    let state=state_dir(&project,args).canonicalize()?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    if read(&state.join("project-identity.json")).map_err(err)?["project_key"]!=key{
        return Err(err("Project/state identity mismatch"));
    }
    let unit=format!("waywr-{key}.service");
    let active=Command::new("systemctl").args(["--user","show","--property=ActiveState","--value",&unit]).output()?;
    if !active.status.success() || !matches!(String::from_utf8_lossy(&active.stdout).trim(),"inactive"|"failed"){
        return Err(err("Project service must be inactive before location rebind"));
    }
    let _service=FileLock::exclusive(&state.join("session/service.lock")).map_err(err)?;
    let session=Session::open_existing(&project,&state,key,"maintenance").map_err(err)?;
    let revision=session.quiescent_for_release_change().map_err(err)?;
    let registry=value(args,"--registry-dir").map(PathBuf::from).unwrap_or_else(||{
        std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(||
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"))
            .join("wayward-wrangler")
    });
    if !registry.is_absolute(){return Err(err("Registry path must be absolute"));}
    fs::create_dir_all(&registry)?;
    let _lock=FileLock::exclusive(&registry.join("registry.lock")).map_err(err)?;
    let index=registry.join("locations.json");
    let mut document=read(&index).map_err(err)?;
    if document["schema_version"]!=1{return Err(err("Unsupported location index"));}
    let previous=document["projects"][key].clone();
    if !previous.is_object(){return Err(err("Project is not registered; initialize it explicitly"));}
    let replacement=json!({"project":project,"state":state});
    if document["projects"].as_object().is_some_and(|entries|entries.iter().any(|(other,entry)|
        other!=key && entry["state"]==replacement["state"])){
        return Err(err("State location belongs to another registered project"));
    }
    if previous!=replacement{
        let history=registry.join("rebindings");fs::create_dir_all(&history)?;
        let record=json!({"schema_version":1,"project_key":key,"previous":previous,"next":replacement,
            "state_revision":revision});
        let digest=sha256(&serde_json::to_vec(&record)?);
        write(&history.join(format!("{key}-{}.json",&digest[..16])),&record,true).map_err(err)?;
        document["projects"][key]=replacement.clone();write(&index,&document,false).map_err(err)?;
    }
    println!("{}",serde_json::to_string_pretty(&json!({"status":"bound","project_key":key,
        "location":replacement,"state_revision":revision}))?);Ok(())
}
fn start(root:&Path,manifest:&Value,args:&[String])->io::Result<()> {
    let project=selected_project(args)?;
    let run=value(args,"--run").map(str::to_owned).unwrap_or_else(||format!("cli-{}",&sha256(format!("{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()).as_bytes())[..16]));
    let brief_arg=value(args,"--brief").ok_or_else(||err("Use start --brief FILE"))?;
    let mut check=args.to_vec();
    if value(args,"--run").is_none(){check.push("--run".into());check.push(run.clone());}
    if value(args,"--context").is_none(){
        // The native baseline requires explicit context; the simulated v1
        // profile has no selected context dependency.
        check.push("--context".into());check.push(brief_arg.into());
    }
    let report=preflight(&check)?;
    if report["admission"]!="simulated-ready"{
        println!("{}",serde_json::to_string_pretty(&report)?);
        return Err(err("Native admission remains blocked until the context/tool isolation gate passes"));
    }
    let brief=selected_text(&project,brief_arg)?;
    let snapshot=rust_runtime::service_call(root,manifest,args,"snapshot",json!({}))?;
    if let Some(socket)=value(args,"--socket"){
        let bound=read(&project.join(".waywr/herdr.json")).map_err(err)?;
        if bound["socket"]!=socket{return Err(err("--socket differs from the project's bound Herdr endpoint"));}
    }
    if let Some(workspace)=value(args,"--workspace"){
        let binding=read(&state_dir(&project,args).join("session/ui-current.json")).map_err(err)?;
        if binding["pane"]["workspace_id"]!=workspace{return Err(err("--workspace is not this project's verified UI workspace"));}
    }
    let identity=json!({"run_id":run,"brief_path":brief_arg,"brief_sha256":report["brief_sha256"],
        "profile_path":report["profile"],"profile_sha256":report["profile_sha256"],
        "context_path":value(args,"--context"),"context_sha256":report["context_sha256"],
        "workspace":value(args,"--workspace"),"socket":value(args,"--socket")});
    let first=json!({"command_id":command_id("submit_message",&identity)?,
        "action":"submit_message","expected_state_revision":snapshot["state_revision"],
        "payload":{"text":brief,"run_id":run}});
    let frozen=state_dir(&project,args).join("session/cli-start").join(format!("{run}.json"));
    let submission=if frozen.exists(){
        let saved=read(&frozen).map_err(err)?;
        if saved["identity"]!=identity || saved["command"]["payload"]!=first["payload"]{
            return Err(err("Frozen CLI start inputs differ; inspect the existing run instead of retrying"));
        }
        saved["command"].clone()
    }else{
        let saved=json!({"schema_version":1,"identity":identity,"command":first});
        write(&frozen,&saved,true).map_err(err)?;
        saved["command"].clone()
    };
    let submitted=submit_command(root,manifest,args,submission)?;
    let task=submitted["task_id"].as_str().ok_or_else(||err("Task ID missing after acceptance"))?;
    let deadline=Instant::now()+Duration::from_secs(30);
    let detail=loop{
        let detail=rust_runtime::service_call(root,manifest,args,"task.get",json!({"task_id":task}))?;
        if detail["task"]["status"]=="proposed"{break detail;}
        if detail["task"]["run_id"]==run && ["ready","active","awaiting-input","paused","approved","rejected"].contains(&detail["task"]["status"].as_str().unwrap_or("")){
            println!("{}",serde_json::to_string_pretty(&detail)?);
            return Ok(());
        }
        if ["failed","blocked","uncertain","cancelled"].contains(&detail["task"]["status"].as_str().unwrap_or("")){
            return Err(err(format!("Coordinator stopped before proposal; task {task}: {}",detail["task"]["status"])));
        }
        if Instant::now()>=deadline{return Err(err(format!("Task {task} remains {}. Inspect status; no coordinator retry was launched",detail["task"]["status"])));}
        thread::sleep(Duration::from_millis(250));
    };
    let profile=value(args,"--profile").or_else(||value(args,"--config")).unwrap_or(".waywr/workflow.toml");
    let plan=rust_runtime::service_call(root,manifest,args,"task.plan",json!({"task_id":task,"profile":profile,
        "context":value(args,"--context")}))?;
    if let Some(workspace)=value(args,"--workspace"){
        if plan["plan"]["workspace"]!=workspace{return Err(err("Planned workspace differs from explicit selection"));}
    }
    println!("{}",serde_json::to_string_pretty(&plan)?);
    if let Some(expected)=value(args,"--authorize-native"){
        if plan["plan_digest"]!=expected{return Err(err("Plan digest authorization mismatch"));}
    }else{
        eprint!("Authorize this simulated plan for task {task}? [y/N] ");std::io::stderr().flush()?;
        let mut answer=String::new();std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(),"y"|"yes"){
            return Err(err(format!("Task {task} remains proposed; no worker was authorized")));
        }
    }
    let selected=&plan["plan"];
    let payload=json!({"task_id":task,"run_id":selected["run_id"],"workspace":selected["workspace"],
        "profile":selected["profile"],"context":selected["context"],"plan_digest":plan["plan_digest"]});
    submit_command(root,manifest,args,json!({"command_id":command_id("admit_task",&payload)?,
        "action":"admit_task","expected_task_version":detail["task"]["version"],
        "expected_state_revision":detail["task"]["state_revision"],"payload":payload}))?;
    Ok(())
}
fn supervise(root:&Path,manifest:&Value,args:&[String],index:usize,install:bool)->io::Result<()> {
    let run=next(args,index).ok_or_else(||err("Use supervise RUN --socket PATH"))?;
    let project=selected_project(args)?;
    let recorded=read(&project.join(".waywr/herdr.json")).map_err(err)?;
    if recorded["socket"]!=value(args,"--socket").ok_or_else(||err("--socket required"))?{
        return Err(err("Explicit socket differs from the project's bound Herdr endpoint"));
    }
    if let Some(interval)=value(args,"--interval"){
        if interval!="2.0" && interval!="2"{return Err(err("Project service owns its fixed scheduler interval; custom --interval is unsupported"));}
    }
    if let Some(attempts)=value(args,"--max-attempts"){
        if attempts!="3"{return Err(err("Project service owns three bounded recovery attempts; custom --max-attempts is unsupported"));}
    }
    let detail=selected_task(root,manifest,args,run)?;
    let task_id=detail["task"]["task_id"].as_str().ok_or_else(||err("Task ID missing"))?;
    let key=offline(root,manifest,args)?.0["project_key"].as_str().ok_or_else(||err("Project key missing"))?.to_owned();
    let _=rust_runtime::service_call(root,manifest,args,"snapshot",json!({}))?;
    if install || args.iter().any(|v|v=="--once"){
        println!("{}",serde_json::to_string_pretty(&json!({"status":"supervised",
            "task_id":task_id,"service_unit":format!("waywr-{key}.service"),
            "scheduler":"project-owned; no additional supervisor launched"}))?);
        return Ok(());
    }
    loop{
        let detail=rust_runtime::service_call(root,manifest,args,"task.get",json!({"task_id":task_id}))?;
        println!("{}",serde_json::to_string_pretty(&detail)?);
        if ["approved","rejected","failed","cancelled","uncertain","blocked","output-invalid"]
            .contains(&detail["task"]["status"].as_str().unwrap_or("")){break;}
        thread::sleep(Duration::from_secs(2));
    }
    Ok(())
}
fn offline(_root:&Path,manifest:&Value,args:&[String])->io::Result<(Value,Option<Session>)>{
    let project=selected_project(args)?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    let release=manifest["release_id"].as_str().ok_or_else(||err("Release ID missing"))?;
    let state=state_dir(&project,args);
    if !state.join("state.sqlite").is_file(){
        return Ok((json!({"project_key":key,"release_id":release,"state_revision":0,
            "session":{"phase":"not-started"},"tasks":[],"recent_events":[],"cursor":0}),None));
    }
    let session=Session::open_existing(&project,&state,key,release).map_err(err)?;
    Ok((session.snapshot().map_err(err)?,Some(session)))
}
pub fn legacy(root:&Path,manifest:&Value,args:&[String])->io::Result<()> {
    let (index,action)=command(args).ok_or_else(||err("Missing command"))?;
    if action=="legacy-inspect"{
        if value(args,"--legacy-origin").is_some(){return Err(err("legacy-inspect uses the selected state directly; --legacy-origin is unsupported"));}
        let run=next(args,index).ok_or_else(||err("Use legacy-inspect RUN --state-dir DIR"))?;
        let state=args.windows(2).find(|pair|pair[0]=="--state-dir")
            .map(|pair|std::path::PathBuf::from(&pair[1])).ok_or_else(||err("Explicit --state-dir required for historical inspection"))?;
        let report=waywr_backend::legacy::LegacyReader::open(&state).map_err(err)?.inspect(run).map_err(err)?;
        println!("{}",serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let project=selected_project(args)?;
    check_project_compatibility(root,manifest,&project,args)?;
    if value(args,"--legacy-origin").is_some(){return Err(err("--legacy-origin applies only to an explicitly bound legacy release"));}
    if value(args,"--release").is_some(){return Err(err("--release is only valid for session bind/upgrade/rollback"));}
    if value(args,"--color")==Some("always"){
        return Err(err("Rust CLI diagnostics emit plain JSON/text; --color always is unsupported"));
    }
    match action{
        "init"=>rust_runtime::initialize(root,manifest,args),
        "location-rebind"=>location_rebind(args),
        "preflight"=>{let report=preflight(args)?;println!("{}",serde_json::to_string_pretty(&report)?);
            if report["admission"]=="blocked-native-isolation"{return Err(err("Native preflight verified CLI inputs but admission is blocked by native-isolation gate"));}
            Ok(())},
        "start"=>start(root,manifest,args),
        "supervise"=>supervise(root,manifest,args,index,false),
        "supervisor-install"=>supervise(root,manifest,args,index,true),
        "status"=>{
            loop {
                let (snapshot,session)=offline(root,manifest,args)?;
                let result=if let Some(run)=next(args,index){
                    let session=session.ok_or_else(||err("No Rust session state yet"))?;
                    let task=resolve_task(&session,run)?;
                    session.task_detail(task["task_id"].as_str().unwrap()).map_err(err)?
                }else{snapshot};
                println!("{}",serde_json::to_string_pretty(&result)?);
                if !args.iter().any(|arg|arg=="--watch"){break;}
                thread::sleep(Duration::from_secs(2));
            }
            Ok(())
        }
        "show"=>{
            let run=next(args,index).ok_or_else(||err("Use show RUN [--artifact STAGE]"))?;
            let project=selected_project(args)?;
            let detail=selected_task(root,manifest,args,run)?;
            if host_project(&project)?{
                let task=detail["task"]["task_id"].as_str().ok_or_else(||err("Task ID missing"))?;
                let artifact=value(args,"--artifact").unwrap_or("latest");
                let mut offset=0u64;
                loop{
                    let page=rust_runtime::service_call(root,manifest,args,"artifact.read",
                        json!({"task_id":task,"artifact":artifact,"offset":offset,"limit":16*1024}))?;
                    let bytes=decode_hex(page["data"].as_str().ok_or_else(||err("Artifact page missing bytes"))?)?;
                    print!("{}",safe_terminal(&bytes));
                    let next=page["next_offset"].as_u64().ok_or_else(||err("Artifact cursor missing"))?;
                    if next==offset && page["complete"]!=true{return Err(err("Artifact cursor did not advance"));}
                    offset=next;
                    if page["complete"]==true{break;}
                }
                io::stdout().flush()?;return Ok(());
            }
            let actual=detail["run"]["run_id"].as_str().ok_or_else(||err("Task has no run yet"))?;
            let (path,_)=super::rust_review::artifact(&project,args,actual,value(args,"--artifact").unwrap_or("latest"),None)?;
            print!("{}",safe_terminal(&fs::read(path)?));Ok(())
        }
        "notices"=>{
            let snapshot=offline(root,manifest,args)?.0;
            let run=next(args,index).ok_or_else(||err("Use notices RUN"))?;
            let task=selected_task(root,manifest,args,run)?["task"].clone();
            println!("{}",serde_json::to_string_pretty(&json!({"task":task,
                "service_attention":snapshot["service_attention"],"session_notice":snapshot["session"]["notice"]}))?);Ok(())
        }
        "workers"=>{
            let project=selected_project(args)?;
            let run=next(args,index).ok_or_else(||err("Use workers RUN"))?;
            if host_project(&project)?{
                let task=selected_task(root,manifest,args,run)?["task"]["task_id"]
                    .as_str().ok_or_else(||err("Task ID missing"))?.to_owned();
                println!("{}",serde_json::to_string_pretty(&rust_runtime::service_call(root,manifest,args,
                    "operation.list",json!({"task_id":task}))?)?);return Ok(());
            }
            println!("{}",serde_json::to_string_pretty(&workers(&project,args,run)?)?);Ok(())
        }
        "logs"=>{
            let project=selected_project(args)?;
            let run=next(args,index).ok_or_else(||err("Use logs RUN --operation STAGE"))?;
            let detail=selected_task(root,manifest,args,run)?;
            let stage=value(args,"--operation").ok_or_else(||err("--operation required"))?;
            if host_project(&project)?{
                return output_pages(root,manifest,args,detail["task"]["task_id"].as_str()
                    .ok_or_else(||err("Task ID missing"))?,stage,false);
            }
            let actual=detail["run"]["run_id"].as_str().ok_or_else(||err("Task has no run yet"))?;
            let dir=operation_dir(&project,args,actual,stage)?;
            let transcript=dir.join("transcript.log");
            if !dir.join("operation.json").exists(){return Err(err("Operation has not been prepared"));}
            print!("{}",safe_terminal(&fs::read(transcript)?));Ok(())
        }
        "observe"=>{
            let project=selected_project(args)?;
            let run=next(args,index).ok_or_else(||err("Use observe RUN --operation STAGE"))?;
            let detail=selected_task(root,manifest,args,run)?;
            let stage=value(args,"--operation").ok_or_else(||err("--operation required"))?;
            if host_project(&project)?{
                if value(args,"--viewer")==Some("herdr"){
                    let socket=value(args,"--socket").or_else(||value(args,"--herdr-socket"))
                        .ok_or_else(||err("Explicit --socket PATH required for Herdr view"))?;
                    let opened=super::rust_views::open(root,manifest,args,
                        detail["task"]["task_id"].as_str().ok_or_else(||err("Task ID missing"))?,stage,socket)?;
                    println!("{opened}");return Ok(());
                }
                return output_pages(root,manifest,args,detail["task"]["task_id"].as_str()
                    .ok_or_else(||err("Task ID missing"))?,stage,true);
            }
            let actual=detail["run"]["run_id"].as_str().ok_or_else(||err("Task has no run yet"))?;
            let dir=operation_dir(&project,args,actual,stage)?;
            let binding=read(&dir.join("binding.json")).map_err(err)?;
            let launch=read(&dir.join("launch.json")).map_err(err)?;
            let socket=launch["target"]["socket"].as_str().ok_or_else(||err("Worker Herdr socket missing"))?;
            let herdr=Herdr::new(socket);
            if !identity_matches(&launch["server"],&herdr.server_identity().map_err(err)?){return Err(err("Original Herdr server is unavailable; inspect retained output"));}
            let op=read(&dir.join("operation.json")).map_err(err)?;
            let matches=herdr.find(op["launch_token"].as_str().ok_or_else(||err("Launch token missing"))?,&root.join("bin/waywr-runner")).map_err(err)?;
            if matches.len()!=1 || matches[0]["pane_id"]!=binding["pane_id"] || matches[0]["tab_id"]!=binding["tab_id"]{
                return Err(err("Worker pane identity cannot be verified; inspect retained output"));
            }
            herdr.call("tab.focus",json!({"tab_id":binding["tab_id"]})).map_err(err)?;
            println!("{}",json!({"status":"focused","worker":value(args,"--operation"),"binding":binding}));Ok(())
        }
        "open-link"=>super::rust_review::open_link(root,manifest,args),
        "open"=>super::rust_review::open_explicit(root,manifest,args,index),
        "integration"=>super::rust_review::integration(root,args,index),
        "rebind"=>rebind(root,manifest,args,index),
        "resume"=>resume_recorded(root,manifest,args,next(args,index).ok_or_else(||err("Use waywr resume for project session, or resume RUN for a record-only answer"))?),
        "demo"|"open-demo"|
        "probe-start"|"probe-resume"|"probe-cancel"|"probe-observe"|
        "adopt-saved-draft"|"adopt-provisional-revision"|"adopt-reviewed-critique"=>
            Err(err("This historical command belongs to the explicitly bound legacy release; Rust project workflow is controlled through waywr")),
        "cancel"=>cancel(root,manifest,args,index),
        "decide"|"approve"|"reject"=>decide(root,manifest,args,index,action),
        _=>Err(err(format!("Unsupported Rust-release command: {action}")))
    }
}
pub fn session(root:&Path,manifest:&Value,args:&[String])->io::Result<()> {
    let project=selected_project(args)?;
    let (index,_)=command(args).ok_or_else(||err("Missing session command"))?;
    let action=next(args,index).ok_or_else(||err("Use session status, stop, bind, upgrade or rollback"))?;
    if action=="reconcile-request-routing"{return super::rust_maintenance::reconcile_request_routing(args)}
    if action=="repair-answer-history"{return super::rust_maintenance::repair_answer_history(args)}
    if action=="migrate"{return super::rust_migrate::migrate(root,manifest,args)}
    if !matches!(action,"bind"|"upgrade"|"rollback"){
        check_project_compatibility(root,manifest,&project,args)?;
    }
    if value(args,"--legacy-origin").is_some(){return Err(err("--legacy-origin applies only to an explicitly bound legacy release"));}
    if value(args,"--release").is_some() && !matches!(action,"bind"|"upgrade"|"rollback"){
        return Err(err("--release is only valid for session bind/upgrade/rollback"));
    }
    match action{
        "status"=>{
            let (snapshot,session)=offline(root,manifest,args)?;
            let result=if let Some(session)=session{
                json!({"session":snapshot["session"],"project_key":snapshot["project_key"],
                    "state_revision":snapshot["state_revision"],
                    "checkpoint":session.workflow.state.join("session/checkpoints/latest.json")})
            }else{json!({"session":snapshot["session"],"project_key":snapshot["project_key"],
                "state_revision":0})};
            println!("{}",serde_json::to_string_pretty(&result)?);Ok(())
        }
        "stop"=>{
            for _ in 0..4 {
                let (snapshot,session)=offline(root,manifest,args)?;
                if session.is_none() || snapshot["session"]["phase"]!="running"{
                    println!("{}",json!({"status":"accepted","already_stopped":true,"session":snapshot["session"]}));
                    return Ok(());
                }
                let id=format!("cli-exit-{}",waywr_backend::execution::random_token().map_err(err)?);
                let result=deliver_command(root,manifest,args,json!({
                    "command_id":id,"action":"stop_session",
                    "expected_state_revision":snapshot["state_revision"],"payload":{}}))
                    ?;
                if result["code"]=="stale_state"{continue;}
                println!("{}",serde_json::to_string_pretty(&result)?);
                if result["status"]!="accepted" && result["code"]!="already_paused"{
                    return Err(err("Session stop was refused; inspect the response"));
                }
                return Ok(());
            }
            Err(err("Session changed during four stop attempts; no stop was accepted"))
        }
        "command-status"=>{
            let id=args.iter().position(|value|value=="command-status").and_then(|position|next(args,position))
                .ok_or_else(||err("Use session command-status ID"))?;
            let path=saved_command_path(args,id)?;
            let result=rust_runtime::service_call(root,manifest,args,"command_get",json!({"command_id":id}))?;
            println!("{}",serde_json::to_string_pretty(&json!({"command_id":id,
                "saved_input":if path.is_file(){Some(path)}else{None},"result":result}))?);Ok(())
        }
        "command-replay"=>{
            let id=args.iter().position(|value|value=="command-replay").and_then(|position|next(args,position))
                .ok_or_else(||err("Use session command-replay ID"))?;
            let path=saved_command_path(args,id)?;
            let saved=read(&path).map_err(|_|err("No saved CLI command for this project and ID"))?;
            if saved["schema_version"]!=1 || saved["command"]["command_id"]!=id{
                return Err(err("Saved CLI command identity is invalid"));
            }
            submit_command(root,manifest,args,saved["command"].clone())?;Ok(())
        }
        "bind"|"upgrade"|"rollback"=>super::rust_maintenance::change(root,manifest,args,action),
        "recover-coordinator"=>{
            let task=args.iter().position(|v|v=="recover-coordinator").and_then(|i|next(args,i))
                .ok_or_else(||err("Use session recover-coordinator TASK_ID"))?;
            let payload=json!({"task_id":task});
            submit_command(root,manifest,args,json!({"command_id":command_id("recover_coordinator",&payload)?,
                "action":"recover_coordinator","payload":payload}))?;Ok(())
        },
        "retry-start"=>{
            let task=args.iter().position(|v|v=="retry-start").and_then(|i|next(args,i))
                .ok_or_else(||err("Use session retry-start TASK_ID"))?;
            let detail=rust_runtime::service_call(root,manifest,args,"task.get",json!({"task_id":task}))?;
            let row=&detail["task"];
            let payload=json!({"task_id":task,"task_state_revision":row["state_revision"]});
            submit_command(root,manifest,args,json!({"command_id":command_id("operation.retry_start",&payload)?,
                "action":"operation.retry_start","expected_task_version":row["version"],
                "expected_state_revision":row["state_revision"],"payload":{"task_id":task}}))?;Ok(())
        },
        _=>Err(err("Unknown project session action"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn status_only_auth_checks_use_mock_clis_without_a_model_call(){
        let root=tempfile::tempdir().unwrap();
        let cli=root.path().join("mock-agent");
        fs::write(&cli,"#!/bin/sh\ncase \"$1\" in\n auth) echo '{\"loggedIn\":true}';;\n login) echo 'Logged in';;\n *) exit 3;;\nesac\n").unwrap();
        fs::set_permissions(&cli,fs::Permissions::from_mode(0o700)).unwrap();
        let binary=cli.to_str().unwrap();
        verify_auth_status("claude",binary).unwrap();
        verify_auth_status("codex",binary).unwrap();
        fs::write(&cli,"#!/bin/sh\necho '{\"loggedIn\":false}'\n").unwrap();
        assert!(verify_auth_status("claude",binary).is_err());
        assert!(verify_auth_status("codex",binary).is_err());
    }
    #[test]
    fn native_preflight_checks_mock_status_but_keeps_admission_blocked(){
        let root=tempfile::tempdir().unwrap();let project=root.path();
        let claude=project.join("mock-claude");let codex=project.join("mock-codex");
        fs::write(&claude,"#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo '2.1.296'; elif [ \"$1\" = 'auth' ]; then echo '{\"loggedIn\":true}'; else exit 9; fi\n").unwrap();
        fs::write(&codex,"#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo '0.162.1'; elif [ \"$1\" = 'login' ]; then echo 'Logged in'; else exit 9; fi\n").unwrap();
        for path in [&claude,&codex]{fs::set_permissions(path,fs::Permissions::from_mode(0o700)).unwrap();}
        fs::write(project.join("brief.md"),"Specify a local decision log.\n").unwrap();
        fs::write(project.join("context.md"),"Project-only context.\n").unwrap();
        fs::write(project.join("workflow.toml"),format!(
            "review_limit = 2\n[roles.author]\nprovider = 'claude'\nmodel = 'sonnet'\nreasoning = 'medium'\nbinary = '{}'\n[roles.critic]\nprovider = 'codex'\nmodel = 'gpt-6.1-sol'\nreasoning = 'high'\nbinary = '{}'\n",
            claude.display(),codex.display())).unwrap();
        let args=["--project",project.to_str().unwrap(),"preflight","--run","mock-run",
            "--brief","brief.md","--profile","workflow.toml","--context","context.md"]
            .iter().map(|part|part.to_string()).collect::<Vec<_>>();
        let report=preflight(&args).unwrap();
        assert_eq!(report["native_clis_authenticated"],true);
        assert_eq!(report["admission"],"blocked-native-isolation");
        fs::create_dir_all(project.join(".waywr")).unwrap();
        fs::write(project.join(".waywr/herdr.json"),"not-json").unwrap();
        assert!(preflight(&args).unwrap_err().to_string().contains("expected"));
    }
}
