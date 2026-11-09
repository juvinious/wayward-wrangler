//! Offline Linux Rust release assembly and installation. No Python is used.
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{env,fs,io,os::unix::fs::symlink,path::{Component,Path,PathBuf},process::Command};

fn err(message:impl std::fmt::Display)->io::Error{io::Error::other(message.to_string())}
fn value(args:&[String],flag:&str)->io::Result<String>{
    args.windows(2).find(|pair|pair[0]==flag).map(|pair|pair[1].clone())
        .ok_or_else(||err(format!("Missing {flag}")))
}
fn digest(path:&Path)->io::Result<String>{Ok(format!("{:x}",Sha256::digest(fs::read(path)?)))}
fn clean_id(id:&str)->io::Result<&str>{
    if id.is_empty() || id=="." || id==".." || id.starts_with('.') || id.len()>64 || !id.bytes().all(|byte|byte.is_ascii_alphanumeric() || byte==b'-' || byte==b'_' || byte==b'.'){
        return Err(err("Invalid release ID"));
    }
    Ok(id)
}
fn supported_schema_range(manifest:&Value)->bool{
    let Some(minimum)=manifest["state_schema_min"].as_u64() else{return false};
    let Some(maximum)=manifest["state_schema_max"].as_u64() else{return false};
    minimum>=3 && maximum<=9 && minimum<=maximum
}
fn copy(source:&Path,destination:&Path,files:&mut serde_json::Map<String,Value>,relative:&str)->io::Result<()>{
    let meta=fs::symlink_metadata(source)?;
    if !meta.file_type().is_file(){return Err(err(format!("Bundle source is not a regular file: {}",source.display())));}
    if let Some(parent)=destination.parent(){fs::create_dir_all(parent)?;}
    fs::copy(source,destination)?;
    files.insert(relative.into(),Value::String(digest(destination)?));
    Ok(())
}
fn regular_resources(source:&Path,destination:&Path,files:&mut serde_json::Map<String,Value>,prefix:&str)->io::Result<()> {
    let mut entries=fs::read_dir(source)?.map(|item|item.map(|e|e.path())).collect::<Result<Vec<_>,_>>()?;
    entries.sort();
    for item in entries{
        let name=item.file_name().ok_or_else(||err("Resource name missing"))?.to_string_lossy().into_owned();
        let relative=format!("{prefix}/{name}");
        let meta=fs::symlink_metadata(&item)?;
        if meta.file_type().is_dir(){regular_resources(&item,&destination.join(&name),files,&relative)?;}
        else if meta.file_type().is_file(){copy(&item,&destination.join(&name),files,&relative)?;}
        else{return Err(err("Resource symlink or special file refused"));}
    }
    Ok(())
}
pub fn build(args:&[String])->io::Result<()> {
    let id=value(args,"--release-id")?;clean_id(&id)?;
    let output=PathBuf::from(value(args,"--output")?);
    if output.exists(){return Err(err("Bundle destination already exists"));}
    let source=PathBuf::from(value(args,"--source-root")?).canonicalize()?;
    let frontend=PathBuf::from(value(args,"--frontend")?);
    let service=PathBuf::from(value(args,"--service")?);
    let runner=PathBuf::from(value(args,"--runner")?);
    fs::create_dir_all(&output)?;
    let outcome=(||->io::Result<Value>{
        let mut files=serde_json::Map::new();
        for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"] {
            copy(&source.join(name), &output.join(name), &mut files, name)?;
        }
        for (name,path) in [("waywr",frontend),("waywr-service",service),("waywr-runner",runner)]{
            copy(&path,&output.join("bin").join(name),&mut files,&format!("bin/{name}"))?;
        }
        copy(&source.join("frontend/resources/fake-workflow.toml"),
            &output.join("resources/fake-workflow.toml"),&mut files,"resources/fake-workflow.toml")?;
        regular_resources(&source.join("backend/resources"),&output.join("resources/backend"),
            &mut files,"resources/backend")?;
        for skill in ["draft.md","critique.md","revise.md"]{
            copy(&source.join("frontend/resources/skills").join(skill),
                &output.join("resources/skills").join(skill),&mut files,&format!("resources/skills/{skill}"))?;
        }
        let output_text=|program:&str,args:&[&str]|->Option<String>{Command::new(program).args(args).current_dir(&source).output().ok().filter(|o|o.status.success()).map(|o|String::from_utf8_lossy(&o.stdout).trim().to_owned())};
        let build=json!({"source_commit":output_text("git",&["rev-parse","HEAD"]),"source_dirty":output_text("git",&["status","--porcelain"]).map(|v|!v.is_empty()),"cargo_lock_sha256":digest(&source.join("Cargo.lock"))?,"toolchain_file_sha256":digest(&source.join("rust-toolchain.toml"))?,"rustc":output_text("rustc",&["--version","--verbose"]),"libc":output_text("getconf",&["GNU_LIBC_VERSION"]),"builder_image":env::var("WAYWR_BUILDER_IMAGE").ok(),"binary_reproducibility":"not claimed; normalized packaging is separate from bit-identical builds","published_binary_baseline":"Linux x86_64 GNU / Ubuntu 24.04 / glibc 2.39"});
        let manifest=json!({"build":build,"license":"MIT","notices":"THIRD_PARTY_NOTICES.md","schema_version":1,"release_id":id,"frontend":env!("CARGO_PKG_VERSION"),
            "backend":"0.1.0","backend_kind":"rust","protocol_major":1,
            "state_schema_min":3,"state_schema_max":9,"project_layout":"waywr-v1","interaction_contract":"conversation-v1","request_routing_contract":"pinned-roles-v1","context_delivery_contract":"evidence-v1","context_memory_contract":"working-memory-v2","coordinator_recovery_contract":"recovery-v1","development_convergence_contract":"convergence-v1","ordinary_project_contract":"ordinary-project-v1","provider_transport_contract":"ollama-http-v1","native_client_identity_contract":"native-client-v1",
            "platform":{"system":env::consts::OS,"machine":env::consts::ARCH},"files":files});
        fs::write(output.join("manifest.json"),serde_json::to_vec_pretty(&manifest)?
            .into_iter().chain([b'\n']).collect::<Vec<_>>())?;
        Ok(manifest)
    })();
    if outcome.is_err(){return outcome.map(|_|());}
    println!("{}",json!({"bundle":output,"release_id":id,
        "manifest_sha256":digest(&output.join("manifest.json"))?}));
    Ok(())
}
pub(crate) fn validate(bundle:&Path)->io::Result<Value>{
    let bundle=bundle.canonicalize()?;
    let manifest:Value=serde_json::from_slice(&fs::read(bundle.join("manifest.json"))?)?;
    if manifest["schema_version"]!=1 || manifest["backend_kind"]!="rust" || manifest["protocol_major"]!=1 ||
        manifest["frontend"]!=env!("CARGO_PKG_VERSION") || manifest["backend"]!="0.1.0" ||
        !supported_schema_range(&manifest) ||
        manifest["platform"]["system"]!=env::consts::OS || manifest["platform"]["machine"]!=env::consts::ARCH{
        return Err(err("Bundle platform, schema or paired binary version mismatch"));
    }
    clean_id(manifest["release_id"].as_str().ok_or_else(||err("Missing release ID"))?)?;
    let files=manifest["files"].as_object().ok_or_else(||err("Bundle file list missing"))?;
    // Old paired releases predate notice metadata and remain loadable. New
    // releases declaring it must inventory both texts under the integrity gate.
    if manifest.get("license").is_some() || manifest.get("notices").is_some() {
        if manifest["license"] != "MIT" || manifest["notices"] != "THIRD_PARTY_NOTICES.md"
            || !files.contains_key("LICENSE") || !files.contains_key("THIRD_PARTY_NOTICES.md") {
            return Err(err("Bundle license notices are missing or inconsistent"));
        }
    }
    for required in ["bin/waywr","bin/waywr-service","bin/waywr-runner",
        "resources/fake-workflow.toml",
        "resources/backend/prompt-contract-v1/context-findings-v2.txt",
        "resources/backend/prompt-contract-v1/context.txt",
        "resources/backend/prompt-contract-v1/legacy.txt",
        "resources/backend/prompt-contract-v1/product-spec-findings-v2-draft.txt",
        "resources/backend/prompt-contract-v1/product-spec-findings-v2-later.txt",
        "resources/backend/prompt-contract-v1/product-spec.txt",
        "resources/skills/draft.md","resources/skills/critique.md","resources/skills/revise.md"]{
        if !files.contains_key(required){return Err(err(format!("Bundle missing {required}")));}
    }
    for (name,expected) in files{
        let path=Path::new(name);
        if path.is_absolute() || path.components().any(|part|!matches!(part,Component::Normal(_))){
            return Err(err("Bundle file path escapes its release"));
        }
        let file=bundle.join(path).canonicalize()?;
        if !file.starts_with(&bundle) || !fs::symlink_metadata(&file)?.file_type().is_file() ||
            expected.as_str()!=Some(&digest(&file)?) {
            return Err(err(format!("Bundle integrity mismatch: {name}")));
        }
    }
    Ok(manifest)
}
#[cfg(test)] mod tests{
    use super::*;
    #[test] fn existing_schema_three_release_remains_compatible(){
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":3})));
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":4})));
        assert!(!supported_schema_range(&json!({"state_schema_min":4,"state_schema_max":3})));
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":6})));
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":6})));
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":7})));
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":8})));
        assert!(supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":9})));
        assert!(!supported_schema_range(&json!({"state_schema_min":3,"state_schema_max":10})));
    }
}
fn select_default(prefix:&Path,id:&str)->io::Result<()> {
    let user_alias=env::var_os("HOME").map(PathBuf::from).filter(|home|prefix==home.join(".local/share/wayward-wrangler")).map(|home|home.join(".local/bin/waywr"));
    if let Some(alias)=&user_alias{if fs::symlink_metadata(alias).is_ok(){let link=fs::read_link(alias).map_err(|_|err("Unrelated ~/.local/bin/waywr; default unchanged"))?;if link!=prefix.join("bin/waywr")&&link!=prefix.join("current/bin/waywr"){return Err(err("Unrelated ~/.local/bin/waywr alias; default unchanged"));}}}
    let bin=prefix.join("bin");fs::create_dir_all(&bin)?;
    let expected=Path::new("../current/bin/waywr");
    for name in ["waywr","wayward-wrangler","awp"]{
        let path=bin.join(name);
        if fs::symlink_metadata(&path).is_ok() && fs::read_link(&path).ok().as_deref()!=Some(expected){
            return Err(err(format!("Unrelated executable or alias at {}; default unchanged",path.display())));
        }
    }
    let current=prefix.join("current");
    if fs::symlink_metadata(&current).is_ok(){
        let old=fs::read_link(&current).map_err(|_|err("Default release path is not an owned link"))?;
        if !old.starts_with("releases") || old.components().count()!=2{return Err(err("Default release link is not owned"));}
    }
    let temporary=prefix.join(".current-next");
    if fs::symlink_metadata(&temporary).is_ok(){return Err(err("Pending default update exists"));}
    symlink(format!("releases/{id}"),&temporary)?;
    fs::rename(&temporary,&current)?;
    for name in ["waywr","wayward-wrangler","awp"]{
        let path=bin.join(name);
        if fs::symlink_metadata(&path).is_err(){symlink(expected,&path)?;}
    }
    if let Some(alias)=user_alias{fs::create_dir_all(alias.parent().unwrap())?;if fs::symlink_metadata(&alias).is_err(){symlink(prefix.join("bin/waywr"),alias)?;}}
    Ok(())
}
pub fn install(args:&[String])->io::Result<()> {
    let bundle=PathBuf::from(value(args,"--bundle")?).canonicalize()?;
    let prefix=PathBuf::from(value(args,"--prefix")?);
    if !prefix.is_absolute(){return Err(err("Installation prefix must be absolute"));}
    let manifest=validate(&bundle)?;
    let id=clean_id(manifest["release_id"].as_str().unwrap())?;
    let destination=prefix.join("releases").join(id);
    if destination.exists(){
        if digest(&destination.join("manifest.json"))?!=digest(&bundle.join("manifest.json"))?{
            return Err(err("Installed release ID has different contents"));
        }
        validate(&destination)?;
    }else{
        fs::create_dir_all(prefix.join("releases"))?;
        let staging=prefix.join("releases").join(format!(".install-{id}"));
        if staging.exists(){return Err(err("Incomplete staged installation exists; inspect it"));}
        fs::create_dir(&staging)?;
        for name in manifest["files"].as_object().ok_or_else(||err("Missing file list"))?.keys(){
            let source=bundle.join(name);let target=staging.join(name);
            fs::create_dir_all(target.parent().ok_or_else(||err("No target parent"))?)?;
            fs::copy(source,target)?;
        }
        fs::copy(bundle.join("manifest.json"),staging.join("manifest.json"))?;
        validate(&staging)?;
        let status=Command::new(staging.join("bin/waywr")).arg("--backend-info").status()?;
        if !status.success(){return Err(err("Staged Rust release failed paired backend validation"));}
        fs::rename(&staging,&destination)?;
    }
    if args.iter().any(|arg|arg=="--select-default"){select_default(&prefix,id)?;}
    println!("{}",json!({"release":destination,"release_id":id,
        "default_selected":args.iter().any(|arg|arg=="--select-default")}));
    Ok(())
}
