//! Explicit Herdr output mirrors. These tabs never host or control workers.
use super::{protocol,request};
use serde_json::{Value,json};
use std::{fs,io::{self,Write},path::{Path,PathBuf},thread,time::{Duration,Instant}};
use waywr_backend::{execution::{read,write,identity_matches,random_token},herdr::Herdr,view_registry};

fn err(message:impl std::fmt::Display)->io::Error{io::Error::other(message.to_string())}
fn arg<'a>(args:&'a [String],name:&str)->Option<&'a str>{args.windows(2).find(|pair|pair[0]==name).map(|pair|pair[1].as_str())}
fn state(project:&Path,args:&[String])->PathBuf{
    let selected=arg(args,"--state-dir").map(PathBuf::from)
        .unwrap_or_else(||project.join(".waywr/state"));
    if selected.is_absolute(){selected}else{project.join(selected)}
}
fn matching_view(herdr:&Herdr,grant:&Value,frontend:&Path)->io::Result<Option<Value>>{
    let token=grant["token"].as_str().ok_or_else(||err("View token missing"))?;
    let matches=herdr.find(token,frontend).map_err(err)?;
    if matches.len()>1{return Err(err("Multiple matching view panes; no new pane created"));}
    Ok(matches.into_iter().next())
}
fn workspace(herdr:&Herdr,state:&Path,project:&Path,key:&str,server:&Value)->io::Result<String>{
    let record=view_registry::workspace_record(state,server).map_err(err)?;
    if record.exists(){
        let prior=read(&record).map_err(err)?;
        if prior["server"]!=*server || prior["project_key"]!=key || prior["project"]!=json!(project){
            return Err(err("Inspection workspace belongs to another project or server"));
        }
        let id=prior["workspace_id"].as_str().ok_or_else(||err("Inspection workspace lacks identity"))?;
        herdr.workspace(id).map_err(|_|err("Owned inspection workspace is unavailable; output remains in Waywr"))?;
        return Ok(id.into());
    }
    let intent=record.with_extension("intent.json");
    let maybe_sent=record.with_extension("maybe-sent.json");
    if !intent.exists(){
        let label=format!("Waywr inspect {} {}",&key[..key.len().min(8)],random_token().map_err(err)?);
        view_registry::record_workspace_intent(&record,&label,server).map_err(err)?;
    }
    let saved=read(&intent).map_err(err)?;
    if saved["server"]!=*server{return Err(err("Inspection workspace intent server changed"));}
    let label=saved["label"].as_str().ok_or_else(||err("Inspection workspace intent lacks label"))?;
    let listed=herdr.call("workspace.list",json!({})).map_err(err)?;
    let matches=listed["workspaces"].as_array().ok_or_else(||err("Herdr omitted workspaces"))?
        .iter().filter(|item|item["label"]==label).collect::<Vec<_>>();
    if matches.len()>1{return Err(err("Multiple matching inspection workspaces; no view created"));}
    let found=if let Some(item)=matches.first(){item["workspace_id"].as_str().map(str::to_owned)}else{None};
    let id=if let Some(id)=found{id}else{
        // An earlier create may have reached Herdr before its reply was lost.
        if maybe_sent.exists(){
            return Err(err("Inspection workspace create reply was lost; inspect Herdr before another create"));
        }
        write(&maybe_sent,&json!({"schema_version":1,"label":label,"server":server,
            "status":"create-maybe-sent"}),true).map_err(err)?;
        let created=herdr.call("workspace.create",json!({"label":label,"cwd":project,"focus":false}))
            .map_err(|e|err(format!("Inspection workspace create uncertain: {e}; output remains in Waywr")))?;
        created["workspace"]["workspace_id"].as_str().ok_or_else(||err("Herdr omitted created workspace identity"))?.to_owned()
    };
    let detail=herdr.workspace(&id).map_err(err)?;
    if detail["workspace"]["label"]!=label{
        return Err(err("Inspection workspace identity/label mismatch"));
    }
    write(&record,&json!({"schema_version":1,"project_key":key,"project":project,
        "server":server,"workspace_id":id,"label":label}),true).map_err(err)?;
    Ok(id)
}

pub fn open(root:&Path,manifest:&Value,args:&[String],task:&str,stage:&str,socket:&str)->io::Result<String> {
    let project=super::selected_project(args)?;
    let state=state(&project,args);
    let endpoint=Path::new(socket);
    if !endpoint.is_absolute(){return Err(err("Explicit Herdr socket path must be absolute"));}
    let herdr=Herdr::new(endpoint);
    let server=herdr.server_identity().map_err(|_|err("Herdr view unavailable; output remains in Waywr"))?;
    let prepared=||super::rust_runtime::existing_service_call(manifest,args,"view.prepare",
        json!({"task_id":task,"stage":stage,"socket":socket,"server":server}));
    let mut grant=prepared()?;
    if grant["server"]!=server{return Err(err("View grant server identity differs"));}
    let frontend=root.join("bin/waywr");
    let mut found=matching_view(&herdr,&grant,&frontend)?;
    if found.is_none(){
        let id=grant["view_id"].as_str().ok_or_else(||err("View grant identity missing"))?;
        if view_registry::binding(&state,id).is_some(){
            grant=super::rust_runtime::existing_service_call(manifest,args,"view.prepare",
                json!({"task_id":task,"stage":stage,"socket":socket,"server":server,
                    "reopen_after":id}))?;
        }else if view_registry::was_sent(&state,id){
            return Err(err("View creation outcome uncertain; no duplicate view sent. Inspect Herdr tabs or retained output."));
        }
        let workspace=workspace(&herdr,&state,&project,
            grant["project_key"].as_str().ok_or_else(||err("View project key missing"))?,&server)?;
        let id=grant["view_id"].as_str().ok_or_else(||err("View ID missing"))?;
        super::rust_runtime::existing_service_call(manifest,args,"view.sent",json!({"view_id":id}))?;
        let project_config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
        let key=project_config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
        let location=super::rust_runtime::service_socket(&project,args,key)?;
        let command=vec![frontend.to_string_lossy().to_string(),"--view-output".into(),
            "--socket".into(),location.to_string_lossy().to_string(),
            "--project-key".into(),key.into(),"--release-id".into(),
            manifest["release_id"].as_str().ok_or_else(||err("Release ID missing"))?.into(),
            "--view-id".into(),id.into(),"--view-token".into(),
            grant["token"].as_str().ok_or_else(||err("View token missing"))?.into(),
            "--view-task".into(),task.into(),"--view-stage".into(),stage.into(),
            "--view-attempt".into(),grant["attempt_id"].as_str().ok_or_else(||err("Attempt ID missing"))?.into()];
        herdr.call("layout.apply",json!({"workspace_id":workspace,
            "tab_label":format!("Waywr read-only {stage}"),"focus":false,
            "root":{"type":"pane","label":format!("Waywr {stage} output"),
                "cwd":project,"command":command}}))
            .map_err(|e|err(format!("Herdr view creation may have been sent: {e}. No automatic second view.")))?;
        let deadline=Instant::now()+Duration::from_secs(3);
        while found.is_none() && Instant::now()<deadline{
            found=matching_view(&herdr,&grant,&frontend)?;
            if found.is_none(){thread::sleep(Duration::from_millis(100));}
        }
    }
    let found=found.ok_or_else(||err("View pane not yet identifiable; no second pane sent"))?;
    if !identity_matches(&server,&herdr.server_identity().map_err(err)?){
        return Err(err("Herdr server identity changed during view creation"));
    }
    let bound=super::rust_runtime::existing_service_call(manifest,args,"view.bind",json!({
        "view_id":grant["view_id"],"pane_id":found["pane_id"],
        "tab_id":found["tab_id"],"workspace_id":found["workspace_id"]}))?;
    herdr.call("tab.focus",json!({"tab_id":found["tab_id"]})).map_err(err)?;
    Ok(format!("Read-only Herdr output view: {} (tab {}). Worker execution is unchanged.",
        bound["pane_id"].as_str().unwrap_or("unknown"),bound["tab_id"].as_str().unwrap_or("unknown")))
}

pub fn viewer(args:&[String])->io::Result<()> {
    let required=|name|arg(args,name).ok_or_else(||err(format!("Missing {name} for read-only viewer")));
    let socket=required("--socket")?;let key=required("--project-key")?;
    let release=required("--release-id")?;let id=required("--view-id")?;
    let token=required("--view-token")?;let task=required("--view-task")?;
    let stage=required("--view-stage")?;let attempt=required("--view-attempt")?;
    let mut peer=protocol::connect_view(socket,key,release,id,token)?;
    let mut cursor=Value::Null;let mut request_id=0u64;
    println!("Wayward Wrangler · read-only {stage} output\nTask: {task}\nClosing this view does not stop the worker.\n");
    loop{
        request_id+=1;
        let reply=request(&mut peer,"operation.output.read",&format!("view-{request_id}"),
            json!({"task_id":task,"stage":stage,"attempt_id":attempt,
                "cursor":cursor,"limit":16*1024}))?;
        if reply.kind!="operation.output.page"{
            return Err(err(format!("View read unavailable: {}",reply.body)));
        }
        let page=&reply.body["page"];
        for segment in page["segments"].as_array().ok_or_else(||err("View page missing segments"))?{
            let bytes=super::rust_cli::decode_hex(segment["data"].as_str().ok_or_else(||err("View segment missing data"))?)?;
            print!("[{}] {}",segment["stream"].as_str().unwrap_or("output"),
                super::rust_cli::safe_terminal(&bytes));
        }
        io::stdout().flush()?;
        cursor=page["cursor"].clone();
        let end=cursor["sequence"].as_u64().unwrap_or(0)>page["high_water"].as_u64().unwrap_or(0);
        if page["terminal"]==true && end{
            println!("\nOutput complete. This retained read-only tab stays open until closed or Ctrl-C; worker execution is unaffected.");
            io::stdout().flush()?;
            loop{thread::sleep(Duration::from_secs(1));}
        }
        thread::sleep(Duration::from_millis(250));
    }
}
