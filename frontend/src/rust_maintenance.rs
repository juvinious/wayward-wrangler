//! Explicit, project-scoped Rust release binding. Never touches another project.
use super::{rust_bundle,selected_project};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{fs,io::{self,Read},path::{Path,PathBuf},process::Command};
use waywr_backend::{execution::{read,write,FileLock},host_attempt::Attempt,session_store::Session};

fn err(message:impl std::fmt::Display)->io::Error{io::Error::other(message.to_string())}
fn value(args:&[String],flag:&str)->Option<String>{
    args.windows(2).find(|pair|pair[0]==flag).map(|pair|pair[1].clone())
}
fn state_path(project:&Path,args:&[String])->io::Result<PathBuf>{
    let chosen=value(args,"--state-dir").map(PathBuf::from).unwrap_or_else(||project.join(".waywr/state"));
    Ok(if chosen.is_absolute(){chosen}else{project.join(chosen)})
}
pub(super) fn unit_inactive(key:&str)->io::Result<()> {
    let unit=format!("waywr-{key}.service");
    let output=Command::new("systemctl").args(["--user","show","--property=ActiveState","--value",&unit]).output()?;
    if !output.status.success(){return Err(err("Cannot verify selected project service state"));}
    if !matches!(String::from_utf8_lossy(&output.stdout).trim(),"inactive"|"failed"){
        return Err(err("Selected project service is active or stopping; stop and drain it before release change"));
    }
    Ok(())
}
pub fn repair_answer_history(args:&[String])->io::Result<()> {
    let project=selected_project(args)?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    let state=state_path(&project,args)?;
    let _startup=FileLock::exclusive(&state.join("session/start.lock")).map_err(err)?;
    unit_inactive(key)?;
    let session=Session::open_existing(&project,&state,key,"history-maintenance").map_err(err)?;
    session.quiescent_for_release_change().map_err(err)?;host_attempts_quiescent(&state,key)?;
    let event:i64=value(args,"--event").ok_or_else(||err("Use --event SEQ"))?.parse().map_err(err)?;
    let request=value(args,"--request").ok_or_else(||err("Use --request ID"))?;
    let related=value(args,"--misrouted-events").unwrap_or_default().split(',').filter(|s|!s.is_empty()).map(str::parse::<i64>).collect::<Result<Vec<_>,_>>().map_err(err)?;
    waywr_backend::coordination::schema(&session).map_err(err)?;
    let out=waywr_backend::coordination::repair_history(&session,event,&request,&related).map_err(err)?;
    println!("{}",json!({"status":"history_corrected","request_id":out["request_id"],"source_event":event,"subject_id":out["subject_id"],"continuation_queued":false,"approval_replayed":false}));Ok(())
}
pub fn reconcile_request_routing(args:&[String])->io::Result<()> {
    let project=selected_project(args)?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    let state=state_path(&project,args)?;let _startup=FileLock::exclusive(&state.join("session/start.lock")).map_err(err)?;
    unit_inactive(key)?;let session=Session::open_existing(&project,&state,key,"routing-maintenance").map_err(err)?;
    session.quiescent_for_release_change().map_err(err)?;host_attempts_quiescent(&state,key)?;
    let request=value(args,"--request").ok_or_else(||err("Use --request ID"))?;let message=value(args,"--message").ok_or_else(||err("Use --message ID"))?;
    let out=waywr_backend::request_routing::reconcile(&session,&request,&message).map_err(err)?;println!("{}",out);Ok(())
}
// This is deliberately conservative. A release change never restores a prior
// snapshot; it only changes which compatible binary can read the current state.
// Every file (including receipts and SQLite/WAL) participates in rollback's
// boundary. A harmless write can therefore refuse rollback, which is safer
// than overlooking accepted work outside session_metadata.state_revision.
fn state_digest(state:&Path)->io::Result<String>{
    let mut files=Vec::new();
    fn collect(root:&Path,dir:&Path,files:&mut Vec<PathBuf>)->io::Result<()>{
        if !dir.exists(){return Ok(());}
        for entry in fs::read_dir(dir)?{
            let path=entry?.path();
            let kind=fs::symlink_metadata(&path)?.file_type();
            if kind.is_dir(){collect(root,&path,files)?;}
            else if kind.is_file(){files.push(path.strip_prefix(root).map_err(err)?.to_path_buf());}
            else if path==root.join("history")&&root.file_name().is_some_and(|n|n=="state")&&
                root.parent().is_some_and(|p|p.file_name().is_some_and(|n|n==".waywr")&&waywr_backend::project_layout::valid_alias(&path,&p.join("history"))){collect(root,&path,files)?;}
            else{return Err(err("State contains a link or special file; release change refused"));}
        }
        Ok(())
    }
    collect(state,state,&mut files)?;files.sort();
    let mut hash=Sha256::new();
    for relative in files{
        let bytes=relative.to_string_lossy();
        hash.update((bytes.len() as u64).to_le_bytes());hash.update(bytes.as_bytes());
        let mut file=fs::File::open(state.join(&relative))?;
        let mut buffer=[0u8;65536];
        loop {let count=file.read(&mut buffer)?;if count==0{break;}hash.update(&buffer[..count]);}
    }
    Ok(format!("{:x}",hash.finalize()))
}
pub(super) fn host_attempts_quiescent(state:&Path,key:&str)->io::Result<()> {
    fn visit(dir:&Path,state:&Path,key:&str)->io::Result<()>{
        if !dir.is_dir(){return Ok(());}
        for entry in fs::read_dir(dir)?{
            let path=entry?.path();
            let kind=fs::symlink_metadata(&path)?.file_type();
            if kind.is_dir(){visit(&path,state,key)?;continue;}
            if !kind.is_file(){return Err(err("Host evidence contains a link or special file"));}
            if path.file_name().is_some_and(|name|name=="intent.json") &&
                path.parent().and_then(Path::parent).is_some_and(|parent|parent.file_name().is_some_and(|name|name=="attempts")){
                let intent=read(&path).map_err(err)?;
                let unit=intent["unit"].as_str().ok_or_else(||err("Host attempt unit missing"))?;
                if intent["project_key"]!=key || !unit.starts_with(&format!("waywr-op-{key}-")) || !unit.ends_with(".service"){
                    return Err(err("Host attempt unit identity invalid"));
                }
                let attempt=path.parent().ok_or_else(||err("Host attempt path invalid"))?;
                let received=attempt.join("receipt.json").exists();
                let retired=attempt.join("retired.json").exists();
                if !received && !retired{
                    return Err(err(format!("Unresolved host attempt {}; release change refused",attempt.display())));
                }
                let receipt=if received{
                    let operation=attempt.parent().and_then(Path::parent).ok_or_else(||err("Host operation path invalid"))?;
                    let active=Attempt::load(state,operation).map_err(err)?;
                    if active.id!=intent["attempt_id"] || active.unit!=unit{
                        return Err(err("Completed host attempt is not the active operation identity"));
                    }
                    active.validated_receipt().map_err(err)?
                }else{None};
                let output=Command::new("systemctl").args(["--user","show",
                    "--property=Id,ActiveState,SubState,MainPID,ControlGroup,InvocationID,Restart,ExitType,RemainAfterExit,CollectMode",unit]).output()?;
                if !output.status.success(){return Err(err("Cannot verify exact host unit for release change"));}
                let text=String::from_utf8_lossy(&output.stdout);
                let properties=text.lines()
                    .filter_map(|line|line.split_once('=')).collect::<std::collections::HashMap<_,_>>();
                let get=|name|properties.get(name).copied().unwrap_or("");
                if get("Id")!=unit{return Err(err("Host unit identity differs from attempt"));}
                let empty=matches!(get("ActiveState"),"inactive"|"failed") ||
                    (get("ActiveState")=="active" && get("SubState")=="exited" &&
                     get("MainPID")=="0" && get("ControlGroup").is_empty() &&
                     get("Restart")=="no" && get("ExitType")=="cgroup" &&
                     get("RemainAfterExit")=="yes" && get("CollectMode")=="inactive" &&
                     receipt.as_ref().is_some_and(|value|value["invocation_id"]==get("InvocationID")));
                if !empty{
                    return Err(err(format!("Host attempt unit {unit} is active or unverified; release change refused")));
                }
            }
        }
        Ok(())
    }
    visit(&state.join("session/coordinator"),state,key)?;
    visit(&state.join("session/development"),state,key)?;
    visit(&state.join("runs"),state,key)
}
/// Stop only a positively quiescent selected service under execution fencing.
pub(super) fn stop_quiescent_project(project:&Path,state:&Path,key:&str)->io::Result<()> {
    let _execution=FileLock::exclusive(&state.join("supervision/fence.lock")).map_err(err)?;
    if state.join("state.sqlite").exists(){Session::open_existing(project,state,key,"upgrade-preflight").map_err(err)?.quiescent_for_release_change().map_err(err)?;host_attempts_quiescent(state,key)?;}
    let unit=format!("waywr-{key}.service");let status=Command::new("systemctl").args(["--user","show","--property=ActiveState","--value",&unit]).output()?;
    if !status.status.success(){return Err(err("Cannot establish selected service state; no stop attempted"))}
    let active=String::from_utf8_lossy(&status.stdout);if matches!(active.trim(),"active"|"activating"){let stop=Command::new("systemctl").args(["--user","stop",&unit]).output()?;if !stop.status.success(){return Err(err("Quiescent selected service did not stop; binding unchanged"))}}
    unit_inactive(key)
}
pub fn change(root:&Path,_manifest:&Value,args:&[String],action:&str)->io::Result<()> {
    let project=selected_project(args)?;
    let config:toml::Value=toml::from_str(&fs::read_to_string(project.join(".waywr/project.toml"))?).map_err(err)?;
    let key=config["project_key"].as_str().ok_or_else(||err("Project key missing"))?;
    let host=config["schema_version"].as_integer()==Some(2) &&
        config.get("execution").and_then(|value|value.get("host")).and_then(toml::Value::as_str)==Some("systemd-user-v1");
    if !host || key.len()!=32 || !key.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err(err("Unsupported project identity"));
    }
    let state=state_path(&project,args)?;
    let migration_journal=project.join(".waywr/provider-migration.json");
    if migration_journal.exists()&&read(&migration_journal).map_err(err)?["status"]=="prepared"{
        let _fence=FileLock::exclusive(&state.join("session/start.lock")).map_err(err)?;unit_inactive(key)?;
        // Recovery cannot rely on an old binary opening a half-published version boundary.
        for folder in [state.join("session/outbox"),state.join("session/ui-claims")]{
            if folder.is_dir(){for entry in fs::read_dir(&folder)?{let path=entry?.path();if path.extension().is_some_and(|x|x=="json")&&
                (folder.ends_with("outbox")||waywr_backend::execution::alive(&read(&path).map_err(err)?["process"])){return Err(err("Pending delivery or live UI prevents migration recovery"));}}}
        }
        host_attempts_quiescent(&state,key)?;
        waywr_backend::provider_selection::migrate_project(&project,&state,key).map_err(err)?;
    }
    let contract=read(&state.join("state-contract.json")).map_err(err)?;
    let schema=contract["state_schema"].as_u64().ok_or_else(||err("State schema missing"))?;
    if !matches!(schema,3|4|5|6|7|8|9) || contract["project_key"]!=key ||
        contract["execution_host"]!="systemd-user-v1"{
        return Err(err("Unsupported project state contract"));
    }
    fs::create_dir_all(state.join("session"))?;
    let _startup_fence=FileLock::exclusive(&state.join("session/start.lock")).map_err(err)?;
    if action=="upgrade"{stop_quiescent_project(&project,&state,key)?;}else{unit_inactive(key)?;}
    let binding=project.join(".waywr/release.json");
    let prior=if binding.exists(){Some(read(&binding).map_err(err)?)}else{None};
    if action=="bind" && prior.is_some(){return Err(err("Project already has a release binding"));}
    if action!="bind" && prior.is_none(){return Err(err("Project has no release binding"));}
    let revision=if state.join("state.sqlite").exists(){
        let revision=Session::open_existing(&project,&state,key,"maintenance").map_err(err)?.quiescent_for_release_change().map_err(err)?;
        host_attempts_quiescent(&state,key)?;
        revision
    }else{
        if read(&state.join("project-identity.json")).map_err(err)?!=
            json!({"schema_version":1,"project_key":key}) ||
            contract!=json!({"schema_version":1,"state_schema":schema,
                "execution_host":"systemd-user-v1","project_key":key}){
            return Err(err("Uninitialized state identity or host contract differs; release change refused"));
        }
        for entry in fs::read_dir(&state)?{
            let entry=entry?;
            let name=entry.file_name();
            if name=="project-identity.json" || name=="state-contract.json"{continue;}
            if name=="session" && entry.path().is_dir(){
                for child in fs::read_dir(entry.path())?{
                    let child=child?;
                    if child.file_name()!="start.lock" || !child.file_type()?.is_file() ||
                        child.metadata()?.len()!=0{
                        return Err(err("Uninitialized state has unrecognized records; release change refused"));
                    }
                }
                continue;
            }
            return Err(err("Uninitialized state has unrecognized records; release change refused"));
        }
        0
    };
    // Resume an interrupted, positively quiescent state8 -> state9 import through maintenance.
    let migration=project.join(".waywr/provider-migration.json");
    let imported8=migration.exists()&&read(&migration).map_err(err)?["original"]["contract"]["state_schema"]==8;
    let fingerprint=state_digest(&state)?;
    let target=value(args,"--release").ok_or_else(||err("Use --release ID"))?;
    if target.is_empty() || target.contains('/') || target.contains(".."){
        return Err(err("Invalid target release ID"));
    }
    let catalog=root.parent().ok_or_else(||err("Release catalog missing"))?;
    let destination=catalog.join(&target);
    let target_manifest=rust_bundle::validate(&destination)?;
    let interactive=if state.join("state.sqlite").exists(){waywr_backend::coordination::requires_contract(&Session::open_existing(&project,&state,key,"maintenance").map_err(err)?).map_err(err)?}else{false};
    if state.join("state.sqlite").exists()&&waywr_backend::request_routing::requires_contract(&Session::open_existing(&project,&state,key,"maintenance").map_err(err)?).map_err(err)?&&target_manifest["request_routing_contract"]!="pinned-roles-v1"{return Err(err("Saved request role choices require a release enforcing pinned-roles-v1; preserve them and choose a compatible release"));}
    if state.join("ollama-transport-v1.json").exists()&&target_manifest["provider_transport_contract"]!="ollama-http-v1"{return Err(err("Retained Ollama profiles/attempts need an Ollama HTTP compatible release"));}
    if interactive&&target_manifest["interaction_contract"]!="conversation-v1"{return Err(err("This project retains conversational coordination records; choose a compatible release, never discard or reinterpret them"));}
    if state.join("context-delivery-v1.json").exists()&&target_manifest["context_delivery_contract"]!="evidence-v1"{return Err(err("Retained read-only evidence continuations require evidence-v1; preserve their records and choose a compatible release"));}
    require_recovery_contract(&state,&target_manifest)?;
    if state.join("context-memory-v2.json").exists()&&target_manifest["context_memory_contract"]!="working-memory-v2"{return Err(err("Retained factual context and active evidence require working-memory-v2; preserve them and choose a compatible release"));}
    if project.join(".waywr-migration.json").exists()&&target_manifest["project_layout"]!="waywr-v1"{return Err(err("Migrated projects require a release supporting canonical .waywr layout"))}
    if target_manifest["release_id"]!=target{return Err(err("Target release identity mismatch"));}
    if target_manifest["state_schema_min"].as_u64().unwrap_or(u64::MAX)>schema ||
        target_manifest["state_schema_max"].as_u64().unwrap_or(0)<schema{
        return Err(err("Target release cannot read this project's state schema"));
    }
    if let Some(old)=&prior{
        if old["project_key"]!=key || old["schema_version"]!=1{return Err(err("Existing binding identity mismatch"));}
        if old["release_id"]==target{return Err(err("Project already uses that release"));}
        let source_id=old["release_id"].as_str().ok_or_else(||err("Old release ID missing"))?;
        let source=catalog.join(source_id);
        let source_manifest=rust_bundle::validate(&source)?;
        let source_hash=format!("{:x}",Sha256::digest(fs::read(source.join("manifest.json"))?));
        if old["manifest_sha256"]!=source_hash ||
            source_manifest["state_schema_min"].as_u64().unwrap_or(u64::MAX)>schema ||
            source_manifest["state_schema_max"].as_u64().unwrap_or(0)<schema && !(imported8&&schema==9&&source_manifest["state_schema_max"]==8&&action=="upgrade"){
            return Err(err("Source release or binding is incompatible with host state"));
        }
        if action=="rollback"{
            let history=project.join(".waywr/release-history");
            let expected=old["binding_generation"].as_u64().ok_or_else(||err("Binding generation missing"))?;
            let record=read(&history.join(format!("{expected}.json"))).map_err(err)?;
            if record["previous"]["release_id"]!=target || record["state_revision"]!=revision ||
                record["state_digest"]!=fingerprint{
                return Err(err("Rollback would cross new project work or a different release; refused"));
            }
        }
    }
    let digest=format!("{:x}",Sha256::digest(fs::read(destination.join("manifest.json"))?));
    let generation=prior.as_ref().and_then(|old|old["binding_generation"].as_u64()).unwrap_or(0)+1;
    let next=json!({"schema_version":1,"project_key":key,"release_id":target,
        "manifest_sha256":digest,"binding_generation":generation});
    let history=project.join(".waywr/release-history");
    fs::create_dir_all(&history)?;
    let record=json!({"schema_version":1,"action":action,"project_key":key,
        "state_revision":revision,"state_digest":fingerprint,"previous":prior,"next":next});
    let record_path=history.join(format!("{generation}.json"));
    if imported8&&record_path.exists(){
        let existing=read(&record_path).map_err(err)?;
        if existing["previous"]!=record["previous"]||existing["next"]!=record["next"]||existing["state_revision"]!=revision{return Err(err("Interrupted upgrade record differs; preserve and reconcile"));}
    }else{write(&record_path,&record,true).map_err(err)?;}
    if action=="upgrade"&&target_manifest["state_schema_max"].as_u64().unwrap_or(0)>=9{
        waywr_backend::provider_selection::migrate_project(&project,&state,key).map_err(err)?;
    }
    write(&binding,&next,false).map_err(err)?;
    println!("{}",json!({"status":"bound","project":project,"release_id":target,
        "binding_generation":generation,"state_revision":revision}));
    Ok(())
}

pub(super) fn require_recovery_contract(state:&Path,manifest:&Value)->io::Result<()>{
    if state.join("native-client-identity-v1.json").exists()&&manifest["native_client_identity_contract"]!="native-client-v1"{return Err(err("Frozen native client identities require native-client-v1; preserve attempts and use a compatible application"));}
    if state.join("coordinator-recovery-v1.json").exists()&&manifest["coordinator_recovery_contract"]!="recovery-v1"{return Err(err("Retained recovery/finding semantics require recovery-v1; choose a compatible release without discarding records"));}
    if state.join("ordinary-project-v1.json").exists()&&manifest["ordinary_project_contract"]!="ordinary-project-v1"{return Err(err("Saved baseline, candidate and retention semantics require an ordinary-project-v1 release"));}
    if state.join("development-convergence-v1.json").exists()&&manifest["development_convergence_contract"]!="convergence-v1"{return Err(err("Retained named task refs and coordinator retirement require convergence-v1; choose a compatible release"));}
    Ok(())
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn native_client_release_guard_rejects_an_app_that_ignores_frozen_clients(){
        let root=tempfile::tempdir().unwrap();
        fs::write(root.path().join("native-client-identity-v1.json"),b"{\"version\":1}").unwrap();
        assert!(require_recovery_contract(root.path(),&json!({})).is_err());
        require_recovery_contract(root.path(),&json!({"native_client_identity_contract":"native-client-v1"})).unwrap();
    }
    #[test] fn recovery_release_guard_preserves_records_and_requires_supported_semantics(){
        let d=tempfile::tempdir().unwrap();let state=d.path();
        require_recovery_contract(state,&json!({})).unwrap();
        let bytes=b"{\"version\":1,\"contract\":\"recovery-v1\"}";fs::write(state.join("coordinator-recovery-v1.json"),bytes).unwrap();
        assert!(require_recovery_contract(state,&json!({})).is_err());
        require_recovery_contract(state,&json!({"coordinator_recovery_contract":"recovery-v1"})).unwrap();assert_eq!(fs::read(state.join("coordinator-recovery-v1.json")).unwrap(),bytes);
    }
    #[test] fn convergence_release_guard_preserves_named_ref_semantics(){
        let d=tempfile::tempdir().unwrap();let state=d.path();let bytes=b"{\"version\":1,\"contract\":\"convergence-v1\"}";
        fs::write(state.join("development-convergence-v1.json"),bytes).unwrap();
        assert!(require_recovery_contract(state,&json!({"coordinator_recovery_contract":"recovery-v1"})).is_err());
        require_recovery_contract(state,&json!({"development_convergence_contract":"convergence-v1"})).unwrap();
        assert_eq!(fs::read(state.join("development-convergence-v1.json")).unwrap(),bytes);
    }
    #[test] fn migrated_history_alias_is_digestible_and_still_covers_history_bytes(){
        let d=tempfile::tempdir().unwrap();let root=d.path().join(".waywr");let state=root.join("state");fs::create_dir_all(&state).unwrap();fs::create_dir(root.join("history")).unwrap();fs::write(root.join("history/request.log"),"saved history").unwrap();std::os::unix::fs::symlink("../history",state.join("history")).unwrap();let before=state_digest(&state).unwrap();fs::write(root.join("history/request.log"),"new history").unwrap();assert_ne!(before,state_digest(&state).unwrap());std::os::unix::fs::symlink("../history",state.join("unexpected")).unwrap();assert!(state_digest(&state).is_err());
    }
    #[test] fn rollback_boundary_covers_files_outside_sqlite_revision() {
        let root=tempfile::tempdir().unwrap();
        let state=root.path();
        fs::create_dir_all(state.join("runs/run-1/operations/draft")).unwrap();
        fs::write(state.join("state.sqlite"),b"database bytes").unwrap();
        let before=state_digest(state).unwrap();
        fs::write(state.join("runs/run-1/operations/draft/receipt.json"),b"receipt").unwrap();
        assert_ne!(before,state_digest(state).unwrap());
        let after=state_digest(state).unwrap();
        fs::write(state.join("runs/run-1/operations/draft/receipt.json"),b"changed receipt").unwrap();
        assert_ne!(after,state_digest(state).unwrap());
        std::os::unix::fs::symlink("state.sqlite",state.join("link")).unwrap();
        assert!(state_digest(state).unwrap_err().to_string().contains("special file"));
    }
}
