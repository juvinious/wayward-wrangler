//! Project-keyed, read-only Herdr review actions for Rust releases.
use super::{check_project_compatibility,selected_project};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{env,fs,io,path::{Path,PathBuf},process::Command};
use waywr_backend::{execution::{FileLock,read,write,write_bytes},herdr::Herdr,workflow_store::Store};

fn err(message:impl std::fmt::Display)->io::Error{io::Error::other(message.to_string())}
fn value(args:&[String],flag:&str)->Option<String>{
    args.windows(2).find(|pair|pair[0]==flag).map(|pair|pair[1].clone())
}
fn registry(args:&[String])->io::Result<PathBuf>{
    let path=value(args,"--registry-dir").map(PathBuf::from).unwrap_or_else(||{
        env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(||
            PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"))
            .join("wayward-wrangler")});
    if !path.is_absolute(){return Err(err("Review registry must be absolute"));}
    Ok(path)
}
fn valid_id(value:&str)->bool{
    !value.is_empty() && value.len()<=64 && value.bytes().next().is_some_and(|b|b.is_ascii_alphanumeric()) &&
        value.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_')
}
fn parse_link(url:&str)->io::Result<(&str,&str,&str,&str)>{
    let tail=url.strip_prefix("https://awp.invalid/v2/review/")
        .ok_or_else(||err("Review link lacks unambiguous v2 project identity"))?;
    let fields=tail.split('/').collect::<Vec<_>>();
    if fields.len()!=4 || fields[0].len()!=32 || !fields[0].bytes().all(|b|b.is_ascii_hexdigit()) ||
        !valid_id(fields[1]) || !matches!(fields[2],"draft"|"critique-1"|"critique-2"|"revision-1"|"revision-2") ||
        fields[3].len()!=64 || !fields[3].bytes().all(|b|b.is_ascii_hexdigit()){
        return Err(err("Invalid review link identity"));
    }
    Ok((fields[0],fields[1],fields[2],fields[3]))
}
fn project_for_key(args:&[String],key:&str)->io::Result<(PathBuf,PathBuf)>{
    let index=registry(args)?.join("locations.json");
    let document=read(&index).map_err(|_|err("Review location index missing; install/rebind the Herdr integration"))?;
    if document["schema_version"]!=1{return Err(err("Unsupported location index"));}
    let entry=&document["projects"][key];
    let project=entry["project"].as_str().ok_or_else(||err("Review project location missing; rebind it explicitly"))?;
    let state=entry["state"].as_str().ok_or_else(||err("Review state location missing"))?;
    let project=PathBuf::from(project).canonicalize()?;
    let state=PathBuf::from(state).canonicalize()?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    if config["project_key"].as_str()!=Some(key) ||
        read(&state.join("project-identity.json")).map_err(err)?["project_key"]!=key{
        return Err(err("Review project location is stale; rebind it explicitly"));
    }
    Ok((project,state))
}
pub(crate) fn artifact(project:&Path,args:&[String],run:&str,stage:&str,expected:Option<&str>)->io::Result<(PathBuf,Value)>{
    if !valid_id(run) || !matches!(stage,"latest"|"draft"|"critique-1"|"critique-2"|"revision-1"|"revision-2"){
        return Err(err("Invalid run or artifact identity"));
    }
    let state=value(args,"--state-dir").map(PathBuf::from).unwrap_or_else(||project.join(".waywr/state"));
    let state=if state.is_absolute(){state}else{project.join(state)};
    let store=Store::open(&state).map_err(err)?;
    let run_root=store.run_dir(run).map_err(err)?.canonicalize()?;
    let reference=if stage=="latest"{store.snapshot(run).map_err(err)?["state"]["current"].clone()}
        else{read(&run_root.join("artifacts").join(format!("{stage}.json"))).map_err(err)?["manifest"].clone()};
    if reference["id"].as_str().is_none() || (stage!="latest" && reference["id"]!=stage){
        return Err(err("Artifact stage identity mismatch"));
    }
    let hash=reference["sha256"].as_str().ok_or_else(||err("Artifact hash missing"))?;
    if expected.is_some_and(|expected|expected!=hash){return Err(err("Review link hash no longer matches this artifact"));}
    let relative=reference["path"].as_str().ok_or_else(||err("Artifact path missing"))?;
    let path=run_root.join(relative).canonicalize()?;
    if !path.starts_with(&run_root){return Err(err("Artifact path leaves selected run"));}
    if format!("{:x}",Sha256::digest(fs::read(&path)?))!=hash{return Err(err("Artifact bytes changed; review refused"));}
    Ok((path,store.effective_target(run).map_err(err)?))
}
fn editor_tab(root:&Path,project:&Path,args:&[String],run:&str,stage:&str,hash:Option<&str>)->io::Result<()> {
    if env::var("HERDR_ENV").ok().as_deref()!=Some("1"){
        return Err(err("Open review inside Herdr or use its review-link action"));
    }
    let (path,target)=artifact(project,args,run,stage,hash)?;
    let socket=target["socket"].as_str().ok_or_else(||err("Review has no bound Herdr server"))?;
    if env::var("HERDR_SOCKET_PATH").ok().as_deref()!=Some(socket){
        return Err(err("Review belongs to another Herdr server"));
    }
    let herdr=Herdr::new(socket);herdr.server_identity().map_err(err)?;
    let vim=env::var_os("PATH").and_then(|paths|env::split_paths(&paths).map(|root|root.join("vim"))
        .find(|path|path.is_file())).ok_or_else(||err("Vim is not available on PATH"))?;
    let argv=vec![vim.to_string_lossy().into_owned(),"-N".into(),"-u".into(),"NONE".into(),
        "-i".into(),"NONE".into(),"-n".into(),"-M".into(),"-c".into(),"set nomodeline".into(),
        "-c".into(),"syntax on".into(),"--".into(),path.to_string_lossy().into_owned()];
    let state=value(args,"--state-dir").map(PathBuf::from).unwrap_or_else(||project.join(".waywr/state"));
    let state=if state.is_absolute(){state}else{project.join(state)};
    let _lock=FileLock::exclusive(&state.join("review-open.lock")).map_err(err)?;
    // Review display follows the *current verified project UI*, not immutable
    // worker provenance. A completed run's old Herdr workspace may be gone.
    let binding=read(&state.join("session/ui-current.json")).map_err(|_|err("Open Wayward Wrangler in this project before reviewing a link"))?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    if binding["project_key"].as_str()!=config["project_key"].as_str() ||
        !waywr_backend::execution::identity_matches(&binding["server"],&herdr.server_identity().map_err(err)?) {
        return Err(err("Project UI binding belongs to another project or Herdr server; reconnect Wayward Wrangler"));
    }
    let token=binding["token"].as_str().ok_or_else(||err("UI binding lacks token"))?;
    let claim=read(&state.join("session/ui-claims").join(format!("{token}.json"))).map_err(err)?;
    if !waywr_backend::execution::alive(&claim["process"]){return Err(err("Project UI is no longer active; reconnect it"));}
    let found=herdr.find(token,&root.join("bin/waywr")).map_err(err)?;
    if found.len()!=1 || found[0]!=binding["pane"]{
        return Err(err("Project UI pane cannot be verified; review tab refused"));
    }
    let workspace=binding["pane"]["workspace_id"].as_str().ok_or_else(||err("UI workspace missing"))?;
    herdr.workspace(workspace).map_err(err)?;
    let mut matching=Vec::new();
    let tabs=herdr.call("tab.list",json!({})).map_err(err)?;
    for tab in tabs["tabs"].as_array().ok_or_else(||err("Herdr tab listing missing"))?{
        if tab["workspace_id"]!=workspace{continue;}
        let layout=herdr.call("layout.export",json!({"tab_id":tab["tab_id"]})).map_err(err)?["layout"].clone();
        if layout["root"]["type"]=="pane" && layout["root"]["command"]==json!(argv){matching.push(layout);}
    }
    if matching.len()>1{return Err(err("Multiple matching review panes; inspect before focusing"));}
    let layout=if let Some(found)=matching.pop(){found}else{
        let artifact_hash=format!("{:x}",Sha256::digest(fs::read(&path)?));
        let label=format!("Wayward Wrangler review {run} {stage} {}",&artifact_hash[..8]);
        herdr.create(workspace,&label,&argv,project,None).map_err(err)?
    };
    herdr.call("tab.focus",json!({"tab_id":layout["tab_id"]})).map_err(err)?;
    println!("{}",json!({"status":"opened","run_id":run,"artifact":stage,"path":path,
        "tab_id":layout["tab_id"]}));
    Ok(())
}
pub fn open_link(root:&Path,manifest:&Value,args:&[String])->io::Result<()> {
    let clicked=env::var("HERDR_PLUGIN_CLICKED_URL").map_err(|_|err("Review link was not provided by Herdr"))?;
    let (key,run,stage,hash)=parse_link(&clicked)?;
    let (project,state)=project_for_key(args,key)?;
    let mut routed=vec!["--project".to_owned(),project.to_string_lossy().into_owned(),
        "--state-dir".to_owned(),state.to_string_lossy().into_owned()];
    routed.extend_from_slice(args);
    check_project_compatibility(root,manifest,&project,&routed)?;
    editor_tab(root,&project,&routed,run,stage,Some(hash))
}
pub fn open_explicit(root:&Path,manifest:&Value,args:&[String],index:usize)->io::Result<()> {
    let project=selected_project(args)?;
    check_project_compatibility(root,manifest,&project,args)?;
    let mut positionals=Vec::new();let mut cursor=index+1;
    while cursor<args.len(){
        if args[cursor].starts_with("--"){cursor+=2;}else{positionals.push(args[cursor].as_str());cursor+=1;}
    }
    let run=positionals.first().ok_or_else(||err("Use open RUN [--artifact STAGE]"))?;
    let positional=positionals.get(1).copied();
    let selected=value(args,"--artifact");
    if positional.is_some() && selected.is_some(){return Err(err("Choose either positional STAGE or --artifact STAGE"));}
    let stage=selected.as_deref().or(positional).unwrap_or("latest");
    editor_tab(root,&project,args,run,stage,value(args,"--sha256").as_deref())
}
pub fn integration(root:&Path,args:&[String],index:usize)->io::Result<()> {
    let legacy=args.get(index+1).map(String::as_str)==Some("herdr") && args.get(index+2).map(String::as_str)==Some("install");
    if !legacy && args.get(index+1).map(String::as_str)!=Some("install-herdr"){
        return Err(err("Use integration herdr install [--destination DIR]"));
    }
    if let Some(socket)=value(args,"--socket"){
        if !Path::new(&socket).is_absolute(){return Err(err("Herdr socket must be absolute"));}
        Herdr::new(socket).server_identity().map_err(err)?;
    }
    let destination=value(args,"--destination").map(PathBuf::from).unwrap_or_else(||{
        env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(||
            PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/share"))
            .join("wayward-wrangler/herdr")});
    if !destination.is_absolute(){return Err(err("Plugin destination must be absolute"));}
    let registry=registry(args)?;
    let launcher=root.join("bin/waywr");
    let proposed=json!({"schema_version":2,"launcher":launcher,"registry":registry});
    let binding=destination.join("binding.json");
    if binding.exists() && read(&binding).map_err(err)?!=proposed{
        return Err(err("Plugin destination belongs to another installation; choose a new destination"));
    }
    let listing=Command::new("herdr").args(["plugin","list","--json"]).output()?;
    if !listing.status.success(){return Err(err("Could not inspect Herdr plugin registrations"));}
    let plugins:Value=serde_json::from_slice(&listing.stdout)?;
    let matches=plugins["result"]["plugins"].as_array().into_iter().flatten()
        .filter(|item|item["plugin_id"]=="local.agent-workflow-review").collect::<Vec<_>>();
    if matches.len()>1 || matches.first().is_some_and(|item|item["plugin_root"]!=destination.to_string_lossy().as_ref()){
        return Err(err("Herdr review plugin is registered elsewhere; unlink explicitly before changing it"));
    }
    fs::create_dir_all(&destination)?;
    let command=vec![launcher.to_string_lossy().into_owned(),"--registry-dir".into(),
        registry.to_string_lossy().into_owned(),"open-link".into()];
    let rendered=command.iter().map(|piece|serde_json::to_string(piece).unwrap()).collect::<Vec<_>>().join(", ");
    let manifest=format!("id = \"local.agent-workflow-review\"\nname = \"Wayward Wrangler Review\"\nversion = \"0.3.0\"\nmin_herdr_version = \"0.9.3\"\ndescription = \"Open hash-checked Wayward Wrangler artifacts in read-only Vim tabs. No approval.\"\nplatforms = [\"linux\"]\n\n[[actions]]\nid = \"open-review\"\ntitle = \"Open workflow review in Vim\"\ncontexts = [\"pane\"]\ncommand = [{rendered}]\n\n[[link_handlers]]\nid = \"review-artifact\"\ntitle = \"Review artifact in Vim\"\npattern = '^https://awp\\.invalid/v2/review/[0-9a-f]{{32}}/[a-zA-Z0-9][a-zA-Z0-9_-]{{0,63}}/(draft|critique-1|critique-2|revision-1|revision-2)/[0-9a-f]{{64}}$'\naction = \"open-review\"\n");
    write_bytes(&destination.join("herdr-plugin.toml"),manifest.as_bytes(),true).map_err(err)?;
    write(&binding,&proposed,true).map_err(err)?;
    if matches.is_empty(){
        let linked=Command::new("herdr").args(["plugin","link",destination.to_str().ok_or_else(||err("Non-UTF8 plugin destination"))?]).output()?;
        if !linked.status.success(){return Err(err(format!("Herdr plugin link failed: {}",String::from_utf8_lossy(&linked.stderr))));}
    }
    println!("{}",json!({"destination":destination,"registry":registry,"launcher":launcher,
        "registration":if matches.is_empty(){"linked"}else{"already-linked"}}));
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn review_link_requires_explicit_project_and_exact_hash(){
        assert!(parse_link("https://awp.invalid/review/run/draft/").is_err());
        assert!(parse_link(&format!("https://awp.invalid/v2/review/{}/r-a/draft/{}","a".repeat(32),"b".repeat(64))).is_ok());
        assert!(parse_link(&format!("https://awp.invalid/v2/review/{}/r-a/draft/{}","a".repeat(31),"b".repeat(64))).is_err());
    }
    #[test]
    fn review_location_keeps_explicit_state_directory(){
        let fixture=tempfile::tempdir().unwrap();
        let project=fixture.path().join("project");
        let state=fixture.path().join("separate-state");
        let registry=fixture.path().join("registry");
        fs::create_dir_all(project.join(".waywr")).unwrap();
        fs::create_dir_all(&state).unwrap();
        fs::create_dir_all(&registry).unwrap();
        let key="a".repeat(32);
        fs::write(project.join(".waywr/project.toml"),format!("schema_version = 1\nproject_key = \"{key}\"\n")).unwrap();
        fs::write(state.join("project-identity.json"),format!("{{\"project_key\":\"{key}\"}}")).unwrap();
        fs::write(registry.join("locations.json"),serde_json::to_vec(&json!({
            "schema_version":1,"projects":{key.clone():{"project":project,"state":state}}
        })).unwrap()).unwrap();
        let args=vec!["--registry-dir".into(),registry.to_string_lossy().into_owned()];
        assert_eq!(project_for_key(&args,&key).unwrap(),(project,state));
    }
}
