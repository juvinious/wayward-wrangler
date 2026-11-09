//! State8: one authorized parent, concurrent native child tasks, serialized checked Git delivery.
//! Models propose work; the existing writer, host permits, receipts and exact Git revisions authorize it.
use crate::development_recovery::{self as recovery, Cause};
use crate::{
    codec::{canonical_sha256, sha256},
    conversation_store::{commit, encode, event, prior},
    execution::{self, FileLock, now, read, write, write_bytes},
    host_attempt::Attempt,
    operation, request_history,
    session_service::Service,
    session_store::Session,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};
fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
fn strv<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key].as_str().ok_or_else(|| format!("Missing {key}"))
}
// A readable ref is separate from durable request/worktree identity. Titles may
// change; once saved, the namespace stays fixed. Never rename retained refs.
fn request_namespace(title: &str, work: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            if slug.len() >= 48 { break; }
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') && slug.len() < 48 {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    format!("waywr/{}-{}", if slug.is_empty() { "request" } else { slug }, &sha256(work.as_bytes())[..12])
}
fn valid_namespace(namespace: &str, work: &str) -> bool {
    if namespace == format!("waywr/{work}") { return true; } // immutable legacy packets
    let Some(slug) = namespace.strip_prefix("waywr/").and_then(|s|s.strip_suffix(&format!("-{}", &sha256(work.as_bytes())[..12]))) else { return false; };
    !slug.is_empty() && slug.len() <= 48 && !slug.starts_with('-') && !slug.ends_with('-')
        && slug.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
pub(crate) fn retain_convergence_contract(s:&Session)->Result<(),String>{
    write(&s.workflow.state.join("development-convergence-v1.json"),&json!({"version":1,"contract":"convergence-v1"}),true)
}
pub(crate) fn integration_checkout(s: &Session, w: &Value) -> Value {
    if !w["head"].is_string() { return Value::Null; }
    let mut out=json!({"commit":w["head"],"branch":w["result_branch"].as_str().or(w["integration_branch"].as_str()),
        "worktree_branch":w["integration_branch"],"directory":w["integration_worktree"],
        "primary_directory":s.project,"delivery_status":w["status"],
        "checks_passed_at_commit":w["checks"]["commit"]==w["head"] && w["checks"]["result"]["passed"]==true,
        "usage":"Combined development checkout. Until delivered, primary/main may contain older code. Run preview commands from directory, consult its README; keep preview data separate. This is not delivery or human acceptance."});
    if w["policy"]["project_workflow"].is_object(){out["target"]=w["main_branch"].clone();out["candidate"]=w["candidate"].clone();out["ownership"]=w["checkout_handoff"].clone();out["usage"]=json!("Use the presented candidate's project instructions. Check out candidate coordinates a clean ordinary checkout; validation owns governing writes until released. Delivered resources may be archived under refs/waywr/archive/REQUEST/governing. Suggestions are not receipts.");}out
}
/// Display projection only. Never feed this back into authorization or model packets.
/// Large immutable artifacts are retained verbatim in their source records/files.
fn development_display_record(kind: &str, record: &Value) -> Result<Value, String> {
    let mut display = record.clone();
    let Some(fields) = display.as_object_mut() else { return Err("Invalid development record".into()) };
    let mut omitted = vec![];
    for name in ["last_coordinator_result", "last_obsolete_result", "interrupted_coordinator",
        "workspace_recoveries", "recoveries", "checks", "review_diff", "review", "worker_report",
        "retained_input_snapshot", "continuation_snapshot", "receipt", "runtime_usage", "evidence",
        "instructions", "submission", "diff", "result", "evidence_context", "native_review", "progress", "recovery_boundary"] {
        if fields.remove(name).is_some() { omitted.push(name); }
    }
    if let Some(all) = record["obligations"].as_array() {
        let compact = all.iter().map(|item| {
            if let Some(text) = item.as_str() { return json!(text.chars().take(512).collect::<String>()); }
            let mut selected = serde_json::Map::new();
            for key in ["obligation_id", "id", "status", "kind", "unit_id", "description"] {
                if let Some(value) = item.get(key) {
                    selected.insert(key.into(), if let Some(text) = value.as_str() {
                        json!(text.chars().take(512).collect::<String>())
                    } else { value.clone() });
                }
            }
            Value::Object(selected)
        }).collect::<Vec<_>>();
        fields.insert("obligations".into(), json!(compact));
        omitted.push("obligations (details)");
    }
    if record["recovery"].is_object(){fields.insert("recovery".into(),recovery::summary(&record["recovery"]));}
    if record["progress"].is_object(){fields.insert("progress".into(),json!({"version":1,"current_sha256":record["progress"]["current"]["sha256"],"wait_state":record["progress"]["wait_state"],"reassessment_state":record["progress"]["reassessment_state"],"retained_recent_approaches":record["progress"]["recent"].as_array().map(Vec::len)}));}
    fields.insert("source_record".into(), json!({"kind":kind,"sha256":hash(record)?,
        "storage":"state.sqlite / development_records; operation evidence files remain unchanged",
        "omitted_fields":omitted}));
    Ok(display)
}
fn id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 80
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
}
fn workspace_snapshot(root: &Path) -> Result<Value, String> {
    let status = git(root, &["status", "--porcelain", "--untracked-files=all"])?;
    let diff = git(root, &["diff", "--binary", "HEAD"])?;
    let mut untracked = Vec::new();
    for path in git(root, &["ls-files", "--others", "--exclude-standard"])?.lines() {
        let file = root.join(path);
        if fs::symlink_metadata(&file)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Untracked symlink retained; reconcile capacity continuation".into());
        }
        untracked.push(json!({"path":path,"sha256":execution::hash_file(&file)?}));
    }
    Ok(
        json!({"head":git(root,&["rev-parse","HEAD"])?,"status":status,"diff_sha256":sha256(diff.as_bytes()),"untracked":untracked}),
    )
}
fn clean(root: &Path) -> Result<(), String> {
    if !git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        Err(format!(
            "Preserved dirty worktree {}; reconcile before integration",
            root.display()
        ))
    } else {
        Ok(())
    }
}
// Operational recovery retains exact cache bytes; it never edits application
// source or treats an arbitrary dirty worktree as disposable.
fn retain_python_caches(s: &Session, work: &Value, command: &str) -> Result<Option<Value>, String> {
    let tree = Path::new(strv(work, "integration_worktree")?);
    let directory = op_dir(s, strv(work, "work_id")?, &format!("cache-recovery-{}", &sha256(command.as_bytes())[..16]));
    let manifest_path = directory.join("manifest.json");
    let head = git(tree, &["rev-parse", "HEAD"])?;
    if head != work["head"] { return Err("Integration HEAD changed; preserve and inspect before recovery".into()); }
    let status = git(tree, &["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
    if status.is_empty() && !manifest_path.exists() { return Ok(None); }
    let mut files = Vec::new();
    for entry in status.split('\0').filter(|entry| !entry.is_empty()) {
        let path = entry.strip_prefix("?? ").ok_or("Tracked integration edits require explicit reconciliation; preserved")?;
        let relative = Path::new(path);
        if !relative.components().all(|c| matches!(c, Component::Normal(_))) {
            return Err("Unexpected integration path preserved; inspect before recovery".into());
        }
        let parent = relative.parent().ok_or("Cache parent missing")?;
        let name = relative.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let source_name = name.strip_suffix(".pyc").and_then(|s| s.split_once(".cpython-")).filter(|(_, tag)|
            !tag.is_empty() && tag.bytes().all(|b| b.is_ascii_digit() || b".opt-".contains(&b))).map(|(base, _)| format!("{base}.py"));
        let source = parent.parent().zip(source_name).map(|(parent, name)| parent.join(name));
        let Some(source) = source.filter(|_| parent.file_name().is_some_and(|s| s == "__pycache__")) else {
            return Err("Non-cache integration files require explicit reconciliation; all files preserved".into());
        };
        git(tree, &["ls-files", "--error-unmatch", "--", source.to_str().ok_or("Non-UTF8 source")?])?;
        let file = tree.join(relative);
        if !fs::symlink_metadata(&file).map_err(|e| e.to_string())?.file_type().is_file()
            || file.canonicalize().map_err(|e| e.to_string())? != tree.canonicalize().map_err(|e| e.to_string())?.join(relative) {
            return Err("Cache path is not an ordinary contained file; preserved".into());
        }
        files.push(json!({"path":path,"sha256":execution::hash_file(&file)?}));
    }
    let manifest = if manifest_path.exists() {
        let saved = read(&manifest_path)?;
        if saved["command_id"] != command || saved["head"] != head || saved["worktree"] != json!(tree) {
            return Err("Cache recovery identity changed; retained evidence requires inspection".into());
        }
        for file in &files {
            if !saved["files"].as_array().is_some_and(|all| all.contains(file)) {
                return Err("Integration files changed during recovery; preserved".into());
            }
        }
        saved
    } else {
        let saved = json!({"schema_version":1,"command_id":command,"head":head,"worktree":tree,
            "label":"Retained untracked Python caches; provenance not inferred","files":files});
        write(&manifest_path, &saved, true)?;
        saved
    };
    for file in manifest["files"].as_array().ok_or("Recovery manifest files missing")? {
        let relative = strv(file, "path")?;
        let path = Path::new(relative);
        if !path.components().all(|c| matches!(c, Component::Normal(_)))
            || !path.parent().and_then(Path::file_name).is_some_and(|name| name == "__pycache__")
            || !path.file_name().and_then(|s| s.to_str()).is_some_and(|name| name.contains(".cpython-") && name.ends_with(".pyc")) {
            return Err("Recovery manifest path leaves retained tree".into());
        }
        let original = tree.join(relative);
        let retained = directory.join("files").join(relative);
        if original.exists() {
            if !fs::symlink_metadata(&original).map_err(|e| e.to_string())?.file_type().is_file()
                || original.canonicalize().map_err(|e| e.to_string())? != tree.canonicalize().map_err(|e| e.to_string())?.join(relative) {
                return Err("Recovery cache path changed; preserved".into());
            }
            let bytes = fs::read(&original).map_err(|e| e.to_string())?;
            if sha256(&bytes) != file["sha256"] { return Err("Cache changed; preserve and inspect recovery".into()); }
            write_bytes(&retained, &bytes, true)?;
            if execution::hash_file(&original)? != file["sha256"] { return Err("Cache changed after retention; original preserved".into()); }
            fs::remove_file(&original).map_err(|e| e.to_string())?;
            fs::File::open(original.parent().ok_or("Missing cache parent")?).and_then(|dir| dir.sync_all()).map_err(|e| e.to_string())?;
        } else if execution::hash_file(&retained)? != file["sha256"] {
            return Err("Retained cache evidence differs; recovery stopped".into());
        }
    }
    clean(tree)?;
    Ok(Some(json!({"manifest":manifest_path,"files":manifest["files"],"head":head})))
}
pub fn find_codex() -> Option<PathBuf> {
    crate::provider_selection::find_client("codex")
}
pub fn profile(s: &Session) -> Result<Value, String> {
    if crate::provider_selection::enabled(s){return crate::provider_selection::effective(s);}
    Err("Legacy native development execution is retired; configure the shared model runtime".into())
}

pub fn preflight(p:&Value)->Result<(),String>{crate::model_runtime::preflight(p)}
pub fn policy(s: &Session) -> Result<Value, String> {
    let p = profile(s)?;
    let mut result =
        json!({"schema_version":1,"project":s.project,"commands":"Project-local file edits, formatting, builds, tests, documentation, local commits and service-owned Git worktree/integration operations","dependency_network":p["dependency_network"],"coordinator":{"provider":p["provider"],"model":p["model"],"effort":p["effort"]},"workers":{"provider":p["provider"],"model":p["model"],"effort":p["effort"],"concurrency":p["concurrency"]},"main_policy":p["main_policy"],"fallback":p["fallback"],"capacity_policy":"persist wait and retry with backoff; no silent provider substitution","limits":"No mandatory call, repair or task-time ceilings; usage observed, cost unknown unless reported","retention":"Retain task branches/worktrees, dirty input and immutable evidence","boundary":"Project-scoped trusted model runtime; worktrees are not an OS security boundary. No remote push/publication, unrelated project or shared provider configuration changes."});
    if crate::provider_selection::enabled(s){
        let (prefs,_)=crate::provider_selection::preferences(s)?;
        result["schema_version"]=json!(2);result["routing"]=prefs.allowed_policy()?;
        for role in crate::provider_selection::ROLES{let name=prefs.role_default(role);let profile=prefs.profile_value(&name)?;result["routing"]["defaults"][role]=json!({"profile_id":name,"profile_sha256":hash(&profile)?,"profile":profile});}
        // Saved defaults and the current binding can differ during an explicit handover.
        let bound=crate::provider_selection::current(s)?;let hash=bound["profile_sha256"].as_str().ok_or("Missing bound profile hash")?;
        result["routing"]["profiles"][hash]=json!({"name":bound["profile_id"],"profile":bound["profile"],"source":"current coordinator binding"});
        if !result["routing"]["roles"]["coordinator"].as_array().unwrap().contains(&json!(hash)){result["routing"]["roles"]["coordinator"].as_array_mut().unwrap().push(json!(hash));}
        result["fallback"]=json!(prefs.fallback);
        result["boundary"]=json!("Project-scoped trusted native clients; no remote publication, unrelated projects or shared provider changes. Worktrees are not a same-UID security boundary.");
    }
    if let Some(setup)=crate::project_workflow::policy(s)?{result["project_workflow"]=setup;result["retention"]=json!("Retire positively settled clean owned resources after dependencies/obligations clear; retain immutable evidence and Git objects");}
    if result["project_workflow"]["completion"]=="github-pr"{result["boundary"]=json!("Approved exact GitHub repository/remote/base and merge method only; normal checks/reviews, no force/admin bypass. No unrelated destinations or shared provider configuration changes.");}
    Ok(result)
}
fn row(tx: &Transaction<'_>, kind: &str, key: &str) -> Result<Option<Value>, String> {
    tx.query_row(
        "SELECT record_json FROM development_records WHERE kind=?1 AND id=?2",
        params![kind, key],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    .transpose()
}
fn save(tx: &Transaction<'_>, kind: &str, key: &str, value: &Value) -> Result<(), String> {
    tx.execute("INSERT INTO development_records(kind,id,work_id,record_json) VALUES(?1,?2,?3,?4) ON CONFLICT(kind,id) DO UPDATE SET record_json=excluded.record_json",params![kind,key,value["work_id"].as_str(),encode(value)?]).map_err(|e|e.to_string())?;
    Ok(())
}
fn invalidate_units(
    tx: &Transaction<'_>,
    key: &str,
    generation: u64,
    reason: &str,
) -> Result<(), String> {
    let mut q = tx
        .prepare("SELECT id,record_json FROM development_records WHERE kind='unit' AND work_id=?1")
        .map_err(|e| e.to_string())?;
    let units = q
        .query_map([key], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(q);
    for (id, raw) in units {
        let mut unit: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if unit["status"] == "running" {
            unit["supersede_pending"] = json!(true)
        } else if !matches!(
            unit["status"].as_str(),
            Some("integrated" | "accepted" | "reviewed" | "failed" | "cancelled")
        ) {
            unit["status"] = json!("superseded")
        };
        unit["steering_generation"] = json!(generation);
        unit["supersession_reason"] = json!(reason);
        save(tx, "unit", &id, &unit)?;
        event(tx, "development.requirement_delta", Some(key), &unit)?;
    }
    Ok(())
}
pub fn approve(
    tx: &Transaction<'_>,
    s: &Session,
    task: &str,
    current: &Value,
    auth: &Value,
) -> Result<(), String> {
    crate::project_workflow::require_ready(&crate::request_routing::policy(s,task)?)?;
    if crate::request_routing::policy(s,task)?["project_workflow"].is_object(){write(&s.workflow.state.join("ordinary-project-v1.json"),&json!({"version":1,"contract":"ordinary-project-v1"}),true)?;}
    let active:i64=tx.query_row("SELECT count(*) FROM development_records WHERE kind='workflow' AND json_extract(record_json,'$.status') NOT IN ('delivered','cancelled') AND id!=?1",[task],|r|r.get(0)).map_err(|e|e.to_string())?;
    if active > 0 {
        return Err("One approved project workflow is already active".into());
    }
    if let Some(mut work) = row(tx, "workflow", task)? {
        if crate::candidate::writes_held(&work){return Err("End candidate validation or supply candidate feedback before revising approved scope; checkout ownership and original input remain preserved".into())}
        if matches!(work["status"].as_str(), Some("delivered" | "cancelled")) {
            return Err("Start a new request for completed or cancelled work".into());
        }
        let generation = work["requirement_generation"].as_u64().unwrap_or(0) + 1;
        work["requirement_generation"] = json!(generation);
        work["scope_base"] = current["base"].clone();
        work["decision_revision"] = current["decision_revision"].clone();
        work["authorization"] = auth.clone();
        work["policy"] = crate::request_routing::policy(s,task)?;
        work["profile"] = profile(s)?;
        let human=work["policy"]["project_workflow"]["human_validation"]==true;
        if work["policy"]["project_workflow"].is_object(){let mut found=false;for o in work["obligations"].as_array_mut().into_iter().flatten(){if o["name"]=="policy:human-validation"{found=true;o["status"]=json!(if human{"open"}else{"resolved"});o["resolution"]=if human{Value::Null}else{json!({"source":"exact new scope/policy approval removed this validation requirement","authorization":auth})};}}if human&&!found{work["obligations"].as_array_mut().unwrap().push(json!({"name":"policy:human-validation","status":"open","owner":"human","resolution":null}));}}

        work["checks"] = Value::Null;
        work["summary"] = Value::Null;
        work["last_context_sha256"] = Value::Null;
        work["status"] = json!(if work["head"].is_null() {
            "initializing"
        } else {
            "running"
        });
        work["attention"] = Value::Null;
        bump(&mut work);
        invalidate_units(tx, task, generation, "new exact scope/policy authorization")?;
        save(tx, "workflow", task, &work)?;
        event(tx, "development.scope_reauthorized", Some(task), &work)?;
        return Ok(());
    }
    let request: Option<String> = tx
        .query_row(
            "SELECT request_id FROM request_history_requests WHERE task_id=?1",
            [task],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let mut w = json!({"schema_version":1,"work_id":task,"request_id":request,"status":"initializing","scope_base":current["base"],"decision_revision":current["decision_revision"],"authorization":auth,"policy":crate::request_routing::policy(s,task)?,"profile":profile(s)?,"revision":1,"requirement_generation":0,"steering":[],"coordinator_sequence":0,"pending":null,"summary":null,"checks":null,"obligations":[],"accepted":[],"attention":null});
    if w["policy"]["project_workflow"]["human_validation"]==true{w["obligations"]=json!([{"name":"policy:human-validation","status":"open","owner":"human","resolution":null}]);}
    save(tx, "workflow", task, &w)?;
    event(tx, "development.authorized", Some(task), &w)?;
    Ok(())
}
impl Session {
    pub fn development(&self) -> bool {
        read(&self.workflow.state.join("state-contract.json"))
            .ok()
            .is_some_and(|v| matches!(v["state_schema"].as_u64(),Some(8|9)) && v["project_key"] == self.key)
    }
    pub fn development_schema(&self) -> Result<(), String> {
        self.workflow.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS development_records(kind TEXT NOT NULL,id TEXT NOT NULL,work_id TEXT,record_json TEXT NOT NULL,PRIMARY KEY(kind,id)); CREATE UNIQUE INDEX IF NOT EXISTS development_one_parent ON development_records((1)) WHERE kind='workflow' AND json_extract(record_json,'$.status') NOT IN ('delivered','cancelled');") .map_err(|e|e.to_string())?;
        request_history::schema(self)?;
        if crate::provider_selection::enabled(self){crate::provider_selection::schema(self)?;crate::coordination::schema(self)?;}
        let p = profile(self)?;
        let c = self.workflow.connection()?;
        c.execute("UPDATE conversation_bindings SET provider=?1,model=?2,effort=?3 WHERE generation=1",params![p["provider"].as_str(),p["model"].as_str(),p["effort"].as_str()]).map_err(|e|e.to_string())?;
        c.execute("UPDATE conversation_budgets SET limit_count=9223372036854775807 WHERE bucket='conversation'",[]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn development_readiness(&self) -> Result<Value, String> {
        let p = match profile(self) {
            Ok(v) => v,
            Err(e) => return Ok(json!({"status":"blocked","reason":e})),
        };
        let pause = self.dialogue_state()?["pause"].clone();
        let active = self.chat_reserved()?.is_some();
        if crate::provider_selection::enabled(self){
            let (preferences,_)=crate::provider_selection::preferences(self)?;
            let missing=crate::model_runtime::preflight(&p).is_err();
            let inventory=preferences.profiles.iter().map(|(id,p)|json!({"profile_id":id,"provider":p.provider,"model":p.model,"effort":p.effort,"configured":true,"binary_present":Path::new(&p.binary).is_file()||(Path::new(&p.binary).file_name().is_some_and(|n|n==p.provider.as_str())&&crate::provider_selection::find_client(&p.provider).is_some()),"authentication":"unknown until an authorized call","role_usability":"unverified until ordinary use"})).collect::<Vec<_>>();
            return Ok(json!({"profile":p,"profile_sha256":hash(&p)?,"authorized":true,"status":if missing||pause["active"]==true&&pause["hard"]==true{"blocked"}else if active{"running"}else if pause["active"]==true{"recoverable"}else{"available"},"reason":if missing{json!("Shared model runtime is unavailable; configure its Node executable, runtime entry and provider in F3")}else{pause["reason"].clone()},"inventory":inventory,"usage":"No mandatory ceiling; authentication/capacity are not inferred from settings"}));
        }
        Ok(
            json!({"profile":p,"profile_sha256":hash(&p)?,"authorized":true,"status":if pause["active"]==true&&pause["hard"]==true{"blocked"}else if active{"running"}else if pause["active"]==true{"recoverable"}else{"available"},"reason":pause["reason"],"inventory":[{"provider":"codex","discovered":true,"configured":true,"usable":"checked locally at launch; authentication established by actual call","model":"gpt-6.1-sol","effort":"medium"}],"usage":"observed invocations; no mandatory ceiling; cost unknown until provider reports"}),
        )
    }
    fn development_reserve_coordinator(&self,work:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let work=work.clone();return writer.call(move|s|s.development_reserve_coordinator(&work));}
        let key=strv(work,"work_id")?;let mut next=work.clone();
        let actual=self.development_record("workflow",key)?.ok_or("Missing request")?;
        if actual["policy"]!=work["policy"]||actual["requirement_generation"]!=work["requirement_generation"]||!matches!(actual["status"].as_str(),Some("running"|"initializing")){return Ok(actual);}
        let (_,preferences_revision)=crate::provider_selection::preferences(self)?;
        let fenced=work["profile"]["selection"]["preferences_revision"]!=preferences_revision||crate::provider_selection::switch_pending(self)||crate::provider_selection::current(self)?["profile_sha256"]!=work["profile"]["selection"]["profile_sha256"];
        if fenced{next["pending"]=Value::Null;next["last_context_sha256"]=Value::Null;}
        self.development_save("workflow",key,&next,if fenced{"providers.reservation_fenced"}else{"development.coordinator_reserved"})?;Ok(next)
    }
    fn development_reserve_unit(&self,work:&Value,unit:&Value,event_kind:&str)->Result<bool,String>{
        if let Some(writer)=&self.workflow.writer{let(w,u,e)=(work.clone(),unit.clone(),event_kind.to_owned());return writer.call(move|s|s.development_reserve_unit(&w,&u,&e));}
        let mut c=self.workflow.connection()?;let tx=c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let id=strv(unit,"unit_id")?;let Some(mut current)=row(&tx,"unit",id)?else{return Ok(false)};
        if current["attempt_revision"]!=unit["attempt_revision"]||!matches!(current["status"].as_str(),Some("planned"|"ready")){return Ok(false);}
        let actual=row(&tx,"workflow",strv(work,"work_id")?)?.ok_or("Missing parent")?;
        let (_,revision)=crate::provider_selection::preferences(self)?;
        let stale=!matches!(actual["status"].as_str(),Some("running"|"initializing"))||unit["selection"]["preferences_revision"]!=revision||actual["policy"]!=work["policy"]||actual["requirement_generation"]!=work["requirement_generation"]||current["steering_generation"]!=unit["steering_generation"];
        if stale{
            current["attempt_revision"]=json!(current["attempt_revision"].as_u64().unwrap_or(1)+1);
            current["discarded_preparation"]=json!({"operation_id":unit["operation_id"],"reason":"Preferences or authority changed before reservation; no host launched"});
            save(&tx,"unit",id,&current)?;event(&tx,"providers.reservation_fenced",current["work_id"].as_str(),&current)?;
        }else{save(&tx,"unit",id,unit)?;event(&tx,event_kind,unit["work_id"].as_str(),unit)?;}
        tx.commit().map_err(|e|e.to_string())?;Ok(!stale)
    }
    pub fn development_record(&self, kind: &str, key: &str) -> Result<Option<Value>, String> {
        let mut c = self.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        row(&tx, kind, key)
    }
    pub fn development_records(&self, kind: &str) -> Result<Vec<Value>, String> {
        let c = self.workflow.connection()?;
        let mut st=c.prepare("SELECT record_json FROM development_records WHERE kind=?1 ORDER BY rowid").map_err(|e|e.to_string())?;
        st.query_map([kind], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .map(|r| {
                serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
            })
            .collect()
    }
    pub fn development_save(
        &self,
        kind: &str,
        key: &str,
        value: &Value,
        event_kind: &str,
    ) -> Result<(), String> {
        if let Some(w) = &self.workflow.writer {
            let (k, i, v, e) = (
                kind.to_owned(),
                key.to_owned(),
                value.clone(),
                event_kind.to_owned(),
            );
            return w.call(move |s| s.development_save(&k, &i, &v, &e));
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if row(&tx, kind, key)?.as_ref() == Some(value) {
            return Ok(());
        }
        if kind == "workflow"
            && row(&tx, kind, key)?
                .is_some_and(|v| v["requirement_generation"] != value["requirement_generation"])
        {
            return Err(
                "Development requirements changed during operation; saved state preserved".into(),
            );
        }
        if crate::provider_selection::enabled(self){
            if let Some(previous)=row(&tx,kind,key)?{
                if kind=="workflow"&&matches!(previous["status"].as_str(),Some("cancelling"|"cancelled"|"delivered"))&&previous["status"]!=value["status"]&&!(previous["status"]=="cancelling"&&value["status"]=="cancelled"){return Err("Terminal/cancelling request fences stale observation; preserve current state".into());}
                if kind=="workflow"&&previous["policy"]!=value["policy"]{return Err("Provider policy changed during observation; reload the current request without replay".into());}
                if kind=="unit"&&previous["attempt_revision"]!=value["attempt_revision"]&&event_kind!="development.capacity_resumed"&&!(event_kind=="development.evidence_retrieved"&&previous["status"]=="running"&&value["status"]=="ready"&&value["receipt"]["status"]=="completed"&&value["evidence_context"]["source_operation"]==previous["operation_id"]&&value["attempt_revision"].as_u64()==previous["attempt_revision"].as_u64().map(|n|n+1)){return Err("Task routing changed before reservation; reload future attempt".into());}
            }
        }
        if kind == "unit"
            && row(&tx, kind, key)?
                .is_some_and(|v| v["steering_generation"] != value["steering_generation"])
        {
            return Err("Unit steering changed during result observation; retry observation of the same receipt".into());
        }
        save(&tx, kind, key, value)?;
        if kind == "workflow" && value["status"] == "delivered" {
            tx.execute("UPDATE session_tasks SET status='approved',state_revision=state_revision+1 WHERE task_id=?1 AND status!='approved'",[key]).map_err(|e|e.to_string())?;
        }
        let mut public=value.clone();if let Some(o)=public.as_object_mut(){if let Some(context)=o.remove("evidence_context"){o.insert("evidence_context".into(),crate::development_context::audit(&context));}}
        let (seq, _) = event(&tx, event_kind, value["work_id"].as_str(), &public)?;
        if let Some(r) = value["request_id"].as_str() {
            request_history::tag(
                &tx,
                seq,
                r,
                if kind == "unit" {
                    "service / worker lifecycle"
                } else {
                    "service"
                },
            )?
        }
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn development_snapshot(&self) -> Result<Value, String> {
        let requests = request_history::requests(self)?;
        let all_units=self.development_records("unit")?;
        let project = |kind| -> Result<Vec<Value>, String> {
            let all=self.development_records(kind)?;
            all.iter().map(|record|{
                let mut display=development_display_record(kind,record)?;
                if kind=="workflow"{display["integration_checkout"]=integration_checkout(self,record);let active=recovery::active_recovery(record,&all_units);if active.is_object(){display["recovery"]=active;}}
                if kind=="unit"&&record["role"]=="review"{display["review_disposition"]=json!({"native_requires_changes":record["native_review"]["requires_changes"],"effective_requires_changes":crate::review_findings::effective_requires_changes(record,&all),"native_result_sha256":record["native_review_sha256"],"original_results":"retained immutable operation results"});}
                Ok(display)
            }).collect()
        };
        Ok(
            json!({"schema_version":1,"workflows":project("workflow")?,"units":project("unit")?,"effects":project("effect")?,"candidates":project("candidate")?,"cleanup":project("cleanup")?,"notifications":project("notification")?,"requests":requests,"context_diagnostics":crate::development_context::diagnostics(self)?,"history_root":crate::project_layout::history_root(&self.project,&self.workflow.state),"project_setup":crate::project_workflow::snapshot(self)?,"providers":if crate::provider_selection::enabled(self){crate::provider_selection::snapshot(self)?}else{Value::Null},"projection":"Display metadata only; full immutable evidence remains in development_records and operation files. Refreshed every scheduler tick"}),
        )
    }
    pub fn development_say(&self, work: &Value, text: &str) -> Result<(), String> {
        if let Some(w) = &self.workflow.writer {
            let (v, t) = (work.clone(), text.to_owned());
            return w.call(move |s| s.development_say(&v, &t));
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let msg = format!(
            "dev-{}",
            &hash(&json!([work["work_id"], work["revision"], text]))?[..24]
        );
        let exists: bool = tx
            .query_row(
                "SELECT exists(SELECT 1 FROM conversation_messages WHERE message_id=?1)",
                [&msg],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists {
            return Ok(());
        }
        let (seq, _) = event(
            &tx,
            "conversation.message",
            work["work_id"].as_str(),
            &json!({"message_id":msg,"role":"assistant","text":text,"label":"coordinator narrative"}),
        )?;
        request_history::tag(&tx, seq, strv(work, "request_id")?, "coordinator narrative")?;
        tx.execute(
            "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'assistant',?4,?5,?6,NULL)",
            params![
                msg,
                self.key,
                seq,
                text,
                sha256(text.as_bytes()),
                work["work_id"].as_str()
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    /// Compact application-owned context; never replays the readable log or raw tool streams.
    pub fn development_context(&self) -> Result<Value, String> {
        Ok(json!({"selection":"Compact project handoff; complete records remain in Inspect",
            "workflows":self.development_records("workflow")?.iter().rev().take(8).map(request_history::compact_workflow).collect::<Vec<_>>(),
            "units":self.development_records("unit")?.iter().rev().take(16).map(discussion_unit).collect::<Vec<_>>(),
            "knowledge":self.development_records("knowledge")?.iter().rev().take(4).map(discussion_knowledge).collect::<Vec<_>>(),
            "history":request_history::requests(self)?.iter().take(8).map(discussion_history).collect::<Vec<_>>()}))
    }
    pub(crate) fn development_discussion_inputs(&self,next:&Value)->Result<Value,String>{
        let project=planning_context(self)?;
        let task=next["task_id"].as_str();
        let work=task.map(|t|self.development_record("workflow",t)).transpose()?.flatten();
        let workflows=work.as_ref().map(|w|->Result<Value,String>{
            let mut card=request_history::compact_workflow(w);
            card["integration_checkout"]=integration_checkout(self,w);
            card["revision"]=w["revision"].clone();
            card["question"]=w["question"].clone();
            card["scope_sha256"]=w["scope_base"]["sha256"].clone();
            card["policy_sha256"]=json!(hash(&w["policy"])?);
            Ok(card)
        }).transpose()?.into_iter().collect::<Vec<_>>();
        let units=self.development_records("unit")?.iter().filter(|u|task.is_some_and(|t|u["work_id"]==t)).map(discussion_unit).collect::<Vec<_>>();
        let knowledge=self.development_records("knowledge")?.iter().rev().filter(|k|match task{Some(t)=>k["work_id"]==t,None=>project["head"].as_str().is_some_and(|h|k["source_commit"]==h)}).take(1).map(discussion_knowledge).collect::<Vec<_>>();
        let request:Option<String>=self.workflow.connection()?.query_row("SELECT record_json FROM request_history_requests WHERE subject_id=?1",[next["subject_id"].as_str()],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let history=request.map(|r|serde_json::from_str::<Value>(&r).map(|v|discussion_history(&v))).transpose().map_err(|e|e.to_string())?.into_iter().collect::<Vec<_>>();
        let routing=if crate::provider_selection::enabled(self){let (prefs,revision)=crate::provider_selection::preferences(self)?;json!({"revision":revision,"preferences":prefs.value(),"current_request_policy":work.as_ref().map(|w|w["policy"].clone()),"request_role_choices":crate::request_routing::subject_roles(self,next["subject_id"].as_str().unwrap_or(""))?,"authority":"Routing preferences are not scope or provider authority; approval is through Waywr"})}else{Value::Null};
        Ok(json!({"routing_preferences":routing,"project_context":project,"request_context":{"work_id":task,"selection":if task.is_some(){"Current scope only; facts are context, never authority"}else{"New topic; prior workflow plans/requirements excluded, current project source knowledge only"},"workflows":workflows,"units":units,"knowledge":knowledge,"history":history}}))
    }
    pub fn development_commit_decision(
        &self,
        work: &Value,
        records: &[(String, String, Value, String)],
        operation: &str,
    ) -> Result<(), String> {
        if let Some(writer) = &self.workflow.writer {
            let (w, r, o) = (work.clone(), records.to_vec(), operation.to_owned());
            return writer.call(move |s| s.development_commit_decision(&w, &r, &o));
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let key = strv(work, "work_id")?;
        let current = row(&tx, "workflow", key)?.ok_or("Workflow missing")?;
        if current["pending"]["operation_id"] != operation
            || current["requirement_generation"] != work["requirement_generation"]
        {
            return Err("Coordinator decision is stale; retain result without replay".into());
        }
        if crate::provider_selection::enabled(self){
            let switch=crate::provider_selection::switch(self)?;
            if crate::provider_selection::switch_pending(self)&&switch["mode"]=="interrupt"{
                let mut retained=current.clone();retained["pending"]=Value::Null;retained["last_context_sha256"]=Value::Null;
                retained["interrupted_coordinator"]=json!({"operation_id":operation,"result_adopted":false,"settled_receipt":work["last_coordinator_result"]["receipt"]});
                save(&tx,"workflow",key,&retained)?;event(&tx,"providers.late_decision_fenced",Some(key),&retained)?;tx.commit().map_err(|e|e.to_string())?;return Ok(());
            }
            if current["policy"]!=work["policy"]{return Err("Provider policy amended while decision settled; preserve receipt and review current state".into());}
        }
        for (kind, id, value, event_kind) in records {
            if let Some(previous)=row(&tx,kind,id)? {
                if kind!="unit" || event_kind!="development.assignment_retired" || value["status"]!="superseded"
                    || value["retirement"]["source_operation"]!=operation || value["retirement"]["assignment_sha256"]!=hash(&previous)?
                    || !matches!(previous["status"].as_str(),Some("planned"|"ready")) || previous["work_id"]!=work["work_id"] {
                    return Err("Decision target already exists or changed; preserve and reconcile".into());
                }
            } else if event_kind=="development.assignment_retired" {return Err("Retirement target disappeared; preserve and reconcile".into());}
            save(&tx, kind, id, value)?;
            let (seq,_)=event(&tx,event_kind,Some(key),value)?;
            if kind=="candidate"{let mut presented=value.clone();presented["event_seq"]=json!(seq);presented["at"]=json!(now());save(&tx,kind,id,&presented)?;}
        }
        save(&tx, "workflow", key, work)?;
        let mut public=work.clone();if let Some(o)=public.as_object_mut(){if let Some(context)=o.remove("evidence_context"){o.insert("evidence_context".into(),crate::development_context::audit(&context));}}
        if work["recovery"]!=current["recovery"]&&work["recovery"]["cause"]=="equivalent_work"{event(&tx,"development.reassessment",Some(key),&json!({"work_id":key,"request_id":work["request_id"],"recovery":work["recovery"],"progress_sha256":work["progress"]["current"]["sha256"]}))?;}
        event(&tx, "development.coordinator_decision", Some(key), &public)?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn development_command(&self, body: &Value) -> Result<Value, String> {
        if let Some(w) = &self.workflow.writer {
            let b = body.clone();
            return w.call(move |s| s.development_command(&b));
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(p) = prior(&tx, body)? {
            return Ok(p);
        }
        let key = strv(&body["payload"], "work_id")?;
        let mut work = row(&tx, "workflow", key)?.ok_or("Workflow not found")?;
        if body["action"]=="development.steer"&&crate::candidate::writes_held(&work){return Err("Use candidate feedback or end validation before changing requirements; the validation checkout is preserved".into())}
        if body["action"] == "development.reply" {
            let stable_question=crate::coordination::enabled(self)&&body["payload"]["coordination_version"]==1&&body["payload"]["question_sha256"]==hash(&work["question"])?;
            if work["status"] != "attention" || (!stable_question&&body["payload"]["revision"] != work["revision"]) || work["question"].is_null() {
                return Err("No current coordinator question at this exact revision".into());
            }
            let text=strv(&body["payload"],"text")?;
            if text.trim().is_empty() || text.len()>16000{return Err("Answer is empty or too large".into())}
            let answer=json!({"question":work["question"],"text":text,"source_command":body["command_id"],"scope_base":work["scope_base"],"policy_unchanged":true});
            if !work["answers"].is_array(){work["answers"]=json!([])}
            work["answers"].as_array_mut().unwrap().push(answer.clone());
            work["recovery"]=recovery::resolved(&work["recovery"],json!({"saved_answer":answer}));
            work["question"]=Value::Null;work["status"]=json!("running");work["attention"]=Value::Null;work["last_context_sha256"]=Value::Null;
            bump(&mut work);save(&tx,"workflow",key,&work)?;
            let (seq,_)=event(&tx,"development.answer",Some(key),&answer)?;
            request_history::tag(&tx,seq,strv(&work,"request_id")?,"user")?;
            let out=json!({"status":"accepted","work_id":key,"task_status":"running","scope_unchanged":true});
            commit(&tx,body,&out)?;tx.commit().map_err(|e|e.to_string())?;return Ok(out);
        } else if body["action"] == "development.steer" {
            if let Some(source)=body["payload"].get("coordination_source"){
                if source["policy_sha256"]!=hash(&work["policy"])?||source["scope_sha256"]!=work["scope_base"]["sha256"]{return Err("Conversational steering scope or policy changed; preserve the original intent".into())}
            }
            if body["payload"]["requirement_generation"] != work["requirement_generation"]
                || body["payload"]["in_scope"] != true
            {
                return Err("Steering base changed or in-scope declaration missing".into());
            }
            if work["status"] == "delivered" || work["status"] == "cancelled" {
                return Err("Start a new request for delivered work".into());
            }
            let text = strv(&body["payload"], "text")?;
            if text.trim().is_empty() || text.len() > 16000 {
                return Err("Steering text is empty or too large".into());
            }
            let generation = work["requirement_generation"].as_u64().unwrap_or(0) + 1;
            let msg = format!(
                "steer-{}",
                &sha256(strv(body, "command_id")?.as_bytes())[..24]
            );
            let delta = json!({"generation":generation,"source_command":body["command_id"],"source_message":msg,"text":text,"scope_base":work["scope_base"],"policy_unchanged":true});
            if !work["steering"].is_array() {
                work["steering"] = json!([])
            }
            work["steering"].as_array_mut().unwrap().push(delta.clone());
            work["requirement_generation"] = json!(generation);
            work["checks"] = Value::Null;
            work["summary"] = Value::Null;
            work["last_context_sha256"] = Value::Null;
            work["status"] = json!("running");
            work["attention"] = json!(
                "In-scope steering saved; obsolete running attempts may finish checkpoints, adoption is fenced"
            );
            bump(&mut work);
            // Conservatively invalidate unfinished work transitively; preserve integrated evidence for adaptation.
            let mut q=tx.prepare("SELECT id,record_json FROM development_records WHERE kind='unit' AND work_id=?1").map_err(|e|e.to_string())?;
            let all = q
                .query_map([key], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            drop(q);
            for (uid, raw) in all {
                let mut u: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                if u["status"] == "running" {
                    u["supersede_pending"] = json!(true);
                } else if !matches!(
                    u["status"].as_str(),
                    Some("integrated" | "accepted" | "reviewed" | "failed" | "cancelled")
                ) {
                    u["status"] = json!("superseded");
                }
                u["steering_generation"] = json!(generation);
                save(&tx, "unit", &uid, &u)?;
                event(&tx, "development.requirement_delta", Some(key), &u)?;
            }
            save(&tx, "workflow", key, &work)?;
            let (seq, _) = event(
                &tx,
                "conversation.message",
                Some(key),
                &json!({"message_id":msg,"role":"user","text":text,"kind":"explicit in-scope steering","requirement_generation":generation}),
            )?;
            request_history::tag(&tx, seq, strv(&work, "request_id")?, "user")?;
            tx.execute(
                "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'user',?4,?5,?6,NULL)",
                params![msg, self.key, seq, text, sha256(text.as_bytes()), key],
            )
            .map_err(|e| e.to_string())?;
            event(&tx, "development.steering", Some(key), &delta)?;
            let subject: Option<String> = tx
                .query_row(
                    "SELECT subject_id FROM request_history_requests WHERE task_id=?1",
                    [key],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let out = json!({"status":"accepted","message_id":msg,"subject_id":subject,"steering_generation":generation});
            commit(&tx, body, &out)?;
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(out);
        }
        if body["payload"]["revision"] != work["revision"] {
            return Err("Workflow revision changed".into());
        }
        if body["action"] == "development.cancel" {
            if matches!(work["status"].as_str(), Some("delivered" | "cancelled")) {
                return Err("Workflow already ended".into());
            }
            work["status"] = json!("cancelling");
            work["cancel_command_id"] = body["command_id"].clone();
            work["summary"] = Value::Null;
            work["checks"] = Value::Null;
            bump(&mut work);
            save(&tx, "workflow", key, &work)?;
            event(&tx, "development.cancel_requested", Some(key), &work)?;
            let out = json!({"status":"accepted","work_id":key,"task_status":"cancelling"});
            commit(&tx, body, &out)?;
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(out);
        }

        if work["status"] != "attention" {
            return Err("Workflow is not awaiting a decision".into());
        }
        if !work["question"].is_null() { return Err("Answer the saved coordinator question via /reply before resuming development".into()); }
        if body["action"] != "development.resume" { return Err("Unsupported development recovery action".into()); }
        if !work["pending"].is_null() { return Err("Pending execution must settle before operational recovery".into()); }
        if !authority(self, &work)? { return Err("Current scope or operating policy differs; recovery cannot grant new authority".into()); }
        let _git = FileLock::exclusive(&self.workflow.state.join("development-git.lock"))?;
        if let Some(recovery) = retain_python_caches(self, &work, strv(body, "command_id")?)? {
            if !work["workspace_recoveries"].is_array() { work["workspace_recoveries"] = json!([]); }
            work["workspace_recoveries"].as_array_mut().unwrap().push(recovery.clone());
            let (seq, _) = event(&tx, "development.workspace_reconciled", Some(key), &recovery)?;
            request_history::tag(&tx, seq, strv(&work, "request_id")?, "service")?;
        }
        if let Some(note) = body["payload"].get("recovery_note") {
            let note = note.as_str().ok_or("Recovery note must be text")?;
            if note.trim().is_empty() || note.len() > 16000 {
                return Err("Recovery note must contain 1..16000 bytes".into());
            }
            if !work["recoveries"].is_array() {
                work["recoveries"] = json!([]);
            }
            let entry = json!({"source_command":body["command_id"],
                "at":request_history::timestamp(now()),"note":note,
                "label":"operator-reported recovery context; not scope or execution authority"});
            work["recoveries"]
                .as_array_mut()
                .unwrap()
                .push(entry.clone());
            let (seq, _) = event(&tx, "development.recovery_context", Some(key), &entry)?;
            request_history::tag(&tx, seq, strv(&work, "request_id")?, "operator")?;
            // A repaired environment is relevant new input even when the
            // source/requirements and prior worker failures are unchanged.
            work["last_context_sha256"] = Value::Null;
        }
        work["status"] = json!("running");
        work["attention"] = Value::Null;
        work["last_context_sha256"] = Value::Null;
        work["revision"] = json!(work["revision"].as_u64().unwrap() + 1);
        save(&tx, "workflow", key, &work)?;
        event(&tx, "development.resumed", Some(key), &work)?;
        let out = json!({"status":"accepted"});
        commit(&tx, body, &out)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(out)
    }
}
// Structured provider schema is separate from authority. Everything is validated again on adoption.
fn obj(p: Value) -> Value {
    let keys = p.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    json!({"type":"object","properties":p,"required":keys,"additionalProperties":false})
}
fn string() -> Value {
    json!({"type":"string"})
}
fn strings() -> Value {
    json!({"type":"array","items":{"type":"string"}})
}
fn implementation_role()->String{"implementation".into()}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskSpec {
    #[serde(default="implementation_role")] role:String,
    #[serde(default)] profile:String,
    #[serde(default)] routing_reason:String,
    #[serde(default)] review_targets:Vec<String>,
    #[serde(default)] resolution_targets:Vec<String>,
    id: String,
    title: String,
    instructions: String,
    paths: Vec<String>,
    start_after: Vec<String>,
    accept_after: Vec<String>,
    obligations: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    schema_version: u64,
    assistant_text: String,
    action: String,
    tasks: Vec<TaskSpec>,
    units: Vec<String>,
    checks: Vec<Vec<String>>,
    resolved_obligations: Vec<String>,
    reason: String,
    #[serde(default)] evidence_requests: Vec<crate::development_context::Request>,
    #[serde(default)] context_summary: String,
    #[serde(default)] release_evidence: Vec<String>,
    #[serde(default)] delivery:Value,
}
pub fn result_schema(packet: &Value) -> Value {
    if packet["operation_kind"] == "conversation" {
        let mut schema=if packet["selected_context"]["interaction"]["version"]==1{crate::coordination::result_schema()}else{crate::continuous_adapter::schema()};
        if packet["selected_context"]["interaction"]["version"]==1 && packet["selected_context"]["evidence_allowlist"].as_array().is_some_and(|refs|!refs.is_empty()){
            schema["properties"]["evidence_refs"]["items"]=json!({"type":"string","enum":packet["selected_context"]["evidence_allowlist"]});
        }
        if packet["evidence_index"]["version"]==1{schema["properties"]["action"]["anyOf"].as_array_mut().unwrap().push(json!({"type":"object","properties":{"kind":{"type":"string","const":"retrieve_evidence"},"payload":{"type":"object","properties":{"requests":crate::development_context::request_schema()},"required":["requests"],"additionalProperties":false}},"required":["kind","payload"],"additionalProperties":false}));}return schema;
    }
    if packet["operation_kind"]=="development-review"{
        let mut schema=obj(json!({"schema_version":{"type":"integer","enum":[1]},"summary":string(),"reviewed_commits":strings(),"findings":strings(),"requires_changes":{"type":"boolean"}}));
        if packet["evidence_index"]["version"]==1{schema["properties"]["evidence_requests"]=crate::development_context::request_schema();schema["required"].as_array_mut().unwrap().push(json!("evidence_requests"));}
        if packet["finding_contract"]==1{schema["properties"]["finding_dispositions"]=crate::review_findings::schema();schema["required"].as_array_mut().unwrap().push(json!("finding_dispositions"));}
        if packet["context_delivery"]["memory_version"]==2{for key in ["context_summary","release_evidence"]{schema["properties"][key]=if key=="context_summary"{string()}else{strings()};schema["required"].as_array_mut().unwrap().push(json!(key));}}return schema;
    }
    if packet["operation_kind"] == "development-worker" {
        return obj(
            json!({"schema_version":{"type":"integer","enum":[1]},"summary":string(),"commit":string(),"checks":strings(),"remaining_obligations":strings()}),
        );
    }
    let mut schema=obj(
        json!({"schema_version":{"type":"integer","enum":[1]},"assistant_text":string(),"action":{"type":"string","enum":["assign","integrate","check","complete","wait","decision"]},"tasks":{"type":"array","items":obj(json!({"id":string(),"title":string(),"instructions":string(),"paths":strings(),"start_after":strings(),"accept_after":strings(),"obligations":strings()}))},"units":strings(),"checks":{"type":"array","items":strings()},"resolved_obligations":strings(),"reason":string()}),
    );
    if packet["execution"]["profile"]=="development-profiles-v1"{
        let props=&mut schema["properties"]["tasks"]["items"]["properties"];
        props["role"]=json!({"type":"string","enum":["implementation","review"]});
        props["profile"]=string();props["routing_reason"]=string();props["review_targets"]=strings();
        for field in ["role","profile","routing_reason","review_targets"]{schema["properties"]["tasks"]["items"]["required"].as_array_mut().unwrap().push(json!(field));}
    }
    if packet["evidence_index"]["version"]==1{
        schema["properties"]["action"]["enum"].as_array_mut().unwrap().push(json!("retrieve"));
        schema["properties"]["evidence_requests"]=crate::development_context::request_schema();schema["required"].as_array_mut().unwrap().push(json!("evidence_requests"));
    }
    if packet["ordinary_project_contract"]==1{schema["properties"]["action"]["enum"].as_array_mut().unwrap().extend([json!("candidate"),json!("refresh_target")]);schema["properties"]["delivery"]=crate::candidate::delivery_schema();schema["required"].as_array_mut().unwrap().push(json!("delivery"));}
    if packet["convergence_contract"]==1{schema["properties"]["action"]["enum"].as_array_mut().unwrap().push(json!("retire"));}
    if packet["recovery_contract"]==1{schema["properties"]["tasks"]["items"]["properties"]["resolution_targets"]=strings();schema["properties"]["tasks"]["items"]["required"].as_array_mut().unwrap().push(json!("resolution_targets"));}
    if packet["context_delivery"]["memory_version"]==2{for key in ["context_summary","release_evidence"]{schema["properties"][key]=if key=="context_summary"{string()}else{strings()};schema["required"].as_array_mut().unwrap().push(json!(key));}}
    schema
}
pub fn validate_result(v: &Value, p: &Value) -> Result<(), String> {
    if serde_json::to_vec(v).map_err(|e| e.to_string())?.len() > 65536 {
        return Err("Development result exceeds 64KiB".into());
    }
    if p["operation_kind"]=="development-review"{
        let expected=p["review_targets"].as_array().ok_or("Review lacks frozen targets")?.iter().map(|t|t["result_commit"].clone()).collect::<Vec<_>>();
        let retrieval=p["evidence_index"]["version"]==1;
        let requests=if retrieval{crate::development_context::validate_requests(&v["evidence_requests"])?}else{vec![]};
        if p["context_delivery"]["memory_version"]==2{crate::development_context::validate_memory(v)?;}
        if v.as_object().is_none_or(|o|o.len()!=(if p["context_delivery"]["memory_version"]==2{8}else if retrieval{6}else{5})+if p["finding_contract"]==1{1}else{0})||v["schema_version"]!=1||v["summary"].as_str().is_none()||v["reviewed_commits"]!=if requests.is_empty()&&p["review_resolution_only"]!=true{json!(expected)}else{json!([])}||v["findings"].as_array().is_none_or(|a|a.iter().any(|x|!x.is_string()))||!v["requires_changes"].is_boolean(){return Err("Review must identify exactly the frozen commits and typed findings; retrieval is not a completed review".into());}
        if p["finding_contract"]==1{crate::review_findings::validate(v,p)?;}
        if retrieval&&requests.is_empty()&&p["review_resolution_only"]!=true&&p["review_targets"].as_array().unwrap().iter().any(|u|!crate::development_context::diff_delivered(p,u)){return Err("Full target diff not delivered; request remaining evidence pages before final review".into());}
        return Ok(());
    }
    if p["operation_kind"] == "development-worker" {
        if v["schema_version"] != 1
            || v["summary"].as_str().is_none()
            || !v["commit"].as_str().is_some_and(|s| {
                s.is_empty() || (s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            })
            || !["checks", "remaining_obligations"].iter().all(|key| {
                v[*key]
                    .as_array()
                    .is_some_and(|a| a.iter().all(Value::is_string))
            })
        {
            return Err("Invalid worker result: expected summary, exact commit (or empty for a blocked report), checks and remaining_obligations".into());
        }
        return Ok(());
    }
    if p["operation_kind"] == "development-check" {
        return if v["checks"].is_array() && v["passed"].is_boolean() {
            Ok(())
        } else {
            Err("Invalid check result".into())
        };
    }
    let d: Decision = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    if d.schema_version != 1
        || !matches!(
            d.action.as_str(),
            "assign" | "integrate" | "check" | "complete" | "wait" | "decision" | "retrieve" | "retire" | "candidate" | "refresh_target"
        )
        || d.tasks.len() > 16
        || d.assistant_text.len() > 32768
    {
        return Err("Invalid coordinator decision".into());
    }
    if matches!(d.action.as_str(),"candidate"|"refresh_target")&&p["ordinary_project_contract"]!=1{return Err("Ordinary candidate actions unavailable in this frozen packet".into())}
    if d.action=="retire"&&(p["convergence_contract"]!=1||d.units.is_empty()||!d.tasks.is_empty()||!d.checks.is_empty()||!d.resolved_obligations.is_empty()){return Err("Retirement requires its frozen contract and named units without other execution actions".into());}
    if d.action=="retrieve"&&(p["evidence_index"]["version"]!=1||d.evidence_requests.is_empty()||!d.tasks.is_empty()||!d.units.is_empty()||!d.checks.is_empty()||!d.resolved_obligations.is_empty()){return Err("Retrieval cannot carry execution actions".into());}
    if d.action!="retrieve"&&!d.evidence_requests.is_empty(){return Err("Evidence retrieval must be a separate read-only decision before execution".into());}
    if p["context_delivery"]["memory_version"]==2{crate::development_context::validate_memory(v)?;}
    if p["evidence_index"]["version"]==1{crate::development_context::validate_requests(&v["evidence_requests"])?;}else if !d.evidence_requests.is_empty(){return Err("Packet grants no evidence retrieval".into());}
    Ok(())
}
pub(crate) fn worker_write_roots(packet:&Value)->Result<Vec<PathBuf>,String>{
    let tree = PathBuf::from(strv(packet, "worktree")?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let common = PathBuf::from(strv(packet, "git_common_dir")?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let work = strv(packet, "run_id")?;
    let name = strv(&packet["assignment"], "name")?;
    if !id(work) || !id(name) {
        return Err("Invalid worker Git permission identity".into());
    }
    let namespace = packet["assignment"]["branch_namespace"].as_str().map(str::to_owned).unwrap_or_else(||format!("waywr/{work}"));
    if !valid_namespace(&namespace, work) { return Err("Worker request-ref namespace differs from its frozen identity".into()); }
    let branch = format!("{namespace}/tasks/{name}/work");
    if packet["assignment"]["branch"] != branch
        || git(&tree, &["symbolic-ref", "--short", "HEAD"])? != branch
    {
        return Err("Worker branch lacks isolated task-ref namespace; preserve old work and use a follow-up assignment".into());
    }
    if PathBuf::from(git(
        &tree,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?) != common
    {
        return Err("Worker Git common directory differs from frozen packet".into());
    }
    let metadata = PathBuf::from(git(
        &tree,
        &["rev-parse", "--path-format=absolute", "--git-dir"],
    )?);
    if !metadata.starts_with(common.join("worktrees")) {
        return Err("Worker must use its own linked-worktree Git metadata".into());
    }
    let roots = vec![
        common.join("objects"),
        metadata,
        common
            .join("refs/heads")
            .join(format!("{namespace}/tasks/{name}")),
    ];
    let logs = common
        .join("logs/refs/heads")
        .join(format!("{namespace}/tasks/{name}"));
    let mut roots=roots;
    if logs.exists(){roots.push(logs);}
    for path in &roots {
        let actual=path.canonicalize().map_err(|e|e.to_string())?;
        if actual!=*path||!actual.starts_with(&common)||!actual.is_dir(){return Err("Git permission root changed; preserve and reconcile".into());}
    }
    Ok(roots)
}
const COORDINATOR: &str = r#"You are the selected live development coordinator for this project request. Derive your plan, task decomposition, dependencies, verification commands and follow-ups from the saved human conversation, approved scope, project context and observed results. No application-specific sequence is prescribed. You select and review work; assigned native workers produce all application code. Never run tools yourself. Return typed decision JSON, all fields required, unused arrays empty. Actions: assign tasks, integrate exact frozen submitted units after reviewing their diff and claims, check with project-appropriate argv commands, complete with outcomes and limitations after successful exact combined checks and no open obligations, wait only while dependent/running work exists, or decision for a substantive human question. Expose your intended decomposition in assistant_text before dispatch and explain later adaptations. Use independent concurrent tasks where useful; define start_after separately from accept_after. Decide whether a shared contract or preparatory task is needed from the actual project. Track significant fixtures/placeholders and real-production verification as explicit obligations. paths restrict file/directory prefixes; give shared files a clear owner. Workers own their task branches; only the service mutates integration/main refs. A submission is not delivery. Review actual worker results, diffs, failures, dependency updates, checks and human steering in every supplied packet. Assign repairs or follow-up work as needed rather than following a fixed sequence. Select meaningful checks required by the approved scope and actual project/toolchain. The check executor supports Cargo commands, tracked shell/Python scripts and worktree executables; a worker can provide a tracked check script for other toolchains. Never substitute a report for execution evidence. Operational recovery notes are labelled context, not new authority. After a concrete repair, adapt using that information; preserve old failures and dirty input. Human answers are in workflow.answers; in-scope deltas are in workflow.steering. Boundary expansion needs exact scope/policy approval, never prose. Resolve only obligations proved by integrated code and actual checks. No remote push/publication, shared provider changes or unrelated project access. Wait on capacity under configured policy; no mandatory task time/call/repair ceilings. Closeout distinguishes observed results, claims and limitations, records the actual request outcomes and links its history. When routing_preferences is present, each task also has role (implementation or review), profile (an allowed named profile, or empty to inherit), routing_reason and review_targets (saved task names; empty for implementation). Review tasks have paths empty and inspect frozen commits without tools or commits. Use independent review where justified; concrete findings require implementation follow-ups. Select profiles only inside the approved policy and task override. Output no hidden reasoning."#;
const WORKER: &str = r#"You are a Wayward Wrangler launched development worker, authorized only for the assigned task and project worktree. Produce the application code yourself. Inspect the frozen assignment and relevant local files; implement, test, format, and make a local commit on your assigned branch. Use the project toolchain and applicable project instructions. For Rust use permanent ~/.cargo/bin rustup, no CARGO_HOME/RUSTUP_HOME overrides. Dependency downloads require the frozen policy grant. Work only inside your assigned worktree and paths. Never modify another worktree, integration/main refs, Waywr state, shared provider config/credentials or another project. No push/publication. Preserve unexpected files. Do not run waywr init or launch another agent. No subagents. Report exact commit and actual checks (claims are reviewed later). Disclose test doubles/stubs as remaining obligations; production behavior may not silently use them. If assignment.continuation_snapshot is present, this is a fresh revision after a positively settled capacity refusal: inspect retained edits/commits and continue from them without resetting or blindly replaying completed effects. Do not wait for peers: finish and commit your portion. Service owns synchronization at quiescent clean points. When fixing/reviewing, inspect source and behavior, repair issues, update README if assigned. Return the typed worker result, no hidden reasoning. If blocked before creating a commit, return commit as an empty string with the concrete cause in summary and observed failed checks. This is a failure report, never a submitted application result."#;
pub fn instruction(packet:&Value)->&'static str {
    if packet["operation_kind"] == "conversation" && packet["selected_context"]["interaction"]["version"]==1 {
        crate::coordination::DISCUSSION
    } else if packet["operation_kind"] == "conversation" {
        "You are Waywr's conversational coordinator. Use only the supplied packet, no tools. Respond using schema_version 3 with assistant_text, evidence_refs and action. action:null for discussion; upsert_proposal to save the requested scope with exact selected_context.scope.base (null initially); ask_question only for genuinely missing decisions. A proposal contains title, objective, deliverables, in_scope, out_of_scope, constraints, acceptance_criteria, unresolved_decisions. All lists strings. Scope and operating policy approval happen through the TUI; discussion never authorizes work. Approved development will run via Waywr workers automatically. The operating policy is supplied by the application, not your prose. Do not claim that a scope is already saved or that a worker ran. Explain pending application validation. Develop the plan and intended decomposition from this conversation and supplied project context; explain it to the human and refine it before proposing approval. Do not infer an application or task sequence. While development is active, use request_context to answer progress questions; ordinary discussion cannot silently steer or expand scope. Explain explicit /steer for in-scope changes and /reply for an open development question. No arbitrary budget/call limits. Return no hidden reasoning."
    } else if packet["operation_kind"]=="development-review" {
        "You are Waywr's independent review worker. No tools or edits. Review the exact frozen target commits, bases, full selected diffs, requirements, policy and check claims. Return schema_version 1, summary, reviewed_commits in supplied order, findings as concrete strings, requires_changes boolean. Identify risks and missing evidence without inventing executed checks. A review is advisory evidence; only the coordinator and service can integrate. No commit field, no empty commits, no hidden reasoning."
    } else if packet["operation_kind"] == "development-worker" {
        WORKER
    } else {
        COORDINATOR
    }
}
pub fn prepare_native(
    directory: &Path,
    packet: &Value,
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    let instruction=format!("{}\n{}\n{}\n{}\n{}",instruction(packet),if packet["context_delivery"]["version"]==1&&packet["operation_kind"]!="development-worker"{crate::development_context::INSTRUCTION}else{""},if packet["context_delivery"]["version"]==1&&packet["operation_kind"]=="conversation"{crate::development_context::DISCUSSION_INSTRUCTION}else{""},if packet["context_delivery"]["memory_version"]==2&&matches!(packet["operation_kind"].as_str(),Some("development-coordinator"|"development-review")){crate::development_context::MEMORY_INSTRUCTION}else{""},if packet["finding_contract"]==1{crate::review_findings::INSTRUCTION}else if packet["recovery_contract"]==1&&packet["operation_kind"]=="development-coordinator"{recovery::INSTRUCTION}else{""});
    let instruction=if packet["ordinary_project_contract"]==1{format!("{instruction}\n{}",crate::candidate::context_instruction(packet))}else{instruction};
    crate::model_runtime::prepare(directory,packet,&instruction)
}
pub fn inspect_native(v:&Value,_p:&Value)->Result<(),String>{crate::model_runtime::inspect(v)}
pub fn finish_native(events:&[Value],_directory:&Path,packet:&Value,code:Option<i32>)->Result<(Value,Value),String>{
    crate::model_runtime::finish(events,packet,code)
}

fn proven_uninvoked(s: &Session, directory: &Path) -> Result<bool, String> {
    if directory.join("invocation-reservation.json").exists() {
        return Ok(false);
    }
    if directory.join("active-host-attempt.json").exists() {
        let attempt = Attempt::load(&s.workflow.state, directory)?;
        if attempt.directory.join("spawn-committed.json").exists() {
            return Ok(false);
        }
        if attempt.directory.join("prelaunch-refusal.json").exists()
            || attempt.directory.join("pre-dispatch-refusal.json").exists()
        {
            attempt.retire_proven_no_launch(
                "Obsolete requirement generation; positive nonlaunch proof",
            )?;
            return Ok(true);
        }
        return Ok(false);
    }
    let attempts = directory.join("attempts");
    if attempts.is_dir()
        && fs::read_dir(attempts)
            .map_err(|e| e.to_string())?
            .any(|e| e.is_ok_and(|e| e.path().join("start-maybe-sent.json").exists()))
    {
        return Err("Orphan host may have started; no nonlaunch inference".into());
    }
    Ok(true)
}
pub(crate) fn op_dir(s: &Session, work: &str, op: &str) -> PathBuf {
    s.workflow
        .state
        .join("session/development")
        .join(work)
        .join(op)
}
fn bump(w: &mut Value) {
    w["revision"] = json!(w["revision"].as_u64().unwrap_or(0) + 1)
}
fn belongs(v: &Value, work: &Value) -> bool {
    v["work_id"] == work["work_id"]
}
fn units(s: &Session, w: &Value) -> Result<Vec<Value>, String> {
    Ok(s.development_records("unit")?
        .into_iter()
        .filter(|u| belongs(u, w))
        .collect())
}
fn available(all: &[Value], deps: &Value) -> bool {
    deps.as_array().is_some_and(|ds| {
        ds.iter().all(|d| {
            all.iter().any(|u| {
                u["name"] == *d && matches!(u["status"].as_str(), Some("integrated" | "accepted" | "reviewed"))
            })
        })
    })
}
fn start_dependencies_ready(u:&Value,all:&[Value])->bool{
    available(all,&u["start_after"]) || u["role"]=="review"&&u["start_after"].as_array().is_some_and(|deps|deps.iter().all(|name|all.iter().any(|t|t["name"]==*name&&matches!(t["status"].as_str(),Some("submitted"|"reviewed"|"integrated")))))
}
fn retirement_proof(s:&Session,w:&Value,u:&Value)->Result<Value,String>{
    if !belongs(u,w)||u["requirement_generation"]!=w["requirement_generation"]
        ||!matches!(u["status"].as_str(),Some("planned"|"ready"))
        ||u["attempt_revision"].as_u64().unwrap_or(1)!=1||!u["operation_id"].is_null()
        ||!u["result_commit"].is_null()||!u["receipt"].is_null()||!u["review"].is_null()
        ||u["finding_ledger"].as_array().is_some_and(|a|!a.is_empty()) {
        return Err("Only positively unlaunched assignments can be retired; retain results and reconcile uncertain execution".into());
    }
    if recovery::preparation_proven(s,w,&u["recovery"])? {
        return Ok(u["recovery"]["source_evidence"].clone());
    }
    if u["preparation_revision"].is_null() && (u["recovery"].is_null()
        ||u["recovery"]["source_evidence"]["no_prepared_attempt"]==true) {
        return Ok(json!({"source":"saved first-revision assignment before execution reservation","unit_id":u["unit_id"],"assignment_sha256":hash(u)?}));
    }
    Err("Retirement requires positive scheduler nonlaunch proof".into())
}
fn retire_assignments(s:&Session,w:&Value,all:&[Value],names:&[String],op:&str,reason:&str)->Result<Vec<Value>,String>{
    if names.is_empty()||reason.trim().is_empty(){return Err("Retirement requires named assignments and a concrete rationale".into())}
    let mut retired=vec![];
    for name in names {
        if retired.iter().any(|u:&Value|u["name"]==*name){return Err("Duplicate retirement target".into())}
        let mut u=all.iter().find(|u|u["name"]==*name).ok_or("Unknown retirement target")?.clone();
        let proof=retirement_proof(s,w,&u)?;
        if all.iter().any(|other|!names.iter().any(|n|other["name"]==*n)
            &&!matches!(other["status"].as_str(),Some("superseded"|"cancelled"|"failed"|"reviewed"|"integrated"|"accepted"))
            &&["start_after","accept_after","review_target_names","resolution_target_names"].iter().any(|key|other[key].as_array().is_some_and(|a|a.contains(&json!(name))))) {
            return Err("Retirement would strand an outstanding assignment dependency; adapt it first".into());
        }
        let assignment_sha256=hash(&u)?;
        u["status"]=json!("superseded");
        u["retirement"]=json!({"source_operation":op,"assignment_sha256":assignment_sha256,"reason":reason,"proof":proof,"preserved":"Original assignment, preparation refusal, findings and obligations remain evidence; no call replayed"});
        u["recovery"]=recovery::resolved(&u["recovery"],u["retirement"].clone());
        retired.push(u);
    }
    Ok(retired)
}
pub(crate) fn convergence(s:&Session,w:&Value,all:&[Value])->Value{
    let current=all.iter().filter(|u|u["requirement_generation"]==w["requirement_generation"]&&!matches!(u["status"].as_str(),Some("superseded"|"cancelled"))).collect::<Vec<_>>();
    let names=|statuses:&[&str]|current.iter().filter(|u|u["status"].as_str().is_some_and(|v|statuses.contains(&v))).map(|u|u["name"].clone()).collect::<Vec<_>>();
    json!({"submitted":names(&["submitted"]),"producers":names(&["running","capacity-waiting"]),
        "unlaunched":current.iter().filter(|u|matches!(u["status"].as_str(),Some("planned"|"ready"))).map(|u|json!({"name":u["name"],"blocked":u["recovery"].is_object()&&u["recovery"]["phase"]!="resolved","retirement_available":retirement_proof(s,w,u).is_ok(),"recovery":recovery::summary(&u["recovery"])})).collect::<Vec<_>>(),
        "open_obligation_names":w["obligations"].as_array().into_iter().flatten().filter(|o|o["status"]!="resolved").map(|o|o["name"].clone()).collect::<Vec<_>>(),
        "question":w["question"],"guidance":"Integrate ready exact submissions; run combined checks at combined commit. Retire obsolete unlaunched assignments with an explicit reason. Retirement never resolves findings or obligations. Human preview must use working_summary.integration_checkout.directory, not older primary/main. Ask substantive human questions once; do not wait on a preparation blocker with no producer."})
}
fn require_frozen_review(packet: &Value, unit: &Value) -> Result<(), String> {
    let reviewed = packet["units"].as_array().and_then(|all| {
        all.iter()
            .find(|candidate| candidate["unit_id"] == unit["unit_id"])
    });
    if !reviewed.is_some_and(|r| {
        r["status"] == "submitted"
            && (packet["execution"]["profile"]!="development-profiles-v1"||r["review_diff"].is_string()||crate::development_context::diff_delivered(packet,r))
            && !r["result_commit"].is_null()
            && r["result_commit"] == unit["result_commit"]
    }) {
        return Err("Submission was not frozen in this coordinator's review packet; review the completed commit before integrating".into());
    }
    Ok(())
}
fn worktree_path(s: &Session, work: &str, name: &str) -> Result<PathBuf, String> {
    if !id(work) || !id(name) {
        return Err("Invalid worktree identity".into());
    }
    let base = s.project.join(".waywr/worktrees");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let base = base.canonicalize().map_err(|e| e.to_string())?;
    if !base.starts_with(&s.project) || base.starts_with(&s.workflow.state) {
        return Err("Worktree root leaves project or enters canonical state".into());
    }
    let parent = base.join(work);
    if parent.exists() {
        if parent.canonicalize().map_err(|e| e.to_string())? != parent {
            return Err("Worktree parent changed through a symlink; preserved".into());
        }
    } else {
        fs::create_dir(&parent).map_err(|e| e.to_string())?;
    }
    Ok(parent.join(name))
}
fn ensure_tree(s: &Session, path: &Path, branch: &str, base: &str) -> Result<(), String> {
    if path.exists() {
        if path.canonicalize().map_err(|e| e.to_string())? != path
            || git(path, &["symbolic-ref", "--short", "HEAD"])? != branch
        {
            return Err("Existing worktree path has unexpected ownership; preserved".into());
        }
        return Ok(());
    }
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    if git(
        &s.project,
        &["show-ref", "--verify", &format!("refs/heads/{branch}")],
    )
    .is_ok()
    {
        return Err("Existing branch has no assigned worktree; preserve and reconcile".into());
    }
    git(
        &s.project,
        &[
            "worktree",
            "add",
            "-b",
            branch,
            path.to_str().ok_or("Non-UTF8 worktree")?,
            base,
        ],
    )?;
    Ok(())
}
fn valid_paths(paths: &[String]) -> bool {
    !paths.is_empty()
        && paths.iter().all(|p| {
            !p.is_empty()
                && p != "."
                && !matches!(p.trim_end_matches('/'), ".git" | ".waywr")
                && !p.starts_with(".git/")
                && !p.starts_with(".waywr/")
                && !p.starts_with(".wayward-wrangler/")
                && p != ".wayward-wrangler"
                && !p.starts_with(".waywr-migration")
                && Path::new(p)
                    .components()
                    .all(|c| matches!(c, Component::Normal(_)))
        })
}
fn allowed(path: &str, paths: &Value) -> bool {
    paths.as_array().is_some_and(|a| {
        a.iter().filter_map(Value::as_str).any(|p| {
            path == p.trim_end_matches('/')
                || path.starts_with(&format!("{}/", p.trim_end_matches('/')))
        })
    })
}
// Superseded records retain evidence, not write ownership. Running superseded
// attempts remain "running" with supersede_pending until positively settled.
fn validate_write_ownership(paths: &[String], start_after: &[String], existing: &[Value]) -> Result<(), String> {
    for other in existing.iter().filter(|u| !matches!(u["status"].as_str(),
        Some("integrated"|"accepted"|"reviewed"|"failed"|"cancelled"|"superseded"))) {
        if paths.iter().any(|p| allowed(p,&other["paths"]) || other["paths"].as_array().into_iter().flatten().filter_map(Value::as_str).any(|q|q.starts_with(&format!("{}/",p.trim_end_matches('/')))))
            && !start_after.iter().any(|name|other["name"]==*name) {
            return Err(format!("Concurrent task paths overlap with {} ({}); save a dependency or separate ownership", other["name"].as_str().unwrap_or("unknown"), other["status"].as_str().unwrap_or("unknown")));
        }
    }
    Ok(())
}
fn source_contracts(tree: &Path, commit: &str) -> Result<Vec<Value>, String> {
    let mut remaining = 32 * 1024;
    let mut context = Vec::new();
    for path in git(tree, &["ls-tree", "-r", "--name-only", commit])?
        .lines()
        .filter(|p| [".rs", ".h", ".py", ".ts", ".js", ".go"].iter().any(|ext|p.ends_with(ext)) || matches!(*p,"Cargo.toml"|"package.json"|"pyproject.toml"|"go.mod"))
        .take(24)
    {
        if remaining == 0 {
            break;
        }
        let text = git(tree, &["show", &format!("{commit}:{path}")])?;
        let take = text.len().min(8192).min(remaining);
        let mut end = take;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        remaining -= end;
        context.push(json!({"path":path,"source_commit":commit,"sha256":sha256(text.as_bytes()),"excerpt":&text[..end],"truncated":end<text.len(),"purpose":"accepted interface/source context; workers inspect full relevant local source"}));
    }
    Ok(context)
}
fn discussion_text(v:&Value,max:usize)->Option<String>{v.as_str().map(|s|s.chars().take(max).collect())}
fn discussion_unit(u:&Value)->Value{
    json!({"work_id":u["work_id"],"unit_id":u["unit_id"],"title":discussion_text(&u["title"],256),"status":u["status"],"result_commit":u["result_commit"],"requirement_generation":u["requirement_generation"]})
}
fn discussion_history(r:&Value)->Value{
    json!({"request_id":r["request_id"],"log_path":r["log_path"],"summary_path":r["summary_path"]})
}
pub(crate) fn discussion_knowledge(k:&Value)->Value{
    json!({"work_id":k["work_id"],"source_commit":k["source_commit"],"source_map":discussion_text(&k["source_map"],4096),"conventions":discussion_text(&k["conventions"],512),
        "checks":{"commit":k["checks"]["commit"],"passed":k["checks"]["result"]["passed"],"evidence":k["checks"]["evidence"]},
        "contracts":k["contracts"].as_array().map(|all|all.iter().take(4).map(|c|json!({"path":c["path"],"source_commit":c["source_commit"],"sha256":c["sha256"],"excerpt":discussion_text(&c["excerpt"],1024),"selection":"bounded excerpt; full source remains at the exact commit"})).collect::<Vec<_>>()).unwrap_or_default(),
        "selection":"Compact accepted-source context; obligation resolutions and native transcripts stay in Inspect"})
}
/// Fit optional development context around the unchanged mandatory conversation/scope.
/// This runs before the final size check, freezing and native-call reservation.
pub(crate) fn fit_discussion_context(packet:&mut Value,inputs:&Value)->Result<(),String>{
    packet["provider"]=packet["execution"]["provider"].clone();packet["development"]=json!(true);
    if inputs["routing_preferences"].is_object(){
        if inputs["routing_preferences"]["revision"]!=packet["execution"]["selection"]["preferences_revision"]{return Err("busy: Preferences changed during packet preparation; no call reserved".into());}
        packet["routing_preferences"]=inputs["routing_preferences"].clone();
    }
    packet.as_object_mut().unwrap().remove("request_context");packet.as_object_mut().unwrap().remove("project_context");
    let base=crate::codec::canonical(packet).map_err(|e|e.to_string())?.len();
    let ceiling=(packet["execution"]["context_bytes"].as_u64().unwrap_or(crate::native_contract::PACKET_LIMIT as u64) as usize).min(base+24*1024);
    let request=&inputs["request_context"];let project=&inputs["project_context"];
    packet["request_context"]=json!({"work_id":request["work_id"],"selection":request["selection"],"available_counts":{"workflows":request["workflows"].as_array().map_or(0,Vec::len),"units":request["units"].as_array().map_or(0,Vec::len),"knowledge":request["knowledge"].as_array().map_or(0,Vec::len),"history":request["history"].as_array().map_or(0,Vec::len)},"workflows":[],"units":[],"knowledge":[],"history":[],"bounded":"Only fitting items included; counts disclose omission. Full evidence remains in Inspect"});
    packet["project_context"]=json!({"project":project["project"],"head":project["head"],"provenance":project["provenance"],"documents_total":project["documents"].as_array().map_or(0,Vec::len),"tracked_files_total":project["tracked_files"].as_array().map_or(0,Vec::len),"documents":[],"tracked_files":[]});
    // Even selection metadata must fit; the caller trims older messages and retries.
    if crate::codec::canonical(packet).map_err(|e|e.to_string())?.len()>ceiling{return Ok(())}
    for (section,field) in [("request_context","history"),("request_context","workflows"),("request_context","units"),("project_context","documents"),("project_context","tracked_files"),("request_context","knowledge")] {
        let Some(items)=inputs[section][field].as_array()else{continue};
        for item in items {
            packet[section][field].as_array_mut().unwrap().push(item.clone());
            if crate::codec::canonical(packet).map_err(|e|e.to_string())?.len()>ceiling{packet[section][field].as_array_mut().unwrap().pop();}
        }
    }
    Ok(())
}
/// Bounded, project-local planning input. No tools or filesystem authority is granted.
/// Drop only optional repeated context, explicitly marking omissions. Authority is never trimmed.
fn fit_development_packet(packet:&mut Value)->Result<(),String>{
    if packet["context_delivery"]["version"]==1{return if serde_json::to_vec(packet).map_err(|e|e.to_string())?.len()<=crate::development_context::MEMORY_LIMIT{Ok(())}else{Err("Context exceeds application memory bound; no launch".into())};}
    if packet["execution"]["profile"]!="development-profiles-v1"{return Ok(());}
    let limit=packet["execution"]["context_bytes"].as_u64().unwrap_or(96*1024) as usize;
    if packet["operation_kind"]=="development-coordinator"{
        if let Some(work)=packet["workflow"].as_object_mut(){
            for field in ["last_coordinator_result","interrupted_coordinator"]{
                if let Some(full)=work.remove(field){work.insert(field.into(),json!({"evidence":full["evidence"],"operation_id":full["operation_id"],"sha256":hash(&full)?,"selection":"Repeated raw result omitted; retained in Inspect"}));}
            }
        }
        if let Some(all)=packet["knowledge"].as_array(){let count=all.len();let cards=all.iter().rev().take(2).map(discussion_knowledge).collect::<Vec<_>>();packet["knowledge"]=json!(cards);packet["knowledge_selection"]=json!({"available":count,"included":2.min(count),"details":"Full accepted-source records in Inspect"});}
        // Provide compact outcomes for every unit and one fitting frozen submitted diff at a time.
        let mut diffs=vec![];
        if let Some(units)=packet["units"].as_array_mut(){for (i,u) in units.iter_mut().enumerate(){
            if let Some(o)=u.as_object_mut(){
                for field in ["receipt","runtime_usage","worker_report","retained_input_snapshot"]{if let Some(full)=o.remove(field){o.insert(format!("{field}_reference"),json!({"sha256":hash(&full)?,"evidence":full["evidence"],"summary":full["summary"],"error":discussion_text(&full["error"],2000),"status":full["status"],"selection":"Full record in Inspect"}));}}
                if let Some(diff)=o.remove("review_diff"){o.insert("review_diff_selection".into(),json!({"sha256":hash(&diff)?,"included":false,"reason":"Selected diffs must fit the frozen packet; review after earlier submissions integrate"}));diffs.push((i,diff));}
            }
        }}
        for (i,diff) in diffs.into_iter().rev(){
            packet["units"][i]["review_diff"]=diff;
            packet["units"][i]["review_diff_selection"]["included"]=json!(true);
            if crate::codec::canonical(packet).map_err(|e|e.to_string())?.len()>limit{
                packet["units"][i].as_object_mut().unwrap().remove("review_diff");packet["units"][i]["review_diff_selection"]["included"]=json!(false);
            }
        }
    }
    let size=crate::codec::canonical(packet).map_err(|e|e.to_string())?.len();
    if size>limit{return Err(format!("Mandatory development packet is {size} bytes, exceeding selected {limit}-byte budget; no attempt launched. Reduce assignment or review targets without dropping authority."));}
    Ok(())
}
pub(crate) fn planning_context(s:&Session)->Result<Value,String>{crate::project_knowledge::planning(s)}

pub(crate) fn authority(s: &Session, w: &Value) -> Result<bool, String> {
    let Some(scope) = s.scope_detail(strv(w, "work_id")?)? else {
        return Ok(false);
    };
    let actual = s
        .development_record("workflow", strv(w, "work_id")?)?
        .ok_or("Workflow missing")?;
    Ok(
        actual["requirement_generation"] == w["requirement_generation"]
            && scope["authorized"] == true
            && scope["base"] == w["scope_base"]
            && scope["decision_revision"] == w["decision_revision"]
            && if crate::provider_selection::enabled(s){actual["policy"]==w["policy"]}else{policy(s)? == w["policy"]},
    )
}
impl Service {
    /// Legacy integration refs stay checked out and all old refs/worktrees remain
    /// intact. Publish a readable local result alias using a saved CAS intent.
    fn sync_result_branch(&self, w: &mut Value) -> Result<(), String> {
        if w["policy"]["project_workflow"].is_object(){return Ok(())}
        let s = &self.session;
        if !w["head"].is_string() || !w["pending"].is_null()
            || s.phase()?["phase"] != "running" || !authority(s,w)? { return Ok(()); }
        let key = strv(w,"work_id")?.to_owned();
        if s.development_records("effect")?.iter().any(|e|belongs(e,w)&&matches!(e["status"].as_str(),Some("intent"|"applied"))) { return Ok(()); }
        if w["branch_namespace"].is_string() { return Ok(()); } // new refs already readable
        let _gate = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
        let tree = Path::new(strv(w,"integration_worktree")?);
        if git(tree,&["rev-parse","HEAD"])? != w["head"]
            || git(tree,&["branch","--show-current"])? != w["integration_branch"] { return Ok(()); }
        let title = s.scope_detail(&key)?.and_then(|v|v["record"]["proposal"]["title"].as_str().map(str::to_owned)).unwrap_or_else(||"request".into());
        let branch = w["result_branch"].as_str().map(str::to_owned).unwrap_or_else(||format!("{}/integration",request_namespace(&title,&key)));
        let namespace=branch.strip_suffix("/integration").ok_or("Invalid saved result branch")?;
        if !valid_namespace(namespace,&key) || namespace == format!("waywr/{key}") { return Err("Invalid readable result namespace".into()); }
        let reference = format!("refs/heads/{branch}");
        let actual = git(&s.project,&["rev-parse","--verify",&reference]).ok();
        let intent = if w["result_branch_intent"].is_object() {
            w["result_branch_intent"].clone()
        } else {
            if w["result_branch"].is_null() && actual.is_some() { return Err("Readable result branch already exists; preserved without adoption".into()); }
            let expected = w["result_branch_head"].as_str().map(str::to_owned);
            if actual != expected { return Err("Readable result branch changed externally; preserved".into()); }
            if actual.as_deref() == w["head"].as_str() { return Ok(()); }
            let intent=json!({"branch":branch,"expected":expected,"commit":w["head"],"authority_sha256":recovery::authority(w)?});
            w["result_branch_intent"]=intent.clone();
            s.development_save("workflow",&key,w,"development.result_branch_intent")?;
            intent
        };
        if intent["branch"] != branch || intent["commit"] != w["head"] || intent["authority_sha256"] != recovery::authority(w)? { return Err("Saved result branch intent differs; reconciliation required".into()); }
        retain_convergence_contract(s)?;
        if !authority(s,w)? { return Err("Authority changed before result branch update; intent retained".into()); }
        // A crash after update-ref can adopt this exact saved result once.
        if actual.as_deref() != intent["commit"].as_str() {
            if actual.as_deref() != intent["expected"].as_str() { return Err("Result branch diverged from saved intent; preserved".into()); }
            git(&s.project,&["update-ref",&reference,strv(&intent,"commit")?,intent["expected"].as_str().unwrap_or("0000000000000000000000000000000000000000")])?;
        }
        w["result_branch"]=json!(branch);w["result_branch_head"]=w["head"].clone();w["result_branch_intent"]=Value::Null;
        bump(w);
        s.development_save("workflow",&key,w,"development.result_branch_updated")
    }
    /// Observe/adopt existing evidence through operational attention. This does
    /// not turn a pause, product question or changed authority into permission.
    fn reconcile_development_attention(&self,w:&mut Value)->Result<(),String>{
        let s=&self.session;
        if w["status"]!="attention"||s.phase()?["phase"]!="running"||!w["question"].is_null()
            ||!w["provider_blocked"].is_null()||!authority(s,w)? {return Ok(())}
        let key=strv(w,"work_id")?.to_owned();
        if w["recovery"].is_object()&&!recovery::same_authority(w,&w["recovery"])?{return Ok(())}
        if recovery::preparation_changed(s,w,&units(s,w)?)? {
            let resume=w["recovery"]["resume_status"].as_str().unwrap_or("running").to_owned();
            w["recovery"]=recovery::resolved(&w["recovery"],json!({"changed_preparation_signature":recovery::preparation_signature(s,w,&units(s,w)?)?,"no_call_replayed":true}));
            w["status"]=json!(resume);w["attention"]=Value::Null;w["last_context_sha256"]=Value::Null;
            bump(w);s.development_save("workflow",&key,w,"development.recovery_resolved")?;return Ok(())
        }
        if let Some(op)=w["pending"]["operation_id"].as_str().map(str::to_owned){
            let dir=op_dir(s,&key,&op);
            if operation::receipt(&dir).map_err(|e|e.detail)?.is_some(){
                let before=w["pending"].clone();
                if !w["recovery"].is_object(){w["recovery"]=recovery::observe(w,Cause::HostObservation,&key,&op,"A validated result is awaiting adoption",json!({"operation_directory":dir}))?;}
                self.poll_development_coordinator(w)?;
                if before!=w["pending"]&&w["question"].is_null()&&w["status"]=="attention"&&w["recovery"]["phase"]=="resolved"{
                    w["status"]=json!("running");w["attention"]=Value::Null;w["last_context_sha256"]=Value::Null;
                    bump(w);s.development_save("workflow",&key,w,"development.recovery_resolved")?;
                }
            }
            return Ok(())
        }
        for mut e in s.development_records("effect")?.into_iter().filter(|e|belongs(e,w)&&matches!(e["status"].as_str(),Some("intent"|"applied"))){
            let promote=e["kind"]=="promote";
            let tree=if promote{s.project.clone()}else{PathBuf::from(strv(w,"integration_worktree")?)};
            let head=git(&tree,&["rev-parse","HEAD"])?;
            let eid=strv(&e,"effect_id")?.to_owned();
            let applied=if promote{head==e["submitted_commit"]}else{head!=e["expected_head"]&&git(&tree,&["log","-1","--format=%B"])?.contains(&format!("Waywr-Intent: {eid}"))&&git(&tree,&["rev-parse","HEAD^1"])?==e["expected_head"]&&git(&tree,&["rev-parse","HEAD^2"])?==e["submitted_commit"]};
            let unstarted=w["recovery"]["cause"]=="git_effect"&&head==e["expected_head"]&&!op_dir(s,&key,&eid).join("git-may-have-started.json").exists();
            if !(applied||unstarted)||clean(&tree).is_err(){return Ok(())}
            if !w["recovery"].is_object(){w["recovery"]=recovery::observe(w,Cause::GitEffect,&key,&eid,"Reconciling the saved Git effect",json!({"effect":e}))?;}
            self.apply_development_effect(w,&mut e)?;
            w["recovery"]=recovery::resolved(&w["recovery"],json!({"effect_id":eid,"result_commit":e["result_commit"],"already_applied":applied}));
            if w["status"]=="attention"{w["status"]=json!("running");w["attention"]=Value::Null;}
            w["last_context_sha256"]=Value::Null;bump(w);recovery::retain_contract(s)?;
            s.development_save("workflow",&key,w,"development.recovery_resolved")?;return Ok(())
        }
        Ok(())
    }
    pub(crate) fn tick_development(&self) -> Result<(), String> {
        for mut work in self.session.development_records("workflow")? {
            if work["status"] == "delivered" || work["status"] == "cancelled" {
                crate::project_resources::retire(&self.session,&work)?;
                continue;
            }
            let key = strv(&work, "work_id")?.to_owned();
            let result = self.tick_development_work(&mut work);
            if let Err(reason) = result {
                if self
                    .session
                    .development_record("workflow", &key)?
                    .is_some_and(|v| v["requirement_generation"] != work["requirement_generation"])
                {
                    continue;
                }

                if crate::provider_selection::enabled(&self.session)&&(reason.contains("needs explicit provider-policy approval")||reason.starts_with("Task override includes an unapproved")){work["provider_blocked"]=json!({"kind":"policy","reason":reason});}
                if crate::provider_selection::enabled(&self.session) {
                    let boundary=work.as_object_mut().unwrap().remove("recovery_boundary").unwrap_or(json!({"cause":"application_fault"}));
                    let cause:Cause=serde_json::from_value(boundary["cause"].clone()).unwrap_or(Cause::ApplicationFault);
                    let subject=boundary["subject_id"].as_str().unwrap_or(&key);
                    let operation=boundary["operation_id"].as_str().unwrap_or("");
                    let mut record=recovery::observe(&work,cause,subject,operation,&reason,boundary["proof"].clone())?;
                    record["resume_status"]=json!(if work["status"]=="initializing"{"initializing"}else{"running"});
                    work["recovery"]=record;recovery::retain_contract(&self.session)?;
                }
                work["status"] = json!("attention");
                work["attention"] = json!(reason);
                bump(&mut work);
                self.session
                    .development_save("workflow", &key, &work, "development.attention")?;
            }
        }
        Ok(())
    }
    fn tick_development_work(&self, w: &mut Value) -> Result<(), String> {
        let s = &self.session;
        let key = strv(w, "work_id")?.to_owned();
        if w["status"] == "cancelling" {
            return self.cancel_development_work(w);
        }
        // Adopt settled receipts even during stop/steering. No missing-process inference or replay.
        let mut all = units(s, w)?;
        let mut obligations_changed = false;
        for unit in &all {
            for obligation in unit["submission"]["remaining_obligations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !w["obligations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|o| o["name"] == obligation)
                {
                    let contract = w["scope_base"].clone();
                    w["obligations"].as_array_mut().unwrap().push(json!({"name":obligation,"owner":unit["unit_id"],"status":"open","contract_revision":contract,"source_submission":unit["result_commit"],"resolution":null}));
                    obligations_changed = true;
                }
            }
        }
        if obligations_changed {
            bump(w);
            s.development_save("workflow", &key, w, "development.obligations_disclosed")?;
        }
        for u in &mut all {
            let before_unit=u.clone();
            let observed=(||->Result<(),String>{
            if u["status"] == "failed" && u["worker_report"].is_null() {
                let directory = op_dir(s, &key, strv(u, "operation_id")?);
                if let Ok(attempt) = Attempt::load(&s.workflow.state, &directory) {
                    let path = attempt.directory.join("native-final.json");
                    if let Ok(bytes) = fs::read(&path) {
                        if bytes.len() <= 65536 {
                            if let Ok(report) = serde_json::from_slice::<Value>(&bytes) {
                                if validate_result(
                                    &report,
                                    &json!({"operation_kind":"development-worker"}),
                                )
                                .is_ok()
                                {
                                    // Diagnostic projection only: never amend an immutable
                                    // failed receipt or adopt a commit from this report.
                                    u["worker_report"] = json!({"label":"retained worker claims; failed receipt unchanged", "source":path,"sha256":sha256(&bytes),"report":report});
                                    s.development_save(
                                        "unit",
                                        strv(u, "unit_id")?,
                                        u,
                                        "development.worker_report_retained",
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
            if u["status"] == "running" {
                let directory = op_dir(s, &key, strv(u, "operation_id")?);
                if u["supersede_pending"] == true && proven_uninvoked(s, &directory)? {
                    write(
                        &directory.join("obsolete-before-invocation.json"),
                        &json!({"requirement_generation":w["requirement_generation"],"no_invocation_committed":true}),
                        true,
                    )?;
                    u["status"] = json!("superseded");
                    s.development_save(
                        "unit",
                        strv(u, "unit_id")?,
                        u,
                        "development.obsolete_before_invocation",
                    )?;
                    return Ok(());
                }
                match self.ensure(&directory) {
                    Ok(Some(done)) => {
                        u["recovery"]=recovery::resolved(&u["recovery"],json!({"validated_receipt":done}));
                        u["receipt"] = done.clone();
                        u["evidence"] = json!(directory);
                        u["attempt_id"] = done["attempt_id"].clone();
                        let attempt = Attempt::load(&s.workflow.state, &directory)?;
                        u["runtime_usage"] = read(&attempt.directory.join("runtime-result.json"))
                            .unwrap_or(json!({"usage":"unknown","cost":"unknown"}));
                        if done["status"] == "completed" {
                            let result =
                                read(&operation::result_path(&directory).map_err(|e| e.detail)?)?;
                            if u["role"]=="review"{
                                let packet=read(&directory.join("packet.json"))?;
                                validate_result(&result,&packet)?;
                                let disposition=if packet["finding_contract"]==1{Some(crate::review_findings::apply(&result,&packet)?)}else{None};
                                if let Some(d)=&disposition{u["native_review"]=result.clone();u["native_review_sha256"]=d["native_result_sha256"].clone();u["finding_ledger"]=d["ledger"].clone();u["finding_contract"]=json!(1);u["finding_dispositions"]=d["validated_dispositions"].clone();}
                                if packet["evidence_index"]["version"]==1 {
                                    let requests=crate::development_context::validate_requests(&result["evidence_requests"])?;
                                    if !requests.is_empty(){
                                        if packet["recovery_contract"]==1&&recovery::assess_review_read(s,w,u,&packet,&result)?{
                                            u["status"]=json!("failed");u["review_continuation_blocked"]=json!("Equivalent evidence read; native completion retained, no final review accepted");
                                            recovery::retain_contract(s)?;s.development_save("unit",strv(u,"unit_id")?,u,"development.reassessment")?;
                                            bump(w);w["last_context_sha256"]=Value::Null;s.development_save("workflow",&key,w,"development.progress")?;return Ok(());
                                        }
                                        let summary=if packet["context_delivery"]["memory_version"]==2{result["context_summary"].as_str().unwrap_or("")}else{result["summary"].as_str().unwrap_or("")};
                                        let releases=result["release_evidence"].as_array().map(|all|all.iter().filter_map(|v|v.as_str().map(str::to_owned)).collect::<Vec<_>>()).unwrap_or_default();
                                        let exchange=crate::development_context::retrieve_with_memory(s,&directory,&packet,&requests,&u["evidence_context"],summary,&result["findings"],&releases)?;
                                        u["evidence_context"]=crate::development_context::reference(&directory,&exchange)?;
                                        u["status"]=json!("ready");u["attempt_revision"]=json!(u["attempt_revision"].as_u64().unwrap_or(1)+1);
                                        s.development_save("unit",strv(u,"unit_id")?,u,"development.evidence_retrieved")?;
                                        bump(w);w["last_context_sha256"]=Value::Null;s.development_save("workflow",&key,w,"development.progress")?;return Ok(());
                                    }
                                }
                                if packet["integration_head"]!=w["head"]||packet["review_targets"].as_array().unwrap().iter().any(|target|!units(s,w).unwrap_or_default().iter().any(|now|now["unit_id"]==target["unit_id"]&&now["result_commit"]==target["result_commit"]&&now["base_commit"]==target["base_commit"])){
                                    u["status"]=json!("failed");u["review_stale"]=json!(true);
                                }else{u["status"]=json!("reviewed");}
                                let mut result=disposition.as_ref().map(|d|d["effective"].clone()).unwrap_or(result);
                                if packet["finding_contract"]!=1{for finding in u["evidence_context"]["unresolved_findings"].as_array().into_iter().flatten(){if !result["findings"].as_array().unwrap().contains(finding){result["findings"].as_array_mut().unwrap().push(finding.clone());result["requires_changes"]=json!(true);}}}
                                if packet["context_delivery"]["version"]==1{u["review_coverage"]=json!({"diff_complete":packet["review_targets"].as_array().unwrap().iter().all(|u|crate::development_context::diff_delivered(&packet,u)),"resolution_only":packet["review_resolution_only"]==true,"exhaustive_source_review":false,"packet_sha256":hash(&packet)?,"receipts":packet["evidence_receipts"],"supplied_pages":packet["evidence_pages"].as_array().map(|all|all.iter().map(|p|{let mut p=p.clone();p.as_object_mut().unwrap().remove("data");p}).collect::<Vec<_>>())});}
                                if packet["finding_contract"]==1&&result["requires_changes"]==true{
                                    u["recovery"]=recovery::observe_unit(w,u,Cause::FindingResolution,strv(u,"operation_id")?,"Review findings need an exact linked reviewer judgment or implementation repair",json!({"native_result_sha256":u["native_review_sha256"],"effective_requires_changes":true}))?;
                                }
                                u["review_disposition"]=json!({"native_requires_changes":u["native_review"]["requires_changes"],"effective_requires_changes":result["requires_changes"],"native_result_sha256":u["native_review_sha256"]});
                                u["review"]=result;u["review_head"]=packet["integration_head"].clone();
                                s.development_save("unit",strv(u,"unit_id")?,u,"development.review_outcome")?;
                                bump(w);w["last_context_sha256"]=Value::Null;s.development_save("workflow",&key,w,"development.progress")?;return Ok(());
                            }
                            let tree_path = PathBuf::from(strv(u, "worktree")?);
                            let tree = tree_path.as_path();
                            if result["commit"] == "" {
                                // A settled native invocation can report a block without
                                // producing application code. Preserve its actual cause;
                                // it must never become a frozen submission or integration.
                                u["status"] = json!("failed");
                                u["worker_report"] = result;
                                u["retained_input_snapshot"] = workspace_snapshot(tree)?;
                                s.development_save(
                                    "unit",
                                    strv(u, "unit_id")?,
                                    u,
                                    "development.unit_outcome",
                                )?;
                                bump(w);
                                s.development_save("workflow", &key, w, "development.progress")?;
                                return Ok(());
                            }
                            clean(tree)?;
                            let head = git(tree, &["rev-parse", "HEAD"])?;
                            if result["commit"] != head
                                || git(tree, &["symbolic-ref", "--short", "HEAD"])?
                                    != u["branch"].as_str().unwrap_or("")
                            {
                                return Err("Submitted commit/branch differs from worker worktree; retained for reconciliation".into());
                            }
                            git(
                                tree,
                                &[
                                    "merge-base",
                                    "--is-ancestor",
                                    strv(u, "base_commit")?,
                                    &head,
                                ],
                            )?;
                            let changed = git(
                                tree,
                                &["diff", "--name-only", strv(u, "base_commit")?, &head],
                            )?;
                            if changed.lines().any(|p| !allowed(p, &u["paths"])) {
                                return Err("Worker changed a path outside its assignment; submission preserved, integration refused".into());
                            }
                            u["status"] = json!(if u["supersede_pending"] == true {
                                "superseded"
                            } else {
                                "submitted"
                            });
                            u["generated_resources"]=crate::project_resources::finish_generated(tree,&w["policy"],&directory)?;
                            u["result_commit"] = json!(head);
                            u["submission"] = result;
                            u["diff"] = json!(git(
                                tree,
                                &[
                                    "diff",
                                    "--stat",
                                    strv(u, "base_commit")?,
                                    strv(u, "result_commit")?
                                ]
                            )?);
                            let full=git(tree,&["diff","--no-ext-diff",strv(u,"base_commit")?,strv(u,"result_commit")?])?;
                            u["review_diff"]=json!(full.chars().take(56000).collect::<String>());
                            u["review_diff_provenance"]=json!({"full_sha256":sha256(full.as_bytes()),"total_bytes":full.len(),"included_bytes":u["review_diff"].as_str().unwrap().len(),"truncated":full.chars().count()>56000,"base_commit":u["base_commit"],"commit":u["result_commit"],"selection":"Selected diff; full source remains in the retained worktree"});
                        } else if done["status"] == "capacity-waiting" {
                            u["status"] = json!("capacity-waiting");
                            let refusal = capacity_proof(s, &directory)?;
                            u["retry_at"] = json!(
                                now() + refusal["retry_after_secs"].as_u64().unwrap_or(60) as f64
                            );
                            u["wait_reason"] = refusal;
                            if crate::provider_selection::enabled(s){
                                u["fallback_profile"]=json!(crate::provider_selection::fallback_choice(s,u["role"].as_str().unwrap_or("implementation"),&u["selection"],&u["profile_override"],&w["policy"])?);
                                if u["fallback_profile"].is_string(){u["retry_at"]=json!(now());}
                            }
                            u["continuation_snapshot"] =
                                if u["role"]=="review"{Value::Null}else{workspace_snapshot(Path::new(strv(u, "worktree")?))?};
                        } else {
                            u["status"] = json!("failed");
                            u["retained_input_snapshot"] =
                                if u["role"]=="review"{Value::Null}else{workspace_snapshot(Path::new(strv(u, "worktree")?))?};
                            u["failure_signature"] =
                                json!(hash(&json!([done["status"], done["error"]]))?);
                        }
                        s.development_save(
                            "unit",
                            strv(u, "unit_id")?,
                            u,
                            "development.unit_outcome",
                        )?;
                        bump(w);
                        w["pending"] = w["pending"].clone();
                        s.development_save("workflow", &key, w, "development.progress")?;
                    }
                    Ok(None) => {}
                    Err(e) if crate::provider_selection::enabled(s) => {
                        let cause=match e.kind {operation::ErrorKind::Uncertain=>Cause::UncertainExecution, operation::ErrorKind::PreSendRefusal|operation::ErrorKind::OutputInvalid=>Cause::Integrity,_=>Cause::HostObservation};
                        u["recovery"]=recovery::observe_unit(w,u,cause,strv(u,"operation_id")?,&e.detail,json!({"operation_directory":directory,"packet_sha256":read(&directory.join("operation.json")).ok().map(|v|v["packet_sha256"].clone())}))?;
                        recovery::retain_contract(s)?;
                        s.development_save("unit",strv(u,"unit_id")?,u,"development.recovery")?;
                    }
                    Err(e) if e.kind == operation::ErrorKind::Operational => {}
                    Err(e) => return Err(e.detail),
                }
            }
                    Ok(())
            })();
            if let Err(error)=observed {
                if !crate::provider_selection::enabled(s){return Err(error)}
                *u=before_unit;
                u["recovery"]=recovery::observe_unit(w,u,Cause::Integrity,u["operation_id"].as_str().unwrap_or(""),&error,json!({"receipt_and_packet":"retained in the same operation; no replacement"}))?;
                recovery::retain_contract(s)?;s.development_save("unit",strv(u,"unit_id")?,u,"development.recovery")?;
            }
        }
        if crate::provider_selection::enabled(s) {
            self.reconcile_development_attention(w)?;
        }
        self.sync_result_branch(w)?;
        crate::project_resources::retire(&self.session,w)?;
        if s.phase()?["phase"] != "running" || w["status"] == "attention" {
            if crate::provider_selection::enabled(s)&&crate::provider_selection::switch_pending(s)&&w["pending"]["kind"]=="coordinate"{
                let op=strv(&w["pending"],"operation_id")?.to_owned();let directory=op_dir(s,&key,&op);
                if proven_uninvoked(s,&directory)?{w["pending"]=Value::Null;w["last_context_sha256"]=Value::Null;return s.development_save("workflow",&key,w,"providers.coordinator_retired_before_launch");}
                if directory.join("active-host-attempt.json").exists(){return self.poll_development_coordinator(w);}
            }
            return Ok(());
        }
        if !authority(s, w)? {
            w["status"] = json!("attention");
            w["attention"] = json!(
                "Requirements or operating policy changed; active commits retained, integration fenced"
            );
            bump(w);
            s.development_save("workflow", &key, w, "development.requirements_invalidated")?;
            return Ok(());
        }
        if w["status"] == "initializing" {
            if crate::provider_selection::enabled(s){w["recovery_boundary"]=json!({"cause":"workspace","subject_id":key});}
            let _gate = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
            let (base,main,target_base)=if let Some(selected)=crate::project_workflow::baseline(s,w)? {selected}else{
            let init = op_dir(s, &key, "repository");
            if !s.project.join(".git").exists() {
                let allowed = fs::read_dir(&s.project)
                    .map_err(|e| e.to_string())?
                    .all(|e| {
                        e.is_ok_and(|e| {
                            [".waywr", ".gitignore"]
                                .iter()
                                .any(|n| e.file_name() == *n)
                        })
                    });
                if !allowed {
                    return Err("Non-Git project has unexpected existing files; preserve and choose an explicit input baseline".into());
                }
                write(
                    &init.join("intent.json"),
                    &json!({"project":s.project,"purpose":"empty operational baseline, no application source"}),
                    true,
                )?;
                git(&s.project, &["init", "-b", "main"])?;
            }
            if git(&s.project, &["rev-parse", "HEAD"]).is_err() {
                if !init.join("intent.json").exists() {
                    return Err(
                        "Existing unborn repository requires explicit input baseline".into(),
                    );
                }
                let ignore = s.project.join(".gitignore");
                let mut text = fs::read_to_string(&ignore).unwrap_or_default();
                if !text.lines().any(|l| l == ".waywr/") {
                    text.push_str("\n.waywr/\n");
                    write_bytes(&ignore, text.as_bytes(), false)?;
                }
                git(&s.project, &["add", "--", ".gitignore"])?;
                git(
                    &s.project,
                    &[
                        "-c",
                        "user.name=Waywr",
                        "-c",
                        "user.email=waywr@localhost",
                        "commit",
                        "--allow-empty",
                        "-m",
                        "Waywr project baseline (no application source)",
                    ],
                )?;
            }
            let base = git(&s.project, &["rev-parse", "HEAD"])?;
            let main = git(&s.project, &["symbolic-ref", "--short", "HEAD"])?;
            if main != "main" {
                return Err("Checked-main policy requires the primary worktree on main; preserve current branch for reconciliation".into());
            }
                (base.clone(),main,base)
            };
            let title = s.scope_detail(&key)?.and_then(|v|v["record"]["proposal"]["title"].as_str().map(str::to_owned)).unwrap_or_else(||"request".into());
            let namespace = request_namespace(&title, &key);
            let branch = format!("{namespace}/integration");
            w["branch_namespace"] = json!(namespace);
            w["result_branch"] = json!(branch);
            let tree = worktree_path(s, &key, "integration")?;
            // Save the worktree intent before Git. Re-entry only adopts an exact clean owned tree.
            w["base_commit"] = json!(base);
            w["main_branch"] = json!(main);
            w["target_base_commit"] = json!(target_base);
            w["integration_branch"] = json!(branch);
            w["integration_worktree"] = json!(tree);
            retain_convergence_contract(s)?;
            s.development_save("workflow", &key, w, "development.git_intent")?;
            ensure_tree(s, &tree, &branch, &base)?;
            clean(&tree)?;
            w["head"] = json!(git(&tree, &["rev-parse", "HEAD"])?);
            w["status"] = json!("running");
            bump(w);
            s.development_save("workflow", &key, w, "development.started")?;
        }
        if crate::candidate::writes_held(w){self.start_development_units(w)?;return Ok(())}
        // A saved effect is reconciled before any new coordinator action.
        for mut effect in s
            .development_records("effect")?
            .into_iter()
            .filter(|e| belongs(e, w) && matches!(e["status"].as_str(), Some("intent" | "applied"))).collect::<Vec<_>>()
        {
            if effect["requirement_generation"] != w["requirement_generation"] {
                effect["status"] = json!("superseded");
                self.session.development_save(
                    "effect",
                    strv(&effect, "effect_id")?,
                    &effect,
                    "development.integration_superseded",
                )?;
                continue;
            }
            if effect["kind"]=="promote"&&w["policy"]["project_workflow"].is_object()&&effect["status"]=="intent"&&!op_dir(s,&key,strv(&effect,"effect_id")?).join("git-may-have-started.json").exists(){
                let target=crate::project_workflow::ref_head(&s.project,strv(w,"main_branch")?)?;
                if effect["expected_head"]!=target&&effect["submitted_commit"]!=target{effect["status"]=json!("superseded");effect["reason"]=json!("Target advanced before mutation; coordinator must refresh/recheck the governing candidate");w["target_observation"]=json!({"commit":target,"expected":effect["expected_head"],"requires_refresh":true});w["last_context_sha256"]=Value::Null;s.development_save("effect",strv(&effect,"effect_id")?,&effect,"development.target_changed")?;s.development_save("workflow",&key,w,"development.target_changed")?;continue}
            }
            w["recovery_boundary"]=json!({"cause":"git_effect","subject_id":key,"operation_id":effect["effect_id"],"proof":{"effect_sha256":hash(&effect)?,"effect":effect}});
            let applied=self.apply_development_effect(w,&mut effect);
            if let Err(error)=applied{
                if !crate::provider_selection::enabled(s){return Err(error)}
                w["recovery"]=recovery::observe(w,Cause::GitEffect,&key,strv(&effect,"effect_id")?,&error,json!({"effect_sha256":hash(&effect)?,"effect":effect}))?;
                w["attention"]=json!(error);w.as_object_mut().unwrap().remove("recovery_boundary");
                recovery::retain_contract(s)?;s.development_save("workflow",&key,w,"development.recovery")?;
                // The integration lane stays fenced; pre-approved independent
                // assignments still use their own exact bases and worktrees.
                self.start_development_units(w)?;return Ok(());
            }
            if w["recovery"]["cause"]=="git_effect"{w["recovery"]=recovery::resolved(&w["recovery"],json!({"effect_id":effect["effect_id"],"result_commit":effect["result_commit"]}));w["attention"]=Value::Null;s.development_save("workflow",&key,w,"development.recovery_resolved")?;}
            w.as_object_mut().unwrap().remove("recovery_boundary");
            return Ok(());
        }
        w.as_object_mut().unwrap().remove("recovery_boundary");
        if w["policy"]["project_workflow"].is_object()&&crate::candidate::start_handoff(s,w)?{self.start_development_units(w)?;return Ok(())}
        if w["policy"]["project_workflow"]["human_validation"]==true&&w["candidate"]["state"]=="ready"&&w["candidate"]["commit"]==w["head"]&&w["candidate"]["generation"]==w["requirement_generation"]&&w["pending"].is_null(){self.start_development_units(w)?;return Ok(())}
        // Pending coordinator/check operations do not own the worker dispatch lane.
        // Eligible independent assignments keep starting while those operations settle.
        self.start_development_units(w)?;
        if !w["pending"].is_null() {
            return self.poll_development_coordinator(w);
        }
        if crate::provider_selection::enabled(s)&&crate::provider_selection::switch_pending(s){return Ok(());}
        all = units(s, w)?;
        // A task outcome or no runnable work requests one compact coordinator decision. Avoid idle narration calls.
        let progress=if crate::provider_selection::enabled(s){recovery::progress(s,w,&all)?}else{Value::Null};
        if crate::provider_selection::enabled(s){
            let was_waiting=w["progress"]["wait_state"].is_string();
            if !recovery::wake_progress(w,&progress){return Ok(())}
            if was_waiting{s.development_save("workflow",&key,w,"development.recovery_resolved")?;}
        }
        let context_hash=if crate::provider_selection::enabled(s){hash(&json!([progress["sha256"],w["recovery"]["cause"],w["recovery"]["phase"],w["evidence_pending"]]))?}else{
hash(&json!([
            &all,
            w["head"],
            w["checks"],
            w["obligations"],
            w["scope_base"],
            w["steering"],
            w["recoveries"],
            w["answers"],
            w["evidence_context"],
            w["requirement_generation"],
            w["attention"]
        ]))?};
        if w["last_context_sha256"] == context_hash {
            return Ok(());
        }
        if w["evidence_pending"]!=true && !all.is_empty()
            && all.iter().all(|u| {
                matches!(
                    u["status"].as_str(),
                    Some("running" | "planned" | "capacity-waiting" | "ready")
                )
            })
        {
            return Ok(());
        }
        if w["capacity_retry_at"].as_f64().is_some_and(|t| now() < t) {
            return Ok(());
        }
        let seq = w["coordinator_sequence"].as_u64().unwrap_or(0) + 1;
        let op = if crate::provider_selection::enabled(s){format!("coordinator-{seq}-ctx{}",&hash(&json!([context_hash,s.release,w["revision"]]))?[..12])}else{format!("coordinator-{seq}")};
        let directory = op_dir(s, &key, &op);
        let mut visible = all.clone();
        for u in &mut visible {
            if u["status"] != "submitted" {
                u.as_object_mut().unwrap().remove("review_diff");
            }
        }
        if crate::provider_selection::enabled(s){
            let (execution,selection)=crate::provider_selection::execution(s,"coordinator",&Value::Null,None,&w["policy"],"review current request events")?;
            w["profile"]=execution;w["coordinator_selection"]=selection;
        }
        let mut packet = json!({"schema_version":1,"development":true,"operation_kind":"development-coordinator","operation_id":op,"run_id":key,"stage":"coordinate","role":"coordinator","provider":w["profile"]["provider"],"model":w["profile"]["model"],"execution":w["profile"],"project_key":s.key,"request_id":w["request_id"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"policy":w["policy"],"workflow":w,"units":visible,"requirements":s.scope_detail(&key)?.map(|v|v["record"]["proposal"].clone()),"knowledge":s.development_records("knowledge")?,"context_sha256":context_hash,"routing_preferences":if crate::provider_selection::enabled(s){crate::provider_selection::snapshot(s)?}else{Value::Null}});
        if crate::provider_selection::enabled(s){packet["progress_snapshot"]=progress;}
        let prepared=(||->Result<(),String>{
            crate::development_context::prepare(s,&directory,&mut packet)?;
            fit_development_packet(&mut packet)?;
            operation::prepare_host(&s.workflow.state,&directory,&packet,&s.key).map_err(|e|e.detail)?;Ok(())
        })();
        if let Err(error)=prepared {
            if crate::provider_selection::enabled(s) {
                let signature=recovery::preparation_signature(s,w,&all)?;
                let proof=recovery::preparation_refusal(s,w,&directory,&signature,&error)?;
                w["recovery_boundary"]=json!({"cause":"preparation","subject_id":key,"operation_id":op,"proof":proof});
            }
            return Err(error);
        }
        w["pending"] = json!({"operation_id":op,"kind":"coordinate","started_at":now(),"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"context_sha256":context_hash});
        w["coordinator_sequence"] = json!(seq);
        if crate::provider_selection::enabled(s){
            *w=s.development_reserve_coordinator(w)?;
        }else{s.development_save("workflow", &key, w, "development.coordinator_reserved")?;}
        Ok(())
    }
    fn start_development_units(&self, w: &Value) -> Result<(), String> {
        let s = &self.session;
        let key = strv(w, "work_id")?;
        let all = units(s, w)?;
        // Start independent ready units up to configured capacity; one assigned worktree per unit.
        let mut running = all.iter().filter(|u| u["status"] == "running").count();
        for u in &mut all.clone() {
            if u["status"] == "capacity-waiting"
                && now() >= u["retry_at"].as_f64().unwrap_or(f64::MAX)
            {
                // Positive completed capacity refusal permits a new revision; ambiguous failures never arrive here.
                u["attempt_revision"] = json!(u["attempt_revision"].as_u64().unwrap_or(1) + 1);
                u["status"] = json!("ready");
                s.development_save(
                    "unit",
                    strv(u, "unit_id")?,
                    u,
                    "development.capacity_resumed",
                )?;
            }
            if running >= w["policy"]["workers"]["concurrency"].as_u64().unwrap_or(1) as usize {
                break;
            }
            if matches!(u["status"].as_str(), Some("planned" | "ready"))
                && start_dependencies_ready(u,&all)
            {
                if crate::provider_selection::enabled(s)&&u["recovery"]["phase"]!="resolved"&&u["recovery"].is_object(){
                    let grant_resolved=u["recovery"]["cause"]=="provider_grant"&&u["recovery"]["source_evidence"]["no_prepared_attempt"]==true&&crate::provider_selection::execution(s,u["role"].as_str().unwrap_or("implementation"),&u["profile_override"],u["fallback_profile"].as_str().or(u["requested_profile"].as_str()).filter(|v|!v.is_empty()),&w["policy"],"recheck recorded preparation grant").is_ok();
                    let expected=&u["recovery"]["source_evidence"]["workspace"];
                    let workspace_resolved=u["recovery"]["cause"]=="workspace"&&recovery::same_authority(w,&u["recovery"])?&&expected["path"]==u["worktree"]&&expected["branch"]==u["branch"]&&expected["base_commit"].is_string()&&expected["path"].as_str().is_some_and(|path|{let path=Path::new(path);clean(path).is_ok()&&git(path,&["rev-parse","HEAD"]).ok().as_deref()==expected["base_commit"].as_str()&&git(path,&["branch","--show-current"]).ok().as_deref()==expected["branch"].as_str()})&&u["continuation_snapshot"].is_null();
                    if !(grant_resolved||workspace_resolved||recovery::unit_preparation_changed(s,w,u,&all)?){continue;}
                    u["recovery"]=recovery::resolved(&u["recovery"],json!({"preparation_inputs_changed":!grant_resolved&&!workspace_resolved,"grant_revalidated":grant_resolved,"workspace_reconciled":workspace_resolved}));
                    u["preparation_revision"]=json!(u["preparation_revision"].as_u64().unwrap_or(0)+1);
                }
                let before_unit=u.clone();let mut preparation_cause=Cause::Preparation;
                let prepared=(||->Result<(),String>{
                let role=u["role"].as_str().unwrap_or("implementation").to_owned();
                if role=="review"&&u["review_target_names"].as_array().is_none_or(|targets|targets.iter().any(|name|!all.iter().any(|t|t["name"]==*name&&matches!(t["status"].as_str(),Some("submitted"|"integrated"))))){return Ok(());}

                preparation_cause=Cause::ProviderGrant;
                let execution=if crate::provider_selection::enabled(s){
                    let requested=u["fallback_profile"].as_str().or(u["requested_profile"].as_str()).filter(|v|!v.is_empty());
                    let (mut execution,mut selection)=crate::provider_selection::execution(s,&role,&u["profile_override"],requested,&w["policy"],u["routing_reason"].as_str().unwrap_or("role default"))?;
                    if u["fallback_profile"].is_string(){
                        selection["source"]=json!("settled capacity fallback");selection["reason"]=json!("Explicit capacity refusal inside approved alternatives; retained partial work checked");
                        selection["previous_attempt"]=json!({"operation_id":u["operation_id"],"attempt_id":u["attempt_id"],"packet_sha256":u["receipt"]["packet_sha256"]});execution["selection"]=selection.clone();
                    }
                    u["provider"]=execution["provider"].clone();u["model"]=execution["model"].clone();u["effort"]=execution["effort"].clone();u["selection"]=selection;execution
                }else{w["profile"].clone()};
                if role=="review"{
                    let targets=u["review_target_names"].as_array().ok_or("Review targets missing")?.iter().map(|name|all.iter().find(|t|t["name"]==*name&&matches!(t["status"].as_str(),Some("submitted"|"integrated"))).cloned().ok_or("Review target not submitted")).collect::<Result<Vec<_>,_>>()?;
                    if targets.is_empty(){return Err("Review requires exact submitted targets".into());}
                    let op=format!("review-{}-r{}-ctx{}",strv(u,"name")?,u["attempt_revision"].as_u64().unwrap_or(1),&hash(&json!([s.release,execution,w["scope_base"],u["evidence_context"],u["preparation_revision"]]))?[..12]);
                    u["operation_id"]=json!(op);u["status"]=json!("running");u["started_at"]=json!(now());
                    let mut packet=json!({"schema_version":1,"development":true,"operation_kind":"development-review","operation_id":op,"run_id":key,"stage":u["name"],"role":"review","provider":execution["provider"],"model":execution["model"],"execution":execution,"project_key":s.key,"request_id":w["request_id"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"policy":w["policy"],"assignment":u,"review_targets":targets,"integration_head":w["head"],"checks":w["checks"],"knowledge":s.development_records("knowledge")?.iter().rev().take(1).map(discussion_knowledge).collect::<Vec<_>>(),"requirements":s.scope_detail(key)?.map(|v|v["record"]["proposal"].clone())});
                    preparation_cause=Cause::Preparation;
                    crate::review_findings::prepare(&mut packet,u,&all)?;
                    crate::development_context::prepare(s,&op_dir(s,key,&op),&mut packet)?;
                    fit_development_packet(&mut packet)?;
                    operation::prepare_host(&s.workflow.state,&op_dir(s,key,&op),&packet,&s.key).map_err(|e|e.detail)?;
                    if s.development_reserve_unit(w,u,"development.review_started")?{running+=1;}return Ok(());
                }
                preparation_cause=Cause::Workspace;
                let tree = PathBuf::from(strv(u, "worktree")?);
                let _gate = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
                if u["base_commit"].is_null() {
                    u["base_commit"] = w["head"].clone();
                    s.development_save("unit", strv(u, "unit_id")?, u, "development.unit_base")?;
                }
                if w["policy"]["project_workflow"].is_object()&&u["resource_intent"].is_null()&&!tree.exists()&&crate::project_workflow::ref_head(&s.project,strv(u,"branch")?).is_err(){
                    u["resource_intent"]=json!({"path":tree,"branch":u["branch"],"base":u["base_commit"],"owner":"service"});s.development_save("unit",strv(u,"unit_id")?,u,"development.resource_intent")?;
                }
                ensure_tree(s, &tree, strv(u, "branch")?, strv(u, "base_commit")?)?;
                if u["continuation_snapshot"].is_null()&&u["receipt"].is_null()&&git(&tree,&["rev-parse","HEAD"])?!=strv(u,"base_commit")?{return Err("Fresh assignment checkout differs from its frozen base; preserve and reconcile before launch".into())}
                if u["resource_intent"].is_object(){u["resource_owned"]=json!(true);}

                if u["continuation_snapshot"].is_object() {
                    if workspace_snapshot(&tree)? != u["continuation_snapshot"] {
                        return Err("Capacity continuation input changed after the settled refusal; preserve and reconcile before relaunch".into());
                    }
                } else {
                    clean(&tree)?;
                }
                let mut op = format!(
                    "worker-{}-r{}",
                    strv(u, "name")?,
                    u["attempt_revision"].as_u64().unwrap_or(1)
                );
                if crate::provider_selection::enabled(s){op=format!("{op}-ctx{}",&hash(&json!([s.release,execution,w["scope_base"],u["evidence_context"],u["preparation_revision"]]))?[..12]);}
                crate::project_resources::begin_generated(&tree,&w["policy"],&op_dir(s,key,&op),&u["generated_resources"])?;
                u["operation_id"] = json!(op);
                u["status"] = json!("running");
                u["started_at"] = json!(now());
                let mut packet = json!({"schema_version":1,"development":true,"operation_kind":"development-worker","operation_id":op,"run_id":key,"stage":u["name"],"role":"implementation","provider":execution["provider"],"model":execution["model"],"execution":execution,"project_key":s.key,"request_id":w["request_id"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"policy":w["policy"],"worktree":tree,"git_common_dir":git(&s.project,&["rev-parse","--path-format=absolute","--git-common-dir"])?,"assignment":u,"requirements":s.scope_detail(&key)? .map(|v|v["record"]["proposal"].clone()),"notifications":s.development_records("notification")?.into_iter().filter(|n|n["recipient"]==u["unit_id"]).collect::<Vec<_>>()});
                preparation_cause=Cause::Preparation;
                crate::development_context::prepare(s,&op_dir(s,&key,&op),&mut packet)?;
                fit_development_packet(&mut packet)?;
                operation::prepare_host(&s.workflow.state, &op_dir(s, &key, &op), &packet, &s.key)
                    .map_err(|e| e.detail)?;
                if crate::provider_selection::enabled(s){
                    if s.development_reserve_unit(w,u,"development.assignment_started")?{running+=1;}
                }else{s.development_save("unit",strv(u,"unit_id")?,u,"development.assignment_started")?;running+=1;}
                Ok(())})();
                if let Err(error)=prepared{
                    if !crate::provider_selection::enabled(s){return Err(error)}
                    let op=u["operation_id"].as_str().unwrap_or("").to_owned();
                    let proof=if preparation_cause==Cause::Preparation&&!op.is_empty(){
                        let signature=recovery::unit_preparation_signature(s,w,u,&all)?;
                        recovery::preparation_refusal_kind(s,w,&op_dir(s,key,&op),&signature,&error,"assignment-v2")?
                    }else{json!({"stage":preparation_cause,"no_prepared_attempt":true,"workspace":{"path":u["worktree"],"branch":u["branch"],"base_commit":u["base_commit"]}})};
                    let record=recovery::observe_unit(w,u,preparation_cause,&op,&error,proof)?;
                    *u=before_unit;
                    u["recovery"]=record;
                    recovery::retain_contract(s)?;s.development_save("unit",strv(u,"unit_id")?,u,"development.recovery")?;
                }

            }
        }
        Ok(())
    }
    fn cancel_development_work(&self, w: &mut Value) -> Result<(), String> {
        let s = &self.session;
        let key = strv(w, "work_id")?.to_owned();
        let command = strv(w, "cancel_command_id")?.to_owned();
        let mut waiting = false;
        let mut targets = units(s, w)?
            .into_iter()
            .filter(|u| u["status"] == "running")
            .map(|u| (Some(u.clone()), strv(&u, "operation_id").map(str::to_owned)))
            .collect::<Vec<_>>();
        if let Some(op) = w["pending"]["operation_id"].as_str() {
            targets.push((None, Ok(op.to_owned())))
        }
        for (unit, op) in targets {
            let op = op?;
            let dir = op_dir(s, &key, &op);
            let settled = if dir.join("active-host-attempt.json").exists() {
                let attempt = Attempt::load(&s.workflow.state, &dir)?;
                attempt.request_cancel(&command)?;
                self.ensure(&dir).map_err(|e| e.detail)?.is_some()
            } else {
                // This operation has no allocated host pointer; verify no orphan attempt was possibly sent.
                let attempts = dir.join("attempts");
                if attempts.is_dir()
                    && fs::read_dir(&attempts)
                        .map_err(|e| e.to_string())?
                        .any(|e| e.is_ok_and(|e| e.path().join("start-maybe-sent.json").exists()))
                {
                    return Err("Cancellation found a possibly sent orphan host attempt; preserve and reconcile".into());
                }
                write(
                    &dir.join("cancelled-before-host.json"),
                    &json!({"command_id":command,"no_start_request_sent":true}),
                    true,
                )?;
                true
            };
            if settled {
                if let Some(mut unit) = unit {
                    unit["status"] = json!("cancelled");
                    s.development_save(
                        "unit",
                        strv(&unit, "unit_id")?,
                        &unit,
                        "development.unit_cancelled",
                    )?
                } else {
                    w["pending"] = Value::Null;
                }
            } else {
                waiting = true;
            }
        }
        if !waiting {
            for mut unit in units(s, w)?.into_iter().filter(|u| {
                !matches!(
                    u["status"].as_str(),
                    Some("integrated" | "accepted" | "reviewed" | "failed" | "cancelled")
                )
            }) {
                unit["status"] = json!("cancelled");
                s.development_save(
                    "unit",
                    strv(&unit, "unit_id")?,
                    &unit,
                    "development.unit_cancelled",
                )?;
            }
            w["status"] = json!("cancelled");
            w["attention"] =
                json!("Work cancelled; branches, worktrees, receipts and history retained");
            bump(w);
            s.development_save("workflow", &key, w, "development.cancelled")?;
        }
        Ok(())
    }
    fn poll_development_coordinator(&self, w: &mut Value) -> Result<(), String> {
        let s = &self.session;
        let key = strv(w, "work_id")?.to_owned();
        let pending = w["pending"].clone();
        let op = strv(&pending, "operation_id")?;
        let dir = op_dir(s, &key, op);
        if (pending["requirement_generation"] != w["requirement_generation"]
            || pending["scope_base"] != w["scope_base"])
            && proven_uninvoked(s, &dir)?
        {
            w["last_obsolete_result"] = json!({"operation":op,"positive_nonlaunch":true});
            w["pending"] = Value::Null;
            w["last_context_sha256"] = Value::Null;
            bump(w);
            s.development_save(
                "workflow",
                &key,
                w,
                "development.obsolete_before_invocation",
            )?;
            return Ok(());
        }
        let switching=if crate::provider_selection::enabled(s){crate::provider_selection::switch(s)?}else{Value::Null};
        let interrupt=pending["kind"]=="coordinate"&&switching["mode"]=="interrupt"&&crate::provider_selection::switch_pending(s);
        if interrupt{
            if dir.join("active-host-attempt.json").exists(){Attempt::load(&s.workflow.state,&dir)?.request_cancel(switching["switch_id"].as_str().ok_or("Missing switch ID")?)?;}
            else if proven_uninvoked(s,&dir)?{w["pending"]=Value::Null;w["last_context_sha256"]=Value::Null;return s.development_save("workflow",&key,w,"providers.coordinator_interrupted_before_launch");}
        }
        let done = match self.ensure(&dir) {
            Ok(Some(v)) => v,
            Ok(None) => return Ok(()),
            Err(e) if crate::provider_selection::enabled(s) => {
                let cause=match e.kind {operation::ErrorKind::Uncertain=>Cause::UncertainExecution,operation::ErrorKind::PreSendRefusal|operation::ErrorKind::OutputInvalid=>Cause::Integrity,_=>Cause::HostObservation};
                if cause==Cause::UncertainExecution&&crate::provider_selection::switch_pending(s){w["switch_uncertain"]=json!(true);}
                w["recovery"]=recovery::observe(w,cause,&key,op,&e.detail,json!({"operation_directory":dir}))?;
                recovery::retain_contract(s)?;s.development_save("workflow",&key,w,"development.recovery")?;return Ok(());
            }
            Err(e) if e.kind == operation::ErrorKind::Operational => return Ok(()),
            Err(e) => return Err(e.detail),
        };
        if interrupt{
            let attempt=Attempt::load(&s.workflow.state,&dir)?;
            let proof=read(&attempt.directory.join("completion.json")).unwrap_or(json!({"known_completion":false,"settled":false,"reason":"Missing completion proof; retain host evidence"}));
            if proof["known_completion"]!=true||proof["settled"]!=true{w["switch_uncertain"]=json!(true);return Err("Interrupted coordinator has no positive settlement; replacement blocked".into());}
            w["interrupted_coordinator"]=json!({"operation_id":op,"receipt":done,"proof":proof,"result_adopted":false});
            w["pending"]=Value::Null;w["last_context_sha256"]=Value::Null;bump(w);return s.development_save("workflow",&key,w,"providers.coordinator_interrupted");
        }
        if done["status"] == "capacity-waiting" {
            w["pending"] = Value::Null;
            let refusal = capacity_proof(s, &dir)?;
            w["capacity_retry_at"] =
                json!(now() + refusal["retry_after_secs"].as_u64().unwrap_or(60) as f64);
            w["capacity_refusal"] = refusal;
            if crate::provider_selection::enabled(s){
                let attempt=Attempt::load(&s.workflow.state,&dir)?;let proof=read(&attempt.directory.join("completion.json"))?;
                if crate::provider_selection::capacity_switch(s,&w["policy"],&proof,op)?{w["capacity_retry_at"]=json!(now());}
            }
            if crate::provider_selection::enabled(s){w["recovery"]=recovery::observe(w,Cause::Capacity,&key,op,"Provider capacity refusal; no inference while waiting",json!({"receipt":done,"retry_at":w["capacity_retry_at"]}))?;recovery::retain_contract(s)?;}
            w["attention"] = json!("Provider capacity refusal; saved work retained under the configured wait/fallback policy");
            bump(w);
            return s.development_save("workflow", &key, w, "development.capacity_wait");
        }
        if done["status"] != "completed" {
            let attempt = Attempt::load(&s.workflow.state, &dir)?;
            let completion =
                read(&attempt.directory.join("completion.json")).unwrap_or(Value::Null);
            if crate::provider_selection::enabled(s)&&completion["category"]=="configuration"&&(completion["no_provider_process_spawned"]==true||completion["no_model_request_sent"]==true)&&completion["known_completion"]==true&&completion["settled"]==true{
                w["recovery"]=recovery::observe(w,Cause::ProviderConfiguration,&key,op,"Provider configuration refused before launch",json!({"completion":completion,"receipt":done}))?;recovery::retain_contract(s)?;
                w["pending"]=Value::Null;w["last_context_sha256"]=Value::Null;w["status"]=json!("attention");
                w["provider_blocked"]=json!({"kind":"configuration","proof":completion,"operation_id":op});w["attention"]=json!(format!("Provider configuration blocked before model launch: {}. Correct F3 settings or switch to an approved profile.",done["error"].as_str().unwrap_or("Inspect retained evidence")));
                return s.development_save("workflow",&key,w,"providers.configuration_blocked");
            }
            if completion["known_completion"] == true && completion["settled"] == true {
                // A positively settled failed call has no action to adopt. Keep
                // its immutable receipt and ask a fresh coordinator to adapt.
                // Ambiguous native/Git effects still require reconciliation.
                if crate::provider_selection::enabled(s){
                    let packet=read(&dir.join("packet.json"))?;
                    let mut diagnostic=done["error"].as_str().unwrap_or("").replace(op,"<operation>");if let Some(path)=dir.to_str(){diagnostic=diagnostic.replace(path,"<evidence>");}
                    let state=if packet["progress_snapshot"].is_object(){packet["progress_snapshot"].clone()}else{recovery::progress(s,w,&units(s,w)?)?};
                    let repeated=packet["recovery_contract"]==1&&recovery::reassess(w,&state,&json!({"action":"settled_failure","kind":pending["kind"],"category":completion["category"],"diagnostic":diagnostic}),false)?;
                    if !repeated{w["recovery"]=recovery::observe(w,if pending["kind"]=="check"{Cause::CheckFailure}else{Cause::SettledFailure},&key,op,"Settled operation failed; coordinator can adapt",json!({"receipt":done,"completion":completion}))?;}
                    recovery::retain_contract(s)?;
                }
                w["pending"] = Value::Null;
                w["attention"] = json!({"kind":"settled operation failed; automatic replanning",
                    "operation":op,"receipt":done,"evidence":dir});
                w["last_context_sha256"] = Value::Null;
                w["capacity_retry_at"] = json!(now() + 5.0);
                bump(w);
                return s.development_save("workflow", &key, w, "development.operation_failed");
            }
            return Err(format!(
                "Coordinator/check outcome needs reconciliation; immutable evidence {}: {}",
                dir.display(),
                done["error"]
            ));
        }
        let result = read(&operation::result_path(&dir).map_err(|e| e.detail)?)?;
        let frozen_packet = read(&dir.join("packet.json"))?;
        if hash(&frozen_packet)? != done["packet_sha256"] {
            return Err("Coordinator/check packet differs from its immutable receipt".into());
        }
        if !authority(s, w)? {
            return Err("Requirements changed during coordinator/check; saved result retained, adoption fenced".into());
        }
        w["recovery"]=recovery::resolved(&w["recovery"],json!({"validated_receipt":done,"packet_sha256":hash(&frozen_packet)?}));
        w["pending"] = Value::Null;
        if pending["requirement_generation"] != w["requirement_generation"]
            || pending["scope_base"] != w["scope_base"]
        {
            w["last_obsolete_result"] =
                json!({"operation":op,"receipt":done,"result_sha256":hash(&result)?});
            w["last_context_sha256"] = Value::Null;
            bump(w);
            s.development_save("workflow", &key, w, "development.obsolete_result")?;
            return Ok(());
        }
        if pending["kind"] == "check" {
            if git(
                Path::new(strv(w, "integration_worktree")?),
                &["rev-parse", "HEAD"],
            )? != pending["checked_commit"].as_str().unwrap_or("")
            {
                return Err("Integration revision changed during checks".into());
            }
            clean(Path::new(strv(w, "integration_worktree")?))?;
            let attempt=Attempt::load(&s.workflow.state,&dir)?;let generated=attempt.directory.join("generated-after.json");
            w["checks"] = json!({"commit":pending["checked_commit"],"result":result,"evidence":dir,"receipt":done,"generated_resources":if generated.exists(){json!({"path":generated,"sha256":execution::hash_file(&generated)?})}else{Value::Null}});
            w["generated_resources"]=w["checks"]["generated_resources"].clone();
            bump(w);
            w["last_context_sha256"] = Value::Null;
            s.development_save("workflow", &key, w, "development.combined_checks")?;
            return Ok(());
        }
        let decision: Decision =
            serde_json::from_value(result.clone()).map_err(|e| e.to_string())?;
        let fingerprint = hash(&json!([
            pending["context_sha256"],
            result["action"],
            result["tasks"],
            result["units"],
            result["checks"],
            result["evidence_requests"]
        ]))?;
        let repeated = w["last_action_fingerprint"] == fingerprint;
        w["last_action_fingerprint"] = json!(fingerprint);
        w["last_context_sha256"] = pending["context_sha256"].clone();
        w["last_coordinator_result"] = json!({"evidence":dir,"result":result,"receipt":done});
        w["evidence_pending"]=json!(false);
        if frozen_packet["context_delivery"]["memory_version"]==2{w["context_summary"]=json!({"text":decision.context_summary,"source_operation":op,"label":"public model-reported facts; execution receipts remain authoritative"});}
        let narrative = decision.assistant_text.clone();
        let mut records: Vec<(String, String, Value, String)> = vec![];
        if frozen_packet["recovery_contract"]==1&&recovery::assess_decision(s,w,&frozen_packet,&result,&units(s,w)?)?{
            w["pending"]=Value::Null;w["last_context_sha256"]=Value::Null;
            w["recovery"]["source_operation"]=json!(op);
            recovery::retain_contract(s)?;bump(w);s.development_commit_decision(w,&[],op)?;return Ok(());
        }
        let before_action = w.clone();
        let adoption = (|| -> Result<(), String> {
            if repeated&&frozen_packet["recovery_contract"]!=1 {
                return Err("Equivalent coordinator action repeated without useful progress; adapt using the retained outcome".into());
            }
            match decision.action.as_str() {
                "retrieve" => {
                    let summary=if frozen_packet["context_delivery"]["memory_version"]==2{&decision.context_summary}else{&decision.assistant_text};
                    let exchange=crate::development_context::retrieve_with_memory(s,&dir,&frozen_packet,&decision.evidence_requests,&w["evidence_context"],summary,&json!([]),&decision.release_evidence)?;
                    w["evidence_context"]=crate::development_context::reference(&dir,&exchange)?;
                    records.push(("context_exchange".into(),format!("{key}-{op}"),json!({"work_id":key,"request_id":w["request_id"],"operation_id":op,"packet_sha256":hash(&frozen_packet)?,"requests":decision.evidence_requests,"response":crate::development_context::audit(&w["evidence_context"]),"evidence":dir.join("context-exchange.json")}),"development.evidence_retrieved".into()));
                    w["last_context_sha256"]=Value::Null;
                    w["evidence_pending"]=json!(true);
                }
                "assign" => {
                    if decision.tasks.is_empty() {
                        return Err("Empty assignment makes no progress".into());
                    }
                    let mut existing = units(s, w)?;
                    for spec in decision.tasks {
                        if !id(&spec.id)
                            || spec.role!="review"&&!valid_paths(&spec.paths)
                            || spec.instructions.trim().is_empty()
                            || spec.instructions.len() > 16000
                        {
                            return Err("Invalid assignment identity, paths or instructions".into());
                        }
                        if !matches!(spec.role.as_str(),"implementation"|"review"){return Err("Unknown task role".into());}
                        if !crate::provider_selection::enabled(s)&&(spec.role!="implementation"||!spec.profile.is_empty()){return Err("Role routing requires state9".into());}
                        if spec.role=="review"&&(spec.review_targets.is_empty()||!spec.paths.is_empty()){return Err("Review tasks require targets and no write paths".into());}
                        if !spec.resolution_targets.is_empty()&&(frozen_packet["recovery_contract"]!=1||spec.role!="review"||spec.resolution_targets.iter().any(|name|!existing.iter().any(|u|u["name"]==*name&&u["role"]=="review"&&u["status"]=="reviewed"&&u["requirement_generation"]==w["requirement_generation"]))){return Err("Finding clarification must link settled current-generation reviews".into());}
                        if crate::provider_selection::enabled(s){
                            let (prefs,rev)=crate::provider_selection::preferences(s)?;
                            prefs.resolve(&spec.role,&Value::Null,(!spec.profile.is_empty()).then_some(spec.profile.as_str()),&crate::provider_selection::normalized_policy(s,&w["policy"])?,rev,&spec.routing_reason)?;
                            for target in &spec.review_targets{if !existing.iter().any(|u|u["name"]==*target&&u["role"]!="review"){return Err("Unknown review target".into());}}
                        }
                        let input_fingerprint = hash(&json!([
                            w["scope_base"],
                            w["requirement_generation"],
                            w["recoveries"],
                            w["head"],
                            spec.instructions,
                            spec.paths,
                            spec.start_after,
                            spec.accept_after
                        ]))?;
                        if existing
                            .iter()
                            .any(|u| u["input_fingerprint"] == input_fingerprint)
                        {
                            return Err("Equivalent assignment has identical source, requirements and approach; adapt using retained outcome instead of duplicating work".into());
                        }
                        let uid = format!("{key}-{}", spec.id);
                        if existing.iter().any(|u| u["name"] == spec.id) {
                            return Err("Duplicate assignment ID; use a new repair/follow-up task preserving previous evidence".into());
                        }
                        for dep in spec.start_after.iter().chain(spec.accept_after.iter()) {
                            if !existing.iter().any(|u| u["name"] == *dep) {
                                return Err(
                                    "Task dependency must name a saved prior assignment".into()
                                );
                            }
                        }
                        validate_write_ownership(&spec.paths, &spec.start_after, &existing)?;
                        let mut u = json!({"schema_version":1,"unit_id":uid,"input_fingerprint":input_fingerprint,"name":spec.id,"work_id":key,"request_id":w["request_id"],"requirement_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"title":spec.title,"instructions":spec.instructions,"role":spec.role,"requested_profile":spec.profile,"routing_reason":spec.routing_reason,"review_target_names":spec.review_targets,"resolution_target_names":spec.resolution_targets,"profile_override":{"mode":"inherit"},"paths":spec.paths,"start_after":spec.start_after,"accept_after":spec.accept_after,"obligations":spec.obligations,"provider":"codex","model":w["profile"]["model"],"effort":w["profile"]["effort"],"branch_namespace":w["branch_namespace"].as_str().map(str::to_owned).unwrap_or_else(||format!("waywr/{key}")),"branch":format!("{}/tasks/{}/work",w["branch_namespace"].as_str().map(str::to_owned).unwrap_or_else(||format!("waywr/{key}")),spec.id),"worktree":worktree_path(s,&key,&spec.id)?,"base_commit":null,"result_commit":null,"status":"planned","attempt_revision":1});
                        if crate::provider_selection::enabled(s){
                            let (prefs,revision)=crate::provider_selection::preferences(s)?;
                            let selection=prefs.resolve(&spec.role,&Value::Null,(!spec.profile.is_empty()).then_some(spec.profile.as_str()),&crate::provider_selection::normalized_policy(s,&w["policy"])?,revision,&spec.routing_reason)?;
                            u["provider"]=selection["profile"]["provider"].clone();u["model"]=selection["profile"]["model"].clone();u["effort"]=selection["profile"]["effort"].clone();u["planned_selection"]=selection;
                        }
                        let contract_revision = w["scope_base"].clone();
                        for obligation in &spec.obligations {
                            if !w["obligations"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|o| o["name"] == *obligation)
                            {
                                w["obligations"].as_array_mut().unwrap().push(json!({"name":obligation,"owner":uid,"status":"open","contract_revision":contract_revision,"resolution":null}));
                            }
                        }
                        records.push((
                            "unit".into(),
                            uid.clone(),
                            u.clone(),
                            "development.assignment_planned".into(),
                        ));
                        existing.push(u);
                    }
                    w["last_context_sha256"] = Value::Null;
                }
                "integrate" => {
                    if decision.units.len() != 1 {
                        return Err(
                        "Select exactly one frozen submission per serialized integration intent"
                            .into(),
                    );
                    }
                    let all = units(s, w)?;
                    let u = all
                        .iter()
                        .find(|u| {
                            u["name"] == decision.units[0] || u["unit_id"] == decision.units[0]
                        })
                        .ok_or("Unknown integration task")?;
                    if u["status"] != "submitted" || !available(&all, &u["accept_after"]) {
                        return Err("Submission or integration dependencies are not ready".into());
                    }
                    require_frozen_review(&frozen_packet, u)?;
                    crate::request_routing::require_review(&w["policy"],u,&all)?;
                    if u["requirement_base"] != w["scope_base"]
                        || u["requirement_generation"] != w["requirement_generation"]
                    {
                        return Err("Obsolete submission cannot integrate".into());
                    }
                    let effect_id = format!(
                        "integration-{}-{}",
                        u["name"].as_str().unwrap(),
                        &strv(u, "result_commit")?[..12]
                    );
                    let contained = git(
                        Path::new(strv(w, "integration_worktree")?),
                        &[
                            "merge-base",
                            "--is-ancestor",
                            strv(u, "result_commit")?,
                            strv(w, "head")?,
                        ],
                    )
                    .is_ok();
                    let e = json!({"schema_version":1,"kind":if contained{"contained"}else{"merge"},"effect_id":effect_id,"work_id":key,"request_id":w["request_id"],"status":"intent","expected_head":w["head"],"submitted_commit":u["result_commit"],"unit_id":u["unit_id"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"review":{"operation_id":op,"reason":decision.reason,"result_sha256":hash(&result)?,"packet_sha256":done["packet_sha256"],"reviewed_commit":u["result_commit"]},"owner_generation":self.generation,"result_commit":null});
                    records.push((
                        "effect".into(),
                        effect_id,
                        e,
                        "development.integration_intent".into(),
                    ));
                    w["last_context_sha256"] = Value::Null;
                }
                "retire" => {
                    if frozen_packet["convergence_contract"]!=1{return Err("Retirement is unavailable in this frozen decision contract".into())}
                    let current=units(s,w)?;
                    for name in &decision.units {
                        let u=current.iter().find(|u|u["name"]==*name).ok_or("Unknown retirement target")?;
                        if !frozen_packet["units"].as_array().is_some_and(|all|all.iter().any(|f|f["unit_id"]==u["unit_id"]&&f["source_record"]["sha256"]==hash(u).unwrap_or_default())) {
                            return Err("Retirement assignment changed since the frozen coordinator packet; reassess its current state".into());
                        }
                    }
                    for u in retire_assignments(s,w,&current,&decision.units,op,&decision.reason)? {
                        records.push(("unit".into(),strv(&u,"unit_id")?.into(),u,"development.assignment_retired".into()));
                    }
                    w["last_context_sha256"]=Value::Null;
                }
                "candidate"=>{
                    if let Some(candidate)=crate::candidate::prepare(s,w,&decision.delivery,op)?{records.push(("candidate".into(),strv(&candidate,"candidate_id")?.into(),candidate,"development.candidate_ready".into()));}
                    w["last_context_sha256"]=Value::Null;
                }
                "refresh_target"=>{
                    if !w["policy"]["project_workflow"].is_object()||crate::candidate::writes_held(w){return Err("Target refresh requires ordinary policy and service checkout ownership".into())}
                    let target=crate::project_workflow::ref_head(&s.project,strv(w,"main_branch")?)?;
                    if w["target_base_commit"]==target{return Err("Target has not advanced; no repeated merge".into())}
                    git(&s.project,&["merge-base","--is-ancestor",strv(w,"target_base_commit")?,&target]).map_err(|_|"Target moved non-fast-forward; preserve both histories and obtain a substantive baseline decision before changing integration authority")?;
                    let id=format!("target-{}",&target[..12]);let e=json!({"schema_version":1,"effect_id":id,"work_id":key,"request_id":w["request_id"],"kind":"target-merge","status":"intent","expected_head":w["head"],"submitted_commit":target,"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"owner_generation":self.generation});records.push(("effect".into(),id,e,"development.integration_intent".into()));w["last_context_sha256"]=Value::Null;
                }
                "check" => self.prepare_development_checks(w, decision.checks)?,
                "complete" => {
                    if w["checks"]["commit"] != w["head"] || w["checks"]["result"]["passed"] != true
                    {
                        return Err("Completion requires successful combined checks on the exact integration commit".into());
                    }
                    let checks = w["checks"]["result"]["checks"]
                        .as_array()
                        .ok_or("Missing check records")?;
                    if checks.is_empty() || checks.iter().any(|c| c["success"] != true) {
                        return Err("Completion requires nonempty successful project-appropriate combined check receipts".into());
                    }
                    let all = units(s, w)?;
                    for unit in all.iter().filter(|u|matches!(u["status"].as_str(),Some("integrated"|"accepted"))){crate::request_routing::require_review(&w["policy"],unit,&all)?;}
                    if all.iter().any(|u| {
                        matches!(
                            u["status"].as_str(),
                            Some(
                                "running" | "planned" | "ready" | "submitted" | "capacity-waiting"
                            )
                        )
                    }) {
                        return Err("Completion cannot conceal outstanding assignments".into());
                    }
                    if w["policy"]["project_workflow"].is_object(){
                        if let Some(candidate)=crate::candidate::prepare(s,w,&decision.delivery,op)?{records.push(("candidate".into(),strv(&candidate,"candidate_id")?.into(),candidate,"development.candidate_ready".into()));}
                        if w["policy"]["project_workflow"]["human_validation"]==true&&!crate::candidate::accepted(w){w["last_context_sha256"]=Value::Null;return Ok(())}
                    }
                    for name in &decision.resolved_obligations {
                        if name=="policy:human-validation"&&!crate::candidate::accepted(w){continue}
                        let evidence = w["checks"].clone();
                        for o in w["obligations"].as_array_mut().unwrap() {
                            if o["name"] == *name {
                                o["status"] = json!("resolved");
                                o["resolution"] = json!({"commit":evidence["commit"],"checks":evidence["evidence"],"review":op,"rationale":decision.reason});
                            }
                        }
                    }
                    if w["obligations"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|o| o["status"] != "resolved")
                    {
                        return Err("Open production integration/placeholder obligations prevent feature completion".into());
                    }
                    w["summary"] = json!({"label":"coordinator narrative / opinion","text":decision.assistant_text,"rationale":decision.reason,"commit":w["head"],"source_operation":op});
                    // Promotion itself has a saved durable intent, including expected local main.
                    let effect_id = format!("promote-{}", &strv(w, "head")?[..12]);
                    let e = json!({"schema_version":1,"effect_id":effect_id,"work_id":key,"request_id":w["request_id"],"kind":if w["policy"]["project_workflow"]["completion"]=="github-pr"{"github-pr"}else{"promote"},"status":"intent","expected_head":w["target_base_commit"].as_str().or(w["base_commit"].as_str()),"submitted_commit":w["head"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"owner_generation":self.generation,"result_commit":null});
                    records.push((
                        "effect".into(),
                        effect_id,
                        e,
                        "development.promotion_intent".into(),
                    ));
                }
                "wait" => {
                    let all=units(s,w)?;
                    if !all.iter().any(|u| {
                        matches!(
                            u["status"].as_str(),
                            Some("running" | "capacity-waiting")
                        ) || (matches!(u["status"].as_str(),Some("planned"|"ready"))
                            && (!u["recovery"].is_object()||u["recovery"]["phase"]=="resolved")
                            && start_dependencies_ready(u,&all))
                    }) {
                        return Err("Coordinator requested wait without an outstanding producer; revise the approach".into());
                    }
                }
                "decision" => {
                    w["status"] = json!("attention");
                    w["attention"] = json!(decision.reason);
                    w["question"] = json!({"prompt":decision.assistant_text,"reason":decision.reason,"source_operation":op,"scope_base":w["scope_base"]});
                    if crate::provider_selection::enabled(s){w["recovery"]=recovery::observe(w,Cause::ProductDecision,&key,op,&decision.reason,json!({"question":w["question"]}))?;recovery::retain_contract(s)?;}
                }
                _ => return Err("Unknown coordinator action".into()),
            }
            Ok(())
        })();
        if let Err(reason) = adoption {
            *w = before_action;
            w["pending"] = Value::Null;
            // Rejected decisions have not consumed the active evidence; retain it
            // for autonomous correction rather than forcing another read chain.
            if frozen_packet["context_delivery"]["memory_version"]==2 && !w["evidence_context"].is_null(){w["evidence_pending"]=json!(true);}
            if crate::provider_selection::enabled(s)&&(reason.contains("needs explicit provider-policy approval")||reason.starts_with("Task override includes an unapproved")){
                w["status"]=json!("attention");w["provider_blocked"]=json!({"kind":"policy","source_operation":op});w["attention"]=json!(reason);
                w["last_context_sha256"]=Value::Null;bump(w);s.development_save("workflow",&key,w,"providers.policy_needed")?;return Ok(());
            }
            w["attention"] = json!({"kind":"coordinator action rejected; automatic replanning","reason":reason,"source_operation":op});
            w["last_context_sha256"] = Value::Null;
            bump(w);
            s.development_save("workflow", &key, w, "development.decision_rejected")?;
            return Ok(());
        }
        bump(w);
        s.development_commit_decision(w, &records, op)?;
        if !crate::provider_selection::enabled(s)||s.development_record("workflow",&key)?.is_some_and(|v|v["interrupted_coordinator"]["operation_id"]!=op){s.development_say(w, &narrative)?;}
        Ok(())
    }
    fn prepare_development_checks(
        &self,
        w: &mut Value,
        checks: Vec<Vec<String>>,
    ) -> Result<(), String> {
        let s = &self.session;
        let key = strv(w, "work_id")?.to_owned();
        if checks.is_empty() {
            return Err("No combined check commands selected".into());
        }
        let seq = w["coordinator_sequence"].as_u64().unwrap() + 1;
        let op = format!("checks-{seq}");
        let packet = json!({"schema_version":1,"development":true,"operation_kind":"development-check","operation_id":op,"run_id":key,"stage":"checks","role":"checks","provider":"local","model":"local","project_key":s.key,"request_id":w["request_id"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"policy":w["policy"],"worktree":w["integration_worktree"],"checked_commit":w["head"],"checks":checks,"generated_resources":w["generated_resources"]});
        operation::prepare_host(&s.workflow.state, &op_dir(s, &key, &op), &packet, &s.key)
            .map_err(|e| e.detail)?;
        w["pending"] = json!({"operation_id":op,"kind":"check","started_at":now(),"checked_commit":w["head"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"]});
        w["coordinator_sequence"] = json!(seq);
        Ok(())
    }
    pub(crate) fn apply_development_effect(
        &self,
        w: &mut Value,
        e: &mut Value,
    ) -> Result<(), String> {
        let s = &self.session;
        let key = strv(w, "work_id")?.to_owned();
        let eid = strv(e, "effect_id")?.to_owned();
        let _lock = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
        let _fence = FileLock::exclusive(&s.workflow.state.join("supervision/fence.lock"))?;
        if read(&s.workflow.state.join("supervision/owner.json"))?["generation"] != self.generation
            || s.phase()?["phase"] != "running"
            || !authority(s, w)?
            || e["scope_base"] != w["scope_base"]
            || e["requirement_generation"] != w["requirement_generation"]
        {
            return Err("Git effect fenced by owner or current requirements".into());
        }
        if crate::candidate::writes_held(w){return Err("Candidate checkout is owned for validation; feedback/end-validation must reconcile and release it before Git writes".into())}
        if e["kind"]=="github-pr"{if e["retry_at"].as_f64().is_some_and(|t|t>now()){return Ok(())}if let Err(reason)=crate::project_publication::tick(s,w,e){e["retry_at"]=json!(now()+30.0);e["publication_error"]=json!(reason);s.development_save("effect",strv(e,"effect_id")?,e,"development.publication_wait")?;return Err(reason)}return Ok(())}
        let promote = e["kind"] == "promote";
        let ordinary_target=promote&&w["policy"]["project_workflow"].is_object();
        let target_owner=if ordinary_target{crate::project_workflow::target_checkout(&s.project,strv(w,"main_branch")?)?}else{None};
        let tree = if promote {target_owner.clone().unwrap_or(s.project.clone())} else {PathBuf::from(strv(w,"integration_worktree")?)};
        let expected = strv(e, "expected_head")?.to_owned();
        let submitted = strv(e, "submitted_commit")?.to_owned();
        let head = if ordinary_target{crate::project_workflow::ref_head(&s.project,strv(w,"main_branch")?)?}else{git(&tree,&["rev-parse","HEAD"])?};
        let branch = if promote {
            strv(w, "main_branch")?
        } else {
            strv(w, "integration_branch")?
        }
        .to_owned();
        if (!ordinary_target||target_owner.is_some())&&git(&tree, &["symbolic-ref", "--short", "HEAD"])? != branch {
            return Err("Git effect branch changed externally; retained for reconciliation".into());
        }
        let marker = op_dir(s, &key, &eid).join("git-may-have-started.json");
        if head != expected {
            let applied = if promote {
                head == submitted
            } else {
                git(&tree, &["log", "-1", "--format=%B"])?.contains(&format!("Waywr-Intent: {eid}"))
                    && git(&tree, &["rev-parse", "HEAD^1"])? == expected
                    && git(&tree, &["rev-parse", "HEAD^2"])? == submitted
            };
            if !applied {
                return Err("Git head diverged from saved integration intent; no replay".into());
            }
            if !ordinary_target||target_owner.is_some(){clean(&tree)?;}
        } else {
            if !ordinary_target||target_owner.is_some(){clean(&tree)?;}
            if tree.join(".git/MERGE_HEAD").exists()
                || git(&tree, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_ok()
            {
                return Err("Unresolved merge retained; explicit reconciliation required".into());
            }
            if e["kind"] == "contained" {
                git(&tree, &["merge-base", "--is-ancestor", &submitted, &head])?;
            } else {
                if marker.exists() {
                    return Err("Git mutation may have started without a resulting commit; preserved for reconciliation, no blind replay".into());
                }
                write(
                    &marker,
                    &json!({"effect_id":eid,"expected_head":expected,"submitted_commit":submitted,"owner_generation":self.generation}),
                    true,
                )?;
                if promote {
                    if w["checks"]["commit"] != submitted || w["checks"]["result"]["passed"] != true
                    {
                        return Err("Main advancement lacks exact checked revision".into());
                    }
                    if ordinary_target{crate::project_workflow::advance_target(&s.project,&branch,&expected,&submitted)?;}else{git(&tree, &["merge", "--ff-only", &submitted])?;}
                } else {
                    if e["kind"]!="target-merge"{git(
                        &tree,
                        &[
                            "merge-base",
                            "--is-ancestor",
                            strv(
                                &s.development_record("unit", strv(e, "unit_id")?)?
                                    .ok_or("Unit missing")?,
                                "base_commit",
                            )?,
                            &submitted,
                        ],
                    )?;}
                    git(
                        &tree,
                        &[
                            "-c",
                            "user.name=Waywr",
                            "-c",
                            "user.email=waywr@localhost",
                            "merge",
                            "--no-ff",
                            "--no-edit",
                            "-m",
                            &format!("Integrate frozen worker submission\n\nWaywr-Intent: {eid}"),
                            &submitted,
                        ],
                    )?;
                }
            }
        }
        let actual = if ordinary_target{crate::project_workflow::ref_head(&s.project,&branch)?}else{git(&tree, &["rev-parse", "HEAD"])?};
        e["result_commit"] = json!(actual);
        e["status"] = json!("applied");
        e["reconciled_generation"] = json!(self.generation);
        s.development_save("effect", &eid, e, "development.git_applied")?;
        if promote {
            w["delivery"]=json!({"kind":"checked-local","target":branch,"commit":actual,"candidate_id":w["candidate"]["candidate_id"]});
            w["status"] = json!("delivered");
            w["delivered_commit"] = json!(actual);
            w["attention"] = Value::Null;
            for mut u in units(s, w)? {
                if u["status"] == "integrated" {
                    u["status"] = json!("accepted");
                    u["acceptance_commit"] = w["head"].clone();
                    s.development_save(
                        "unit",
                        strv(&u, "unit_id")?,
                        &u,
                        "development.unit_accepted",
                    )?;
                }
            }
            s.development_say(w,&format!("Delivered at local {} commit {}. Combined checks passed on this exact revision. Request history: {}",branch,actual,crate::project_layout::history_root(&s.project,&s.workflow.state).display()))?;
        } else {
            w["head"] = json!(actual);
            w["checks"] = Value::Null;
            w["last_context_sha256"] = Value::Null;
            if e["kind"]=="target-merge"{w["target_base_commit"]=e["submitted_commit"].clone();w["target_observation"]=Value::Null;w["candidate"]=Value::Null;}else{
            let mut u = s
                .development_record("unit", strv(e, "unit_id")?)?
                .ok_or("Unit missing")?;
            u["status"] = json!("integrated");
            u["integration_commit"] = w["head"].clone();
            u["review"] = e["review"].clone();
            s.development_save(
                "unit",
                strv(&u, "unit_id")?,
                &u,
                "development.unit_integrated",
            )?;
            for target in units(s, w)?.iter().filter(|t| {
                t["start_after"]
                    .as_array()
                    .is_some_and(|ds| ds.contains(&u["name"]))
                    || t["accept_after"]
                        .as_array()
                        .is_some_and(|ds| ds.contains(&u["name"]))
            }) {
                let nid = format!(
                    "notice-{}",
                    &hash(&json!([
                        target["unit_id"],
                        u["unit_id"],
                        w["head"],
                        w["scope_base"]
                    ]))?[..24]
                );
                let n = json!({"schema_version":1,"work_id":key,"request_id":w["request_id"],"notification_id":nid,"recipient":target["unit_id"],"producer":u["unit_id"],"contract_revision":w["scope_base"],"accepted_commit":w["head"],"changes":u["diff"],"state":"pending-safe-point","policy":"running packet unchanged; apply only at a clean quiescent checkpoint or in a follow-up assignment"});
                s.development_save(
                    "notification",
                    &nid,
                    &n,
                    "development.dependency_notification",
                )?;
            }
        }
        }
        if !promote {
            for mut notice in s
                .development_records("notification")?
                .into_iter()
                .filter(|n| n["recipient"] == e["unit_id"] && n["state"] == "pending-safe-point")
            {
                notice["state"] = json!("combined-integration; behavioral acceptance pending");
                notice["combined_commit"] = w["head"].clone();
                s.development_save(
                    "notification",
                    strv(&notice, "notification_id")?,
                    &notice,
                    "development.dependency_combined",
                )?;
            }
        }
        if !authority(s, w)? {
            return Err(
                "Steering won during Git mutation; resulting commit retained without acceptance"
                    .into(),
            );
        }
        let knowledge = json!({"schema_version":1,"work_id":key,"request_id":w["request_id"],"source_commit":w["head"],"requirement_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"source_map":git(Path::new(strv(w,"integration_worktree")?),&["ls-tree","-r","--name-only",strv(w,"head")?])?,"contracts":source_contracts(Path::new(strv(w,"integration_worktree")?),strv(w,"head")?)?,"conventions":"Follow approved request, project instructions and detected toolchain; no application-specific sequence","checks":w["checks"],"open_obligations":w["obligations"],"next_steps":if promote{"Delivered; new work starts a new request"}else{"Review/check integrated changes and remaining obligations"}});
        s.development_save(
            "knowledge",
            &key,
            &knowledge,
            "development.knowledge_updated",
        )?;
        write(
            &s.workflow
                .state
                .join("knowledge")
                .join(format!("{key}.json")),
            &knowledge,
            false,
        )?;
        bump(w);
        s.development_save(
            "workflow",
            &key,
            w,
            if promote {
                "development.delivered"
            } else {
                "development.integration_recorded"
            },
        )?;
        e["status"] = json!("recorded");
        s.development_save("effect", &eid, e, "development.integration_complete")?;
        Ok(())
    }
}

fn validate_check_argv(tree: &Path, argv: &[String]) -> Result<(), String> {
    let Some(program) = argv.first() else {
        return Err("Empty check command".into());
    };
    if program == "cargo"
        || std::env::var("HOME")
            .ok()
            .is_some_and(|h| program == &format!("{h}/.cargo/bin/cargo"))
    {
        return if argv.get(1).is_some_and(|s| {
            matches!(
                s.as_str(),
                "test" | "build" | "check" | "clippy" | "fmt" | "run"
            )
        }) {
            Ok(())
        } else {
            Err("Check policy permits cargo build/test/check/clippy/fmt/run, not installation or publication".into())
        };
    }
    if matches!(program.as_str(), "bash" | "sh" | "python3") {
        let index = if program == "python3" { 1 + argv.iter().skip(1).take_while(|arg| arg.as_str() == "-B").count() } else { 1 };
        let script = argv
            .get(index)
            .filter(|s| !s.starts_with('-'))
            .ok_or("Use a tracked project check script; shell -c is not a check grant")?;
        let path = tree
            .join(script)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(tree.canonicalize().map_err(|e| e.to_string())?) {
            return Err("Check script leaves integration worktree".into());
        }
        git(tree, &["ls-files", "--error-unmatch", "--", script])?;
        return Ok(());
    }
    let path = Path::new(program);
    if !path.is_absolute()
        && path
            .components()
            .all(|p| matches!(p, Component::Normal(_) | Component::CurDir))
        && program.starts_with("./")
    {
        let actual = tree.join(path).canonicalize().map_err(|e| e.to_string())?;
        if actual.starts_with(tree.canonicalize().map_err(|e| e.to_string())?) {
            return Ok(());
        }
    }
    Err("Check command is outside the approved project build/test/script policy".into())
}
/// Executed as a child inside the existing supervised operation group. No model involved.
pub fn check_child(packet_path: &Path) -> Result<(), String> {
    use std::io::Write;
    let packet = read(packet_path)?;
    let tree = Path::new(strv(&packet, "worktree")?);
    let expected = strv(&packet, "checked_commit")?;
    if git(tree, &["rev-parse", "HEAD"])? != expected {
        return Err("Checked revision differs before execution".into());
    }
    let evidence=packet_path.parent().ok_or("Missing check directory")?;
    crate::project_resources::begin_generated(tree,&packet["policy"],evidence,&packet["generated_resources"])?;
    let mut results = vec![];
    let mut passed = true;
    for args in packet["checks"]
        .as_array()
        .ok_or("Missing check commands")?
    {
        let argv = args
            .as_array()
            .ok_or("Invalid check argv")?
            .iter()
            .map(|a| {
                a.as_str()
                    .map(str::to_owned)
                    .ok_or("Non-string check arg".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if argv.is_empty() || argv.len() > 100 || argv.iter().any(|a| a.len() > 8192) {
            return Err("Invalid check command".into());
        }
        let started = now();
        if let Err(reason) = if crate::project_knowledge::recipe_allowed(&packet["policy"],&argv){Ok(())}else{validate_check_argv(tree,&argv)} {
            passed = false;
            results.push(json!({"argv":argv,"success":false,"refused_before_execution":true,"reason":reason}));
            continue;
        }
        let output = Command::new(&argv[0])
            .args(&argv[1..])
            .current_dir(tree)
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
            .map_err(|e| e.to_string())?;
        let evidence_dir = packet_path
            .parent()
            .ok_or("Missing check directory")?
            .join("checks");
        fs::create_dir_all(&evidence_dir).map_err(|e| e.to_string())?;
        let index = results.len();
        let out = evidence_dir.join(format!("{index}.stdout"));
        let err = evidence_dir.join(format!("{index}.stderr"));
        write_bytes(&out, &output.stdout, true)?;
        write_bytes(&err, &output.stderr, true)?;
        passed &= output.status.success();
        let result = json!({"argv":argv,"success":output.status.success(),"exit_code":output.status.code(),"started":request_history::timestamp(started),"finished":request_history::timestamp(now()),"stdout":out,"stderr":err,"stdout_sha256":sha256(&output.stdout),"stderr_sha256":sha256(&output.stderr),"tail":String::from_utf8_lossy(&output.stderr).chars().rev().take(4000).collect::<String>().chars().rev().collect::<String>()});
        println!("{}", json!({"type":"check.completed","check":result}));
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        results.push(result);
    }
    crate::project_resources::finish_generated(tree,&packet["policy"],evidence)?;
    println!(
        "{}",
        json!({"type":"result","result":{"schema_version":1,"checked_commit":expected,"checks":results,"passed":passed}})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context_packet(s:&Session,w:&Value)->Value {
        json!({"schema_version":1,"development":true,"operation_kind":"development-coordinator","role":"coordinator","operation_id":"context-test","run_id":w["work_id"],"project_key":s.key,"request_id":w["request_id"],"provider":"codex","model":"gpt-6.1-sol","execution":{"profile":"development-profiles-v1","provider":"codex","model":"gpt-6.1-sol","context_bytes":98304},"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"workflow":w,"policy":w["policy"],"requirements":{"objective":"Preserve the actual approved requirements"},"knowledge":[]})
    }
    #[test]
    fn context_compaction_preserves_authority_findings_and_accepts_required_input_above_96k() {
        let (_d,service,mut w)=fixture();let s=&service.session;
        w["last_coordinator_result"]=json!({"repeated":"old result".repeat(30000)});
        w["obligations"]=json!([{"name":"Unresolved production behavior","status":"open","resolution":null}]);
        s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
        let uid=format!("{}-review",w["work_id"].as_str().unwrap());let findings=json!(["Concrete unresolved regression".repeat(100)]);
        let u=json!({"unit_id":uid,"work_id":w["work_id"],"status":"reviewed","role":"review","review":{"summary":"Changes required","requires_changes":true,"findings":findings},"receipt":{"raw":"old output".repeat(40000)}});
        s.development_save("unit",&uid,&u,"fixture").unwrap();
        let mut packet=context_packet(s,&w);packet["requirements"]["mandatory"]=json!("r".repeat(110000));
        packet["assignment"]=json!({"started_at":1792258300.7197313_f64,"instructions":"Required exact frozen input"});
        let policy=packet["policy"].clone();let dir=op_dir(s,w["work_id"].as_str().unwrap(),"context-compact");
        crate::development_context::prepare(s,&dir,&mut packet).unwrap();
        assert_eq!(packet["policy"],policy);assert!(packet["workflow"]["policy"].is_null());
        assert_eq!(packet["requirements"]["mandatory"].as_str().unwrap().len(),110000);
        assert_eq!(packet["units"][0]["unresolved_findings"],findings);
        assert_eq!(packet["workflow"]["obligations"],w["obligations"]);
        assert!(serde_json::to_vec(&packet).unwrap().len()>98304);
        assert!(serde_json::to_vec(&packet).unwrap().len()<150000);
        assert_eq!(s.development_record("unit",&uid).unwrap().unwrap(),u);
        let diag=read(&dir.join("context-diagnostics.json")).unwrap();assert_eq!(diag["retrieval_available"],true);assert!(diag["token_estimate"]["method"].as_str().unwrap().contains("Heuristic"));
        operation::prepare_host(&s.workflow.state,&dir,&packet,&s.key).unwrap();let frozen=fs::read(dir.join("packet.json")).unwrap();
        let mut changed=packet.clone();changed["requirements"]=json!({"changed":true});crate::development_context::prepare(s,&dir,&mut changed).unwrap();assert_eq!(fs::read(dir.join("packet.json")).unwrap(),frozen);
        assert!(!dir.join("active-host-attempt.json").exists());
        let attempt=Attempt::create(&s.workflow.state,&dir,&s.key,&std::env::current_exe().unwrap(),&s.release,service.generation).unwrap();
        assert!(attempt.directory.join("intent.json").exists());
        assert!(!attempt.directory.join("start-maybe-sent.json").exists());
        assert_eq!(fs::read(dir.join("packet.json")).unwrap(),frozen);
    }
    #[test]
    fn context_exact_revision_pages_are_read_only_complete_and_gate_final_review() {
        let (_d,service,mut w)=fixture();let s=&service.session;let tree=PathBuf::from(w["integration_worktree"].as_str().unwrap());
        let base=w["head"].as_str().unwrap().to_owned();let source="original immutable source\n".repeat(2200);
        let head=commit_files(&tree,&[("source.txt",&source)]);w["head"]=json!(head);s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
        let uid=format!("{}-implementation",w["work_id"].as_str().unwrap());let unit=json!({"unit_id":uid,"work_id":w["work_id"],"title":"Implementation","status":"submitted","base_commit":base,"result_commit":head});
        s.development_save("unit",&uid,&unit,"fixture").unwrap();let mut packet=context_packet(s,&w);packet["operation_kind"]=json!("development-review");packet["role"]=json!("review");packet["assignment"]=json!({"unit_id":"review","instructions":"Review exact source"});packet["review_targets"]=json!([unit]);packet["integration_head"]=w["head"].clone();
        packet["execution"]["context_bytes"]=json!(16384);
        let dir=op_dir(s,w["work_id"].as_str().unwrap(),"context-review");crate::development_context::prepare(s,&dir,&mut packet).unwrap();
        let final_review=json!({"schema_version":1,"summary":"Reviewed","reviewed_commits":[head],"findings":[],"requires_changes":false,"evidence_requests":[],"context_summary":"Exact diff reviewed","release_evidence":[]});assert!(validate_result(&final_review,&packet).unwrap_err().contains("diff not delivered"));
        let diff=packet["evidence_index"]["entries"].as_array().unwrap().iter().find(|e|e["kind"]=="diff").unwrap();let diff_id=diff["id"].as_str().unwrap().to_owned();let digest=diff["sha256"].as_str().unwrap().to_owned();let size=diff["bytes"].as_u64().unwrap() as usize;
        let mut requests=(0..size).step_by(16384).map(|offset|crate::development_context::Request{evidence_id:diff_id.clone(),path:String::new(),cursor:if offset==0{String::new()}else{format!("{digest}:{offset}")}}).collect::<Vec<_>>();
        requests.push(crate::development_context::Request{evidence_id:format!("source:{head}"),path:"source.txt".into(),cursor:String::new()});
        requests.push(crate::development_context::Request{evidence_id:format!("source:{head}"),path:"../outside".into(),cursor:String::new()});
        commit_files(&tree,&[("source.txt","changed after frozen grant\n")]);
        let exchange=crate::development_context::retrieve(s,&dir,&packet,&requests,&Value::Null,"Retained exact findings",&json!(["A concrete unresolved finding"])).unwrap();
        let source_page=exchange["pages"].as_array().unwrap().iter().find(|p|p["path"]=="source.txt").unwrap();assert!(source_page["data"].as_str().unwrap().starts_with("original immutable source"));
        assert_eq!(exchange["pages"].as_array().unwrap().last().unwrap()["no_bytes_supplied"],true);
        packet["evidence_pages"]=exchange["pages"].clone();assert!(crate::development_context::diff_delivered(&packet,&packet["review_targets"][0]));assert!(validate_result(&final_review,&packet).is_ok());
        let mut changed=packet["review_targets"][0].clone();changed["result_commit"]=json!("f".repeat(40));assert!(!crate::development_context::diff_delivered(&packet,&changed));
        assert_eq!(exchange["unresolved_findings"],json!(["A concrete unresolved finding"]));assert!(!dir.join("active-host-attempt.json").exists());
    }
    #[test]
    fn context_preparation_refusal_recovers_under_existing_approval_without_replaying_calls() {
        let (_d,service,mut w)=fixture();let s=&service.session;let approval=w["authorization"].clone();let scope=w["scope_base"].clone();let before=s.development_records("unit").unwrap();
        w["pending"]=Value::Null;w["status"]=json!("attention");w["attention"]=json!("Mandatory development packet is 104008 bytes, exceeding selected 98304-byte budget; no attempt launched");
        s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
        let out=s.development_command(&json!({"command_id":"recover-context-preparation","action":"development.resume","payload":{"work_id":w["work_id"],"revision":w["revision"],"recovery_note":"Context preparation corrected; preserve completed calls and exact existing authority"}})).unwrap();assert_eq!(out["status"],"accepted");
        let recovered=s.development_record("workflow",w["work_id"].as_str().unwrap()).unwrap().unwrap();assert_eq!(recovered["authorization"],approval);assert_eq!(recovered["scope_base"],scope);assert_eq!(recovered["status"],"running");assert_eq!(s.development_records("unit").unwrap(),before);
        let dir=op_dir(s,w["work_id"].as_str().unwrap(),"recovered-context");let mut packet=context_packet(s,&recovered);crate::development_context::prepare(s,&dir,&mut packet).unwrap();assert!(!dir.join("active-host-attempt.json").exists());
    }
    #[test]
    fn context_discussion_retrieval_continues_once_as_service_input_without_approval() {
        let (_d,service,w)=fixture();let s=&service.session;crate::coordination::schema(s).unwrap();
        let out=s.conversation_submit(&json!({"command_id":"ask-context","action":"submit_message","payload":{"subject_id":"discussion","task_id":w["work_id"],"text":"Inspect the relevant project evidence","intent":"discussion"}})).unwrap();assert_eq!(out["status"],"accepted");
        let next=s.conversation_next_queued().unwrap().unwrap();let mut profile=profile(s).unwrap();profile["profile"]=json!("development-profiles-v1");profile["context_bytes"]=json!(98304);
        let mut packet=s.scope_packet(&next,&profile,&json!(hash(&profile).unwrap())).unwrap();let turn=next["turn_id"].as_str().unwrap();
        let dir=s.workflow.state.join("session/coordinator").join(turn).join("call-v1");crate::development_context::prepare(s,&dir,&mut packet).unwrap();
        let root=s.workflow.state.join("session/conversation").join(turn);write(&root.join("packet.json"),&packet,true).unwrap();let digest=hash(&packet).unwrap();assert_eq!(s.conversation_reserve(turn,&packet,"conversation").unwrap()["status"],"accepted");
        let approvals:i64=s.workflow.connection().unwrap().query_row("SELECT count(*) FROM task_scope_decisions",[],|r|r.get(0)).unwrap();
        let result=json!({"schema_version":3,"assistant_text":"Reading the exact saved workflow before answering","evidence_refs":[],"action":{"kind":"retrieve_evidence","payload":{"requests":[{"evidence_id":"workflow","path":"","cursor":""}]}}});
        assert_eq!(s.conversation_apply(turn,&digest,&result).unwrap()["status"],"accepted");assert_eq!(s.conversation_apply(turn,&digest,&result).unwrap()["replayed"],true);
        let c=s.workflow.connection().unwrap();let queued:i64=c.query_row("SELECT count(*) FROM conversation_turns WHERE status='queued'",[],|r|r.get(0)).unwrap();assert_eq!(queued,1);
        let role:String=c.query_row("SELECT m.role FROM conversation_turns t JOIN conversation_messages m USING(message_id) WHERE t.status='queued'",[],|r|r.get(0)).unwrap();assert_eq!(role,"system");
        assert_eq!(c.query_row("SELECT count(*) FROM task_scope_decisions",[],|r|r.get::<_,i64>(0)).unwrap(),approvals);
        let continuation=s.conversation_next_queued().unwrap().unwrap();let mut continued=s.scope_packet(&continuation,&profile,&json!(hash(&profile).unwrap())).unwrap();let next_dir=s.workflow.state.join("session/coordinator").join(continuation["turn_id"].as_str().unwrap()).join("call-v1");crate::development_context::prepare(s,&next_dir,&mut continued).unwrap();assert!(!continued["evidence_pages"].as_array().unwrap().is_empty());assert_eq!(continued["working_summary"]["source_request"]["text"],"Inspect the relevant project evidence");
    }
    #[test]
    fn context_history_cursor_and_compact_delivery_receipts_survive_continuation() {
        let (_d,service,mut w)=fixture();let s=&service.session;
        let request=crate::request_history::requests(s).unwrap().into_iter().find(|r|r["request_id"]==w["request_id"]).unwrap();let path=PathBuf::from(request["log_path"].as_str().unwrap());fs::create_dir_all(path.parent().unwrap()).unwrap();let original="original public history\n".repeat(2000);fs::write(&path,&original).unwrap();
        let mut packet=context_packet(s,&w);let dir=op_dir(s,w["work_id"].as_str().unwrap(),"history-context-1");crate::development_context::prepare(s,&dir,&mut packet).unwrap();
        let request=crate::development_context::Request{evidence_id:"history".into(),path:String::new(),cursor:String::new()};let first=crate::development_context::retrieve(s,&dir,&packet,&[request],&Value::Null,"Read initial page",&json!(["Unresolved historical finding"])).unwrap();let cursor=first["pages"][0]["next_cursor"].as_str().unwrap().to_owned();assert!(!cursor.is_empty());
        w["evidence_pending"]=json!(true);w["evidence_context"]=crate::development_context::reference(&dir,&first).unwrap();s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();fs::write(&path,"new current history\n").unwrap();
        let mut second=context_packet(s,&w);let dir2=op_dir(s,w["work_id"].as_str().unwrap(),"history-context-2");crate::development_context::prepare(s,&dir2,&mut second).unwrap();assert_eq!(second["evidence_pages"][0]["sha256"],first["pages"][0]["sha256"]);
        let next=crate::development_context::Request{evidence_id:"history".into(),path:String::new(),cursor};let exchange=crate::development_context::retrieve(s,&dir2,&second,&[next],&w["evidence_context"],"Continue exact snapshot",&json!([])).unwrap();
        assert_eq!(exchange["pages"][0]["sha256"],first["pages"][0]["sha256"]);assert_eq!(exchange["pages"][0]["data"].as_str().unwrap(),&original[16384..32768]);
        assert_eq!(exchange["delivered"].as_array().unwrap().len(),1);assert_eq!(exchange["delivered"][0]["ranges"],json!([[0,16384]]));assert_eq!(exchange["unresolved_findings"],json!(["Unresolved historical finding"]));
        let forged=crate::development_context::Request{evidence_id:"source:outside-project".into(),path:"secret".into(),cursor:String::new()};let mut third=second.clone();third["operation_id"]=json!("forged-read");let dir3=op_dir(s,w["work_id"].as_str().unwrap(),"history-context-3");write(&dir3.join("context-evidence.json"),&read(&dir2.join("context-evidence.json")).unwrap(),true).unwrap();let denied=crate::development_context::retrieve(s,&dir3,&third,&[forged],&Value::Null,"Read denied",&json!([])).unwrap();assert_eq!(denied["pages"][0]["no_bytes_supplied"],true);
    }
    #[test]
    fn recovery_releases_superseded_ownership_but_fences_unsettled_writers() {
        let paths=vec!["example_app/model.py".to_owned()];
        let retained=json!({"name":"original","status":"superseded","paths":["example_app","tests"],"result_commit":"d".repeat(40)});
        let bytes=retained.to_string();
        validate_write_ownership(&paths,&[],&[retained.clone()]).unwrap();
        assert_eq!(retained.to_string(),bytes);
        let mut running=retained.clone();running["status"]=json!("running");running["supersede_pending"]=json!(true);
        assert!(validate_write_ownership(&paths,&[],&[running.clone()]).unwrap_err().contains("original (running)"));
        validate_write_ownership(&paths,&["original".into()],&[running]).unwrap();
        let current=json!({"name":"new-current","status":"planned","paths":["example_app/model.py"]});
        assert!(validate_write_ownership(&["example_app".into()],&[],&[retained.clone(),current.clone()]).is_err());
        validate_write_ownership(&["README.md".into()],&[],&[retained,current]).unwrap();
    }
    #[test]
    fn context_active_reads_survive_old_exchanges_and_explicit_release_without_losing_findings() {
        let (_d,service,mut w)=fixture();let s=&service.session;
        let mut packet=context_packet(s,&w);let dir=op_dir(s,w["work_id"].as_str().unwrap(),"memory-v1");
        crate::development_context::prepare(s,&dir,&mut packet).unwrap();
        // An already completed v1 result remains valid after an upgrade.
        packet["context_delivery"].as_object_mut().unwrap().remove("memory_version");
        let old_result=json!({"schema_version":1,"assistant_text":"Read workflow","action":"retrieve","tasks":[],"units":[],"checks":[],"resolved_obligations":[],"reason":"Need exact record","evidence_requests":[{"evidence_id":"workflow","path":"","cursor":""}]});
        validate_result(&old_result,&packet).unwrap();
        assert!(result_schema(&packet)["properties"]["context_summary"].is_null());
        let req=crate::development_context::Request{evidence_id:"workflow".into(),path:String::new(),cursor:String::new()};
        let first=crate::development_context::retrieve(s,&dir,&packet,&[req],&Value::Null,"Public observation",&json!(["Keep this unresolved finding"])).unwrap();
        // Construct an older exchange chain exactly as retained releases stored it.
        let mut old1=first.clone();old1.as_object_mut().unwrap().remove("working_pages");
        let old_dir=op_dir(s,w["work_id"].as_str().unwrap(),"retained-old-exchange");write(&old_dir.join("context-exchange.json"),&old1,true).unwrap();
        let mut old2=old1.clone();old2["pages"]=json!([]);old2["previous_exchange"]=json!({"path":old_dir.join("context-exchange.json"),"sha256":hash(&old1).unwrap()});
        let old_dir2=op_dir(s,w["work_id"].as_str().unwrap(),"retained-old-exchange-2");write(&old_dir2.join("context-exchange.json"),&old2,true).unwrap();
        w["evidence_pending"]=json!(true);w["evidence_context"]=crate::development_context::reference(&old_dir2,&old2).unwrap();
        s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
        let mut next=context_packet(s,&w);let nextdir=op_dir(s,w["work_id"].as_str().unwrap(),"memory-v2");crate::development_context::prepare(s,&nextdir,&mut next).unwrap();
        assert_eq!(next["evidence_pages"],first["pages"]);
        let mut invalid=old_result.clone();assert!(validate_result(&invalid,&next).is_err());invalid["context_summary"]=json!("Workflow facts retained");invalid["release_evidence"]=json!([]);validate_result(&invalid,&next).unwrap();
        let digest=next["evidence_pages"][0]["sha256"].as_str().unwrap().to_owned();
        let read=crate::development_context::Request{evidence_id:next["evidence_index"]["entries"].as_array().unwrap().iter().find(|e|e["kind"]=="tree").unwrap()["id"].as_str().unwrap().to_owned(),path:String::new(),cursor:String::new()};
        let exchange=crate::development_context::retrieve_with_memory(s,&nextdir,&next,&[read],&w["evidence_context"],"Workflow facts retained; next inspect tree",&json!([]),&[digest.clone()]).unwrap();
        assert!(!exchange["working_pages"].as_array().unwrap().iter().any(|p|p["sha256"]==digest));
        assert!(exchange["delivered"].as_array().unwrap().iter().any(|p|p["sha256"]==digest));
        assert_eq!(exchange["unresolved_findings"],json!(["Keep this unresolved finding"]));
        w["evidence_context"]=crate::development_context::reference(&nextdir,&exchange).unwrap();s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
        let mut third=context_packet(s,&w);let third_dir=op_dir(s,w["work_id"].as_str().unwrap(),"memory-v2-next");crate::development_context::prepare(s,&third_dir,&mut third).unwrap();assert_eq!(third["evidence_pages"],exchange["working_pages"]);assert_eq!(third["working_summary"]["prior_turn"],exchange["summary"]);
        // A real decision ends the read set; receipts/findings remain retrievable.
        w["evidence_pending"]=json!(false);w["context_summary"]=json!({"text":"Decision facts"});s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
        let mut decided=context_packet(s,&w);crate::development_context::prepare(s,&op_dir(s,w["work_id"].as_str().unwrap(),"memory-after-decision"),&mut decided).unwrap();assert_eq!(decided["evidence_pages"],json!([]));assert_eq!(decided["working_summary"]["prior_turn"]["text"],"Decision facts");
        assert!(crate::development_context::validate_memory(&json!({"context_summary":"","release_evidence":[digest]})).is_err());
    }
    #[test]
    fn development_snapshot_omits_bulk_evidence_without_changing_saved_records() {
        let (_d, service, mut work) = fixture();
        let s = &service.session;
        let bulk = "retained exact evidence\n".repeat(50_000);
        work["obligations"] = json!([{"obligation_id":"production","status":"open",
            "description":"Check production behavior","resolution":{"evidence":bulk}}]);
        work["last_coordinator_result"] = json!({"raw":bulk});
        s.development_save("workflow", work["work_id"].as_str().unwrap(), &work, "fixture").unwrap();
        let unit = json!({"work_id":work["work_id"],"unit_id":"display-worker","status":"running",
            "title":"Implementation","role":"implementation","provider":"codex","model":"gpt-6.1-sol",
            "selection":{"profile_id":"codex"},"attempt_id":"attempt","operation_id":"worker",
            "review_diff":{"diff":bulk},"review":{"findings":[bulk]},"receipt":{"evidence":bulk}});
        s.development_save("unit", "display-worker", &unit, "fixture").unwrap();
        let snapshot = s.conversation_snapshot().unwrap();
        assert!(serde_json::to_vec(&snapshot).unwrap().len() < 512*1024);
        let dev = &snapshot["development"];
        let shown = dev["units"].as_array().unwrap().iter().find(|u|u["unit_id"]=="display-worker").unwrap();
        for key in ["work_id","unit_id","status","title","role","provider","model","selection","attempt_id","operation_id"] {
            assert_eq!(shown[key], unit[key]);
        }
        assert!(shown["review_diff"].is_null());
        assert_eq!(shown["source_record"]["sha256"], hash(&unit).unwrap());
        let shown_work = dev["workflows"].as_array().unwrap().iter().find(|w|w["work_id"]==work["work_id"]).unwrap();
        assert_eq!(shown_work["obligations"][0]["status"], "open");
        assert!(shown_work["obligations"][0]["resolution"].is_null());
        assert_eq!(s.development_record("unit", "display-worker").unwrap().unwrap(), unit);
        assert_eq!(s.development_record("workflow",work["work_id"].as_str().unwrap()).unwrap().unwrap(), work);
    }
    use crate::session_service::Config;
    #[test]
    fn python_check_flags_and_environment_do_not_dirty_integration() {
        let (_d, service, mut work) = fixture();
        let tree_path = PathBuf::from(work["integration_worktree"].as_str().unwrap());
        let tree = tree_path.as_path();
        work["head"] = json!(commit_files(tree, &[("check_fixture.py", "import sys, fixture_module\nassert sys.dont_write_bytecode\nprint('checked without bytecode')\n"), ("fixture_module.py", "VALUE = 1\n")]));
        for argv in [vec!["python3", "-B", "check_fixture.py"], vec!["python3", "check_fixture.py"]] {
            let args: Vec<String> = argv.into_iter().map(str::to_owned).collect();
            assert!(validate_check_argv(tree, &args).is_ok());
            let directory = op_dir(&service.session, work["work_id"].as_str().unwrap(), if args.len() == 3 { "test-check-b" } else { "test-check-env" });
            let packet = directory.join("packet.json");
            write(&packet, &json!({"worktree":tree,"checked_commit":work["head"],"checks":[args]}), true).unwrap();
            check_child(&packet).unwrap();
            assert!(fs::read_to_string(directory.join("checks/0.stdout")).unwrap().contains("checked without bytecode"));
            assert!(!tree.join("__pycache__").exists());
            clean(tree).unwrap();
        }
        for argv in [vec!["python3", "-c", "print('unsafe')"], vec!["python3", "-E", "check_fixture.py"], vec!["sh", "-c", "true"]] {
            assert!(validate_check_argv(tree, &argv.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
    }
    #[test]
    fn cache_recovery_retains_exact_bytes_and_replay_preserves_authority() {
        let (_d, service, mut work) = fixture(); let s = &service.session;
        let tree_path = PathBuf::from(work["integration_worktree"].as_str().unwrap());
        let tree = tree_path.as_path();
        work["head"] = json!(commit_files(tree, &[("cache_module.py", "VALUE = 1\n")]));
        fs::create_dir(tree.join("__pycache__")).unwrap();
        let original = tree.join("__pycache__/cache_module.cpython-312.pyc");
        let bytes = b"retained bytes; provenance is not inferred"; fs::write(&original, bytes).unwrap();
        work["status"] = json!("attention"); work["pending"] = Value::Null;
        let key = work["work_id"].as_str().unwrap();
        s.development_save("workflow", key, &work, "development.attention").unwrap();
        let command = json!({"command_id":"recover-cache","action":"development.resume","payload":{"work_id":key,"revision":work["revision"]}});
        let result = s.development_command(&command).unwrap(); assert_eq!(result["status"], "accepted");
        assert_eq!(s.development_command(&command).unwrap(), result);
        let saved = s.development_record("workflow", key).unwrap().unwrap();
        assert_eq!(saved["authorization"], work["authorization"]); assert_eq!(saved["scope_base"], work["scope_base"]);
        assert_eq!(saved["head"], work["head"]); assert_eq!(saved["status"], "running"); assert!(saved["pending"].is_null());
        let manifest = Path::new(saved["workspace_recoveries"][0]["manifest"].as_str().unwrap());
        assert_eq!(fs::read(manifest.parent().unwrap().join("files/__pycache__/cache_module.cpython-312.pyc")).unwrap(), bytes);
        assert!(!original.exists()); clean(tree).unwrap();
        // A crash after retaining files but before committing the command can
        // reconcile the existing manifest without replaying application work.
        assert_eq!(retain_python_caches(s, &work, "recover-cache").unwrap().unwrap()["manifest"], json!(manifest));
        let snapshot = s.snapshot().unwrap();
        assert!(snapshot["recent_events"].as_array().unwrap().iter().any(|event| event["type"] == "development.workspace_reconciled" && event["at"].is_number()));
    }
    #[test]
    fn cache_recovery_refuses_source_edits_unknown_files_and_saved_questions() {
        let (_d, service, mut work) = fixture(); let s = &service.session;
        let tree_path = PathBuf::from(work["integration_worktree"].as_str().unwrap());
        let tree = tree_path.as_path();
        work["head"] = json!(commit_files(tree, &[("cache_module.py", "VALUE = 1\n")]));
        fs::create_dir(tree.join("__pycache__")).unwrap();
        let cache = tree.join("__pycache__/cache_module.cpython-312.pyc"); fs::write(&cache, b"keep cache").unwrap();
        fs::write(tree.join("unexpected.txt"), "keep user input").unwrap();
        assert!(retain_python_caches(s, &work, "unknown-file").is_err()); assert!(cache.exists());
        fs::remove_file(tree.join("unexpected.txt")).unwrap();
        fs::write(tree.join("cache_module.py"), "VALUE = 2\n").unwrap();
        assert!(retain_python_caches(s, &work, "source-edit").is_err()); assert!(cache.exists());
        work["status"] = json!("attention"); work["question"] = json!({"prompt":"Choose a format"});
        let key = work["work_id"].as_str().unwrap(); s.development_save("workflow", key, &work, "development.attention").unwrap();
        let command = json!({"command_id":"question-bypass","action":"development.resume","payload":{"work_id":key,"revision":work["revision"]}});
        assert!(s.development_command(&command).unwrap_err().contains("/reply"));
        assert!(cache.exists()); assert_eq!(fs::read_to_string(tree.join("cache_module.py")).unwrap(), "VALUE = 2\n");
    }
    #[test]
    fn new_topic_final_packet_fits_after_large_delivery_and_preserves_blocked_user_input_without_replay(){
        let (_d,service,mut work)=fixture();let s=&service.session;
        let repeated="OLD_REQUEST_RESOLUTION_DO_NOT_REPLAY ".repeat(1000);
        work["obligations"]=json!((0..32).map(|i|json!({"name":format!("Old obligation {i}"),"status":"resolved","resolution":{"rationale":repeated}})).collect::<Vec<_>>());
        s.development_save("workflow",work["work_id"].as_str().unwrap(),&work,"test.large_previous_workflow").unwrap();
        let knowledge=json!({"work_id":work["work_id"],"source_commit":work["head"],"source_map":"src/lib.rs\nsrc/main.rs","open_obligations":work["obligations"],"conventions":"Existing project conventions","contracts":[{"path":"src/lib.rs","source_commit":work["head"],"sha256":"source-hash","excerpt":"pub fn existing_project_api() {}"}]});
        assert!(crate::codec::canonical(&knowledge).unwrap().len()>crate::native_contract::PACKET_LIMIT);
        s.development_save("knowledge",work["work_id"].as_str().unwrap(),&knowledge,"test.large_previous_knowledge").unwrap();
        let sent=s.conversation_submit(&json!({"command_id":"fresh-topic","action":"submit_message","payload":{"subject_id":"new-topic","text":"Plan a change from my conversation","intent":"discussion"}})).unwrap();assert_eq!(sent["status"],"accepted");
        let next=s.conversation_next_queued().unwrap().unwrap();let p=profile(s).unwrap();let packet=s.scope_packet(&next,&p,&json!(hash(&p).unwrap())).unwrap();
        assert!(crate::codec::canonical(&packet).unwrap().len()<=crate::native_contract::PACKET_LIMIT);assert_eq!(packet["project_key"],s.key);assert_eq!(packet["turn_id"],next["turn_id"]);assert_eq!(packet["development"],true);
        assert_eq!(packet["selected_context"]["scope"]["base"],Value::Null);assert_eq!(packet["request_context"]["workflows"],json!([]));assert_eq!(packet["request_context"]["units"],json!([]));assert_eq!(packet["request_context"]["history"].as_array().unwrap().len(),1);
        assert!(!packet.to_string().contains("OLD_REQUEST_RESOLUTION_DO_NOT_REPLAY"));assert!(packet.to_string().contains("existing_project_api"));
        let turn=next["turn_id"].as_str().unwrap();let frozen=s.workflow.state.join("session/conversation").join(turn).join("packet.json");write(&frozen,&packet,true).unwrap();let bytes=fs::read(&frozen).unwrap();
        s.conversation_block_queued(turn,"Fixture pre-reservation size refusal").unwrap();assert!(s.chat_reserved().unwrap().is_none());
        let sent=s.conversation_submit(&json!({"command_id":"human-follow-up","action":"submit_message","payload":{"subject_id":"new-topic","text":"Continue with my saved request","intent":"discussion"}})).unwrap();assert_eq!(sent["status"],"accepted");
        let next2=s.conversation_next_queued().unwrap().unwrap();assert_ne!(next2["turn_id"],next["turn_id"]);let packet2=s.scope_packet(&next2,&p,&json!(hash(&p).unwrap())).unwrap();
        let prior=packet2["selected_context"]["messages"].as_array().unwrap().iter().find(|m|m["text"]=="Plan a change from my conversation").unwrap();assert!(prior["delivery_state"].as_str().unwrap().contains("not an executed call"));assert_eq!(fs::read(frozen).unwrap(),bytes);
        assert_eq!(s.conversation_reserve(next2["turn_id"].as_str().unwrap(),&packet2,"conversation").unwrap()["status"],"accepted");
        assert!(!s.workflow.state.join("session/coordinator").join(next2["turn_id"].as_str().unwrap()).exists());
    }
    #[test]
    fn final_context_fitting_preserves_mandatory_scope_and_rejects_specific_packet_identity_errors(){
        let (_d,service,work)=fixture();let s=&service.session;let sent=s.conversation_submit(&json!({"command_id":"scope-progress","action":"submit_message","payload":{"subject_id":"discussion","task_id":work["work_id"],"text":"Review my current scope progress","intent":"discussion"}})).unwrap();assert_eq!(sent["status"],"accepted");let next=s.conversation_next_queued().unwrap().unwrap();let p=profile(s).unwrap();let packet=s.scope_packet(&next,&p,&json!(hash(&p).unwrap())).unwrap();
        assert_eq!(packet["selected_context"]["scope"]["base"],work["scope_base"]);assert_eq!(packet["request_context"]["workflows"][0]["work_id"],work["work_id"]);
        let mut large=packet.clone();large["message"]=json!("Unicode 日本語 ".repeat(4000));let mandatory=large["selected_context"].clone();let text=large["message"].clone();let inputs=json!({"request_context":{"work_id":work["work_id"],"workflows":[{"large":"x".repeat(120000)}],"history":[],"units":[],"knowledge":[]},"project_context":{"documents":[{"text":"x".repeat(120000)}],"tracked_files":[]}});
        fit_discussion_context(&mut large,&inputs).unwrap();assert_eq!(large["message"],text);assert_eq!(large["selected_context"],mandatory);assert!(crate::codec::canonical(&large).unwrap().len()<=crate::native_contract::PACKET_LIMIT);assert_eq!(large["request_context"]["available_counts"]["workflows"],1);assert!(large["request_context"]["workflows"].as_array().unwrap().is_empty());
        let turn=next["turn_id"].as_str().unwrap();let mut wrong=packet.clone();wrong["project_key"]=json!("wrong-project");assert!(s.conversation_reserve(turn,&wrong,"conversation").unwrap()["message"].as_str().unwrap().contains("different project"));wrong=packet.clone();wrong["turn_id"]=json!("wrong-turn");assert!(s.conversation_reserve(turn,&wrong,"conversation").unwrap()["message"].as_str().unwrap().contains("different turn"));wrong=packet;wrong["extra"]=json!("x".repeat(100000));assert!(s.conversation_reserve(turn,&wrong,"conversation").unwrap()["message"].as_str().unwrap().contains("exceeding the 98304-byte"));assert!(s.chat_reserved().unwrap().is_none());
    }
    #[test]
    fn live_planning_is_project_neutral_and_question_reply_preserves_authority() {
        assert!(!COORDINATOR.contains("Example Task App"));assert!(!COORDINATOR.contains("persistence and CLI"));
        let (_d,service,mut work)=fixture();let s=&service.session;
        fs::write(s.project.join("README.md"),"An existing graph renderer").unwrap();
        let context=planning_context(s).unwrap();assert!(context.to_string().contains("graph renderer"));
        let authority=work["authorization"].clone();let scope=work["scope_base"].clone();let generation=work["requirement_generation"].clone();
        work["status"]=json!("attention");work["question"]=json!({"prompt":"Use SVG or PNG?"});
        s.development_save("workflow",work["work_id"].as_str().unwrap(),&work,"test.question").unwrap();
        let command=json!({"command_id":"answer-renderer","action":"development.reply","payload":{"work_id":work["work_id"],"revision":work["revision"],"text":"Use SVG within the approved scope"}});
        assert_eq!(s.development_command(&command).unwrap()["status"],"accepted");assert_eq!(s.development_command(&command).unwrap()["status"],"accepted");
        let saved=s.development_record("workflow",work["work_id"].as_str().unwrap()).unwrap().unwrap();
        assert_eq!(saved["status"],"running");assert_eq!(saved["answers"].as_array().unwrap().len(),1);assert_eq!(saved["authorization"],authority);assert_eq!(saved["scope_base"],scope);assert_eq!(saved["requirement_generation"],generation);
        let mut stale=command;stale["command_id"]=json!("old-question");assert!(s.development_command(&stale).is_err());
    }
    #[test]
    fn capacity_continuation_preserves_partial_source_and_refuses_changed_input_before_launch() {
        for changed in [false, true] {
            let (_d, service, w) = fixture();
            let s = &service.session;
            let work = w["work_id"].as_str().unwrap();
            let tree = worktree_path(s, work, "capacity").unwrap();
            let branch = format!("waywr/{work}/tasks/capacity/work");
            ensure_tree(s, &tree, &branch, w["head"].as_str().unwrap()).unwrap();
            fs::write(tree.join("partial.txt"), "retained partial worker input").unwrap();
            let snapshot = workspace_snapshot(&tree).unwrap();
            let uid = format!("{work}-capacity");
            let unit = json!({"unit_id":uid,"work_id":work,"request_id":w["request_id"],"name":"capacity",
                "status":"ready","branch":branch,"worktree":tree,"base_commit":w["head"],"attempt_revision":2,
                "paths":["partial.txt"],"start_after":[],"accept_after":[],"continuation_snapshot":snapshot});
            s.development_save("unit", &uid, &unit, "test.capacity_settled_fixture")
                .unwrap();
            if changed {
                fs::write(tree.join("partial.txt"), "unexpected later edit").unwrap();
            }
            let result = service.start_development_units(&w);
            assert_eq!(result.is_err(), changed);
            let saved = s.development_record("unit", &uid).unwrap().unwrap();
            assert_eq!(saved["status"], if changed { "ready" } else { "running" });
            assert_eq!(
                fs::read_to_string(tree.join("partial.txt")).unwrap(),
                if changed {
                    "unexpected later edit"
                } else {
                    "retained partial worker input"
                }
            );
            let operation = op_dir(s, work, "worker-capacity-r2");
            assert_eq!(operation.join("packet.json").exists(), !changed);
            assert!(!operation.join("active-host-attempt.json").exists());
        }
    }
    #[test]
    fn integration_requires_commit_visible_in_the_frozen_coordinator_review() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let unit = json!({"unit_id":"storage","status":"submitted","result_commit":a});
        assert!(
            require_frozen_review(
                &json!({"units":[{"unit_id":"storage","status":"running","result_commit":null}]}),
                &unit
            )
            .is_err()
        );
        assert!(require_frozen_review(&json!({"units":[unit.clone()]}), &unit).is_ok());
        let mut changed = unit.clone();
        changed["result_commit"] = json!(b);
        assert!(require_frozen_review(&json!({"units":[unit]}), &changed).is_err());
        assert!(require_frozen_review(&json!({"units":[]}), &changed).is_err());
    }
    #[test]
    fn combined_check_pending_binds_current_generation_and_exact_commit_before_launch() {
        let (_d, service, mut w) = fixture();
        let checks = vec![vec!["cargo".into(), "test".into(), "--locked".into()]];
        service
            .prepare_development_checks(&mut w, checks.clone())
            .unwrap();
        assert_eq!(
            w["pending"]["requirement_generation"],
            w["requirement_generation"]
        );
        assert_eq!(w["pending"]["scope_base"], w["scope_base"]);
        assert_eq!(w["pending"]["checked_commit"], w["head"]);
        let directory = op_dir(
            &service.session,
            w["work_id"].as_str().unwrap(),
            w["pending"]["operation_id"].as_str().unwrap(),
        );
        let packet = read(&directory.join("packet.json")).unwrap();
        assert_eq!(
            packet["requirement_generation"],
            w["pending"]["requirement_generation"]
        );
        assert_eq!(packet["checked_commit"], w["pending"]["checked_commit"]);
        assert_eq!(packet["checks"], json!(checks));
        assert!(proven_uninvoked(&service.session, &directory).unwrap());
        assert!(!directory.join("active-host-attempt.json").exists());
    }
    #[test]
    fn blocked_native_worker_keeps_its_cause_without_becoming_a_submission() {
        let d = tempfile::tempdir().unwrap();
        let packet =
            json!({"development":true,"operation_kind":"development-worker","model":"gpt-6.1-sol"});
        let report = json!({"schema_version":1,"summary":"bwrap: setting up uid map: Permission denied",
            "commit":"","checks":["Cargo did not execute"],"remaining_obligations":["Implement assigned source"]});
        let bytes = serde_json::to_string(&report).unwrap();
        fs::write(d.path().join("native-final.json"), &bytes).unwrap();
        let events = vec![
            json!({"type":"item.completed","item":{"type":"agent_message","text":bytes}}),
            json!({"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}),
        ];
        let (result, _) = finish_native(&events, d.path(), &packet, Some(0)).unwrap();
        assert_eq!(result, report);
        assert_eq!(result["commit"], "");
        assert!(
            result["summary"]
                .as_str()
                .unwrap()
                .contains("Permission denied")
        );
        let mut invalid = report;
        invalid["commit"] = json!("made-up-commit");
        assert!(validate_result(&invalid, &packet).is_err());
        assert!(finish_native(&events, d.path(), &packet, Some(1)).is_err());
    }
    #[test]
    fn ready_workers_dispatch_while_coordinator_is_pending_without_launching_native_clients() {
        let (_d, service, mut w) = fixture();
        let s = &service.session;
        w["pending"] = json!({"operation_id":"coordinator-already-running","kind":"coordinate"});
        let pending = w["pending"].clone();
        for name in ["persistence", "cli", "later"] {
            let uid = format!("{}-{name}", w["work_id"].as_str().unwrap());
            let unit = json!({"schema_version":1,"unit_id":uid,"name":name,"work_id":w["work_id"],
                "request_id":w["request_id"],"status":"planned","instructions":"independent work",
                "paths":[format!("{name}.rs")],"start_after":[],"accept_after":[],
                "branch":format!("waywr/{}/{name}",w["work_id"].as_str().unwrap()),
                "worktree":worktree_path(s,w["work_id"].as_str().unwrap(),name).unwrap(),
                "base_commit":null,"attempt_revision":1});
            s.development_save("unit", &uid, &unit, "development.assignment_planned")
                .unwrap();
        }
        service.start_development_units(&w).unwrap();
        let all = units(s, &w).unwrap();
        assert_eq!(all.iter().filter(|u| u["status"] == "running").count(), 2);
        assert_eq!(all.iter().filter(|u| u["status"] == "planned").count(), 1);
        assert_eq!(w["pending"], pending);
        for u in all.iter().filter(|u| u["status"] == "running") {
            let directory = op_dir(
                s,
                w["work_id"].as_str().unwrap(),
                u["operation_id"].as_str().unwrap(),
            );
            assert!(directory.join("packet.json").is_file());
            assert!(!directory.join("active-host-attempt.json").exists());
            assert_eq!(u["base_commit"], w["head"]);
        }
    }
    #[test]fn provider_roles_freeze_mixed_packets_without_launching_clients(){
        let (_d,service,mut work)=fixture();let s=&service.session;
        fs::write(s.project.join(".waywr/project.toml"),format!("schema_version = 2\nproject_key = \"{}\"\n[execution]\nhost = \"systemd-user-v1\"\n[conversation]\nschema_version = 5\n",s.key)).unwrap();
        crate::provider_selection::migrate_project(&s.project,&s.workflow.state,&s.key).unwrap();s.development_schema().unwrap();
        let (prefs,rev)=crate::provider_selection::preferences(s).unwrap();let mut config=prefs.value();
        config["profiles"]["reviewer"]=json!({"provider":"claude","model":"claude-sonnet-5-5","effort":"medium","binary":"/missing/claude","binary_sha256":null,"context_bytes":98304});
        config["roles"]["review"]=json!({"default":"reviewer","allowed":["reviewer"]});
        crate::provider_selection::save(s,&json!({"command_id":"fixture-role-save","action":"providers.save","payload":{"expected_revision":rev,"preferences":config}})).unwrap();
        let (prefs,rev)=crate::provider_selection::preferences(s).unwrap();
        crate::provider_selection::command(s,&json!({"command_id":"fixture-role-policy","action":"providers.approve_policy","payload":{"expected_revision":rev,"work_id":work["work_id"],"policy_sha256":hash(&work["policy"]).unwrap(),"routing_sha256":hash(&prefs.allowed_policy().unwrap()).unwrap()}})).unwrap();
        work=s.development_record("workflow",work["work_id"].as_str().unwrap()).unwrap().unwrap();work["pending"]=Value::Null;s.development_save("workflow",work["work_id"].as_str().unwrap(),&work,"fixture").unwrap();
        let submitted=submission(s,&work,"submitted",work["head"].as_str().unwrap(),&[("fixture.txt","exact local fixture")],json!([]));
        for (name,role) in [("edit","implementation"),("review","review")]{
            let uid=format!("{}-{name}",work["work_id"].as_str().unwrap());let unit=json!({"unit_id":uid,"work_id":work["work_id"],"request_id":work["request_id"],"name":name,"title":name,"role":role,"instructions":"Fixture only","status":"planned","paths":if role=="review"{vec![]}else{vec!["other.txt"]},"start_after":[],"accept_after":[],"attempt_revision":1,"base_commit":null,"branch":format!("waywr/{}/tasks/{name}/work",work["work_id"].as_str().unwrap()),"worktree":worktree_path(s,work["work_id"].as_str().unwrap(),name).unwrap(),"review_target_names":if role=="review"{vec!["submitted"]}else{vec![]}});
            s.development_save("unit",&uid,&unit,"fixture").unwrap();
        }
        service.start_development_units(&work).unwrap();let mut frozen=vec![];
        for unit in units(s,&work).unwrap().iter().filter(|u|u["status"]=="running"){
            let dir=op_dir(s,work["work_id"].as_str().unwrap(),unit["operation_id"].as_str().unwrap());let packet=read(&dir.join("packet.json")).unwrap();
            assert!(!dir.join("active-host-attempt.json").exists());
            assert_eq!(packet["provider"],if unit["role"]=="review"{"claude"}else{"codex"});
            if unit["role"]=="review"{assert_eq!(packet["review_targets"][0]["result_commit"],submitted["result_commit"]);assert!(!Path::new(unit["worktree"].as_str().unwrap()).exists());}
            frozen.push((dir.join("packet.json"),fs::read(dir.join("packet.json")).unwrap()));
        }
        assert_eq!(frozen.len(),2);let mut edited=prefs.value();edited["profiles"]["codex"]["effort"]=json!("high");
        crate::provider_selection::save(s,&json!({"command_id":"fixture-future-edit","action":"providers.save","payload":{"expected_revision":rev,"preferences":edited}})).unwrap();
        for (path,bytes) in frozen{assert_eq!(fs::read(&path).unwrap(),bytes);let packet=read(&path).unwrap();assert!(crate::provider_selection::packet_authorized(s,&work,&packet).unwrap());}
        let uid=format!("{}-race",work["work_id"].as_str().unwrap());
        let old_selection=prefs.resolve("implementation",&Value::Null,None,&work["policy"],rev,"prepared before preference save").unwrap();
        let mut candidate=json!({"unit_id":uid,"work_id":work["work_id"],"status":"planned","attempt_revision":1,"role":"implementation","operation_id":"worker-race-r1","selection":old_selection});
        s.development_save("unit",&uid,&candidate,"fixture").unwrap();candidate["status"]=json!("running");
        assert!(!s.development_reserve_unit(&work,&candidate,"development.assignment_started").unwrap());
        let current=s.development_record("unit",&uid).unwrap().unwrap();assert_eq!(current["status"],"planned");assert_eq!(current["attempt_revision"],2);
    }
    pub(super) fn fixture() -> (tempfile::TempDir, Service, Value) {
        let d = tempfile::tempdir().unwrap();
        let project = d.path().join("project");
        let state = project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        let key = "a".repeat(32);
        write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":9,"execution_host":"systemd-user-v1","project_key":key}),true).unwrap();
        write(&project.join(".waywr/coordinator.json"),&crate::provider_selection::recovery_tests::prefs(),true).unwrap();
        let exe = std::env::current_exe().unwrap();
        let service = Service::new(Config {
            project: project.clone(),
            state: state.clone(),
            key: key.clone(),
            release: "test-development".into(),
            ipc: d.path().join("socket"),
            herdr_socket: PathBuf::new(),
            runner: exe.clone(),
            frontend: exe,
        })
        .unwrap();
        let s = &service.session;
        let card=crate::coordination::tests::seed(s);
        crate::coordination::tests::present(s,&card);
        let task="fixture-task".to_owned();
        let scope = s.scope_detail(&task).unwrap().unwrap();
        let mut command = json!({"command_id":"scope-auth","action":"authorize_scope","payload":{"base":scope["base"],"decision_revision":scope["decision_revision"],"authorization_command_id":scope["authorization_command_id"],"acknowledged_unresolved_sha256":hash(&json!([])).unwrap(),"operating_policy_sha256":"old-policy"}});
        assert_eq!(s.scope_command(&command).unwrap()["code"], "stale_policy");
        command["payload"]["operating_policy_sha256"] = json!(hash(&policy(s).unwrap()).unwrap());
        assert_eq!(s.scope_command(&command).unwrap()["status"], "accepted");
        let mut w = s.development_record("workflow", &task).unwrap().unwrap();
        write(
            &state.join("supervision/owner.json"),
            &json!({"schema_version":1,"generation":service.generation,"project_key":key}),
            false,
        )
        .unwrap();
        // Initialization prepares a coordinator packet but never launches an attempt in this fixture.
        service.tick_development_work(&mut w).unwrap();
        w["pending"] = Value::Null;
        (d, service, w)
    }
    fn commit_files(tree: &Path, files: &[(&str, &str)]) -> String {
        for (p, t) in files {
            fs::write(tree.join(p), t).unwrap();
        }
        git(tree, &["add", "--all"]).unwrap();
        git(
            tree,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@localhost",
                "commit",
                "-m",
                "Worker fixture submission",
            ],
        )
        .unwrap();
        git(tree, &["rev-parse", "HEAD"]).unwrap()
    }
    pub(super) fn submission(
        s: &Session,
        w: &Value,
        name: &str,
        base: &str,
        files: &[(&str, &str)],
        deps: Value,
    ) -> Value {
        let tree = worktree_path(s, w["work_id"].as_str().unwrap(), name).unwrap();
        let branch = format!("waywr/{}/{name}", w["work_id"].as_str().unwrap());
        ensure_tree(s, &tree, &branch, base).unwrap();
        let commit = commit_files(&tree, files);
        let uid = format!("{}-{name}", w["work_id"].as_str().unwrap());
        let u = json!({"schema_version":1,"work_id":w["work_id"],"request_id":w["request_id"],"unit_id":uid,"name":name,"status":"submitted","base_commit":base,"result_commit":commit,"branch":branch,"worktree":tree,"requirement_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"start_after":[],"accept_after":deps,"diff":"fixture"});
        s.development_save("unit", &uid, &u, "development.unit_submitted")
            .unwrap();
        u
    }
    pub(super) fn effect(w: &Value, u: &Value) -> Value {
        json!({"effect_id":format!("effect-{}",u["name"].as_str().unwrap()),"work_id":w["work_id"],"request_id":w["request_id"],"status":"intent","expected_head":w["head"],"submitted_commit":u["result_commit"],"unit_id":u["unit_id"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"review":{"fixture":true}})
    }
    #[test]
    fn socket_both_completion_orders_preserve_parallel_commits_and_link_real_library() {
        for storage_first in [true, false] {
            let (_d, service, mut w) = fixture();
            let s = &service.session;
            let baseline = w["head"].as_str().unwrap().to_owned();
            let contract = submission(
                s,
                &w,
                "contract",
                &baseline,
                &[
                    ("socketlib.h", "int socket_value(void);\n"),
                    (".gitignore", ".waywr/\napp\n"),
                ],
                json!([]),
            );
            let mut e = effect(&w, &contract);
            service.apply_development_effect(&mut w, &mut e).unwrap();
            let base = w["head"].as_str().unwrap().to_owned();
            let lib = submission(
                s,
                &w,
                "library",
                &base,
                &[(
                    "socketlib.c",
                    "#include \"socketlib.h\"\nint socket_value(void){return 42;}\n",
                )],
                json!([]),
            );
            let consumer = submission(
                s,
                &w,
                "consumer",
                &base,
                &[
                    (
                        "main.c",
                        "#include \"socketlib.h\"\nint main(void){return socket_value()==42?0:1;}\n",
                    ),
                    (
                        "stub.c",
                        "#include \"socketlib.h\"\nint socket_value(void){return 0;}\n",
                    ),
                ],
                json!([]),
            );
            let original_consumer = git(
                Path::new(consumer["worktree"].as_str().unwrap()),
                &["rev-parse", "HEAD"],
            )
            .unwrap();
            for u in if storage_first {
                [&lib, &consumer]
            } else {
                [&consumer, &lib]
            } {
                let mut e = effect(&w, u);
                service.apply_development_effect(&mut w, &mut e).unwrap();
            }
            assert_eq!(
                git(
                    Path::new(consumer["worktree"].as_str().unwrap()),
                    &["rev-parse", "HEAD"]
                )
                .unwrap(),
                original_consumer
            );
            let tree = Path::new(w["integration_worktree"].as_str().unwrap());
            assert!(
                Command::new("cc")
                    .args(["main.c", "socketlib.c", "-o", "app"])
                    .current_dir(tree)
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(Command::new(tree.join("app")).status().unwrap().success());
            assert!(w["checks"].is_null());
            assert_ne!(w["status"], "delivered");
        }
    }
    #[test]
    fn integration_intent_reconciles_after_git_commit_before_writer_ack_and_preserves_dirty() {
        let (_d, service, mut w) = fixture();
        let s = &service.session;
        let base = w["head"].as_str().unwrap().to_owned();
        let u = submission(s, &w, "worker", &base, &[("one.txt", "one")], json!([]));
        let mut e = effect(&w, &u);
        let eid = e["effect_id"].as_str().unwrap().to_owned();
        s.development_save("effect", &eid, &e, "development.integration_intent")
            .unwrap();
        let tree = Path::new(w["integration_worktree"].as_str().unwrap());
        let msg = format!("Integration\n\nWaywr-Intent: {eid}");
        git(
            tree,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@localhost",
                "merge",
                "--no-ff",
                "-m",
                &msg,
                u["result_commit"].as_str().unwrap(),
            ],
        )
        .unwrap();
        let actual = git(tree, &["rev-parse", "HEAD"]).unwrap();
        service.apply_development_effect(&mut w, &mut e).unwrap();
        assert_eq!(w["head"], actual);
        assert_eq!(e["status"], "recorded");
        let count = git(
            Path::new(w["integration_worktree"].as_str().unwrap()),
            &["rev-list", "--count", "HEAD"],
        )
        .unwrap();
        service.apply_development_effect(&mut w, &mut e).unwrap();
        assert_eq!(
            git(
                Path::new(w["integration_worktree"].as_str().unwrap()),
                &["rev-list", "--count", "HEAD"]
            )
            .unwrap(),
            count
        );
        let u = submission(s, &w, "second", &actual, &[("two.txt", "two")], json!([]));
        let mut e = effect(&w, &u);
        let path = Path::new(w["integration_worktree"].as_str().unwrap()).join("one.txt");
        fs::write(&path, "external dirty edit").unwrap();
        assert!(
            service
                .apply_development_effect(&mut w, &mut e)
                .unwrap_err()
                .contains("dirty")
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "external dirty edit");
    }
    #[test]
    fn request_history_links_scope_attempts_corrections_and_restart() {
        let (d, service, w) = fixture();
        let s = &service.session;
        request_history::project(s).unwrap();
        let requests = request_history::requests(s).unwrap();
        assert_eq!(requests.len(), 1);
        let p = Path::new(requests[0]["log_path"].as_str().unwrap());
        let first = fs::read_to_string(p).unwrap();
        assert!(first.contains("Socket example"));
        assert!(first.contains("Proposal for review"));
        assert!(first.contains("development.authorized"));
        let mut v = json!({"work_id":w["work_id"],"request_id":w["request_id"],"correction":"Prior claim superseded; checks still pending","attempt_id":"replacement"});
        s.development_save("note", "correction", &v, "development.correction")
            .unwrap();
        request_history::project(s).unwrap();
        let after = fs::read_to_string(p).unwrap();
        assert!(after.starts_with(&first));
        assert!(after.contains("Prior claim superseded"));
        v["attempt_id"] = json!("third");
        s.development_save("note", "correction", &v, "development.reassignment")
            .unwrap();
        request_history::project(s).unwrap();
        let a = fs::read(p).unwrap();
        request_history::project(s).unwrap();
        assert_eq!(fs::read(p).unwrap(), a);
        assert!(p.starts_with(d.path()));
        assert!(p.with_extension("summary.md").exists());
    }
}

#[cfg(test)]
mod connected_recovery_tests {
    use super::*;
    #[test]
    fn coordinator_batch_is_atomic_and_duplicate_adoption_does_not_assign_twice() {
        let (_d, service, mut w) = super::tests::fixture();
        let s = &service.session;
        let key = w["work_id"].as_str().unwrap().to_owned();
        w["pending"] = json!({"operation_id":"decision-a"});
        s.development_save("workflow", &key, &w, "test.reserved")
            .unwrap();
        let u = json!({"work_id":key,"unit_id":"a"});
        let records = vec![
            (
                "unit".into(),
                "a".into(),
                u.clone(),
                "test.assignment".into(),
            ),
            (
                "unit".into(),
                "a".into(),
                u.clone(),
                "test.assignment".into(),
            ),
        ];
        w["pending"] = Value::Null;
        assert!(
            s.development_commit_decision(&w, &records, "decision-a")
                .is_err()
        );
        assert!(s.development_record("unit", "a").unwrap().is_none());
        s.development_commit_decision(&w, &records[..1], "decision-a")
            .unwrap();
        assert!(
            s.development_commit_decision(&w, &records[..1], "decision-a")
                .is_err()
        );
        assert_eq!(s.development_records("unit").unwrap().len(), 1);
    }
    #[test]
    fn steering_fences_old_integration_and_retains_running_and_unaffected_commits() {
        let (_d, service, w) = super::tests::fixture();
        let s = &service.session;
        let base = w["head"].as_str().unwrap().to_owned();
        let u = super::tests::submission(
            s,
            &w,
            "consumer",
            &base,
            &[("consumer.txt", "useful work")],
            json!([]),
        );
        let mut e = super::tests::effect(&w, &u);
        let saved_scope = s
            .scope_detail(w["work_id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        let command = json!({"command_id":"steer-a","action":"development.steer","payload":{"work_id":w["work_id"],"requirement_generation":w["requirement_generation"],"in_scope":true,"text":"Clarify Unicode invalid-input errors within the approved scope"}});
        assert_eq!(
            s.development_command(&command).unwrap()["status"],
            "accepted"
        );
        assert_eq!(
            s.development_command(&command).unwrap()["status"],
            "accepted"
        );
        let mut updated = s
            .development_record("workflow", w["work_id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(updated["steering"].as_array().unwrap().len(), 1);
        assert_eq!(updated["requirement_generation"], 1);
        assert!(
            service
                .apply_development_effect(&mut updated, &mut e)
                .is_err()
        );
        let after = s
            .scope_detail(w["work_id"].as_str().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(after["base"], saved_scope["base"]);
        assert_eq!(after["authorized"], true);
        assert_eq!(
            git(
                Path::new(u["worktree"].as_str().unwrap()),
                &["rev-parse", "HEAD"]
            )
            .unwrap(),
            u["result_commit"].as_str().unwrap()
        );
    }
    #[test]
    fn task_worktree_symlink_is_preserved_and_refused() {
        let (d, service, w) = super::tests::fixture();
        let base = service.session.project.join(".waywr/worktrees");
        let foreign = d.path().join("foreign");
        fs::create_dir(&foreign).unwrap();
        let work = "other-work";
        std::os::unix::fs::symlink(&foreign, base.join(work)).unwrap();
        assert!(
            worktree_path(&service.session, work, "worker")
                .unwrap_err()
                .contains("symlink")
        );
        assert!(foreign.read_dir().unwrap().next().is_none());
        assert!(!w["head"].is_null());
    }
}

/// Only explicit terminal/provider errors establish capacity refusal. Quiet output, missing
/// processes and arbitrary tool diagnostics cannot authorize another invocation.
pub fn capacity_refusal(events: &[Value], exit: Option<i32>) -> Option<Value> {
    if exit == Some(0) || exit.is_none() {
        return None;
    }
    events.iter().rev().filter(|e|e["type"]=="turn.failed"||e["type"]=="error").find_map(|e|{
        let error=if e["error"].is_object(){&e["error"]}else{e};
        let status=error["status_code"].as_u64();
        let code=error["code"].as_str().or(error["type"].as_str()).unwrap_or("");
        if matches!(status,Some(400|401|403))||matches!(code,"authentication_error"|"permission_denied"|"invalid_request_error"|"configuration_error"){return None;}
        let message=error["message"].as_str().unwrap_or("");let lower=message.to_ascii_lowercase();
        let explicit=status==Some(429)||["rate_limit_exceeded","rate_limit","insufficient_quota","usage_limit_reached","capacity_unavailable","overloaded_error"].contains(&code)||
            ["rate limit reached","rate limit exceeded","hit your usage limit","usage limit reached","quota exceeded","insufficient quota","capacity unavailable"].iter().any(|text|lower.contains(text));
        if !explicit{return None;}
        Some(json!({"reason":message.chars().take(1000).collect::<String>(),"provider_error_code":code,"retry_after_secs":error["retry_after_secs"].as_u64().or(e["retry_after_secs"].as_u64()).unwrap_or(60).max(60)}))
    })
}
fn capacity_proof(s: &Session, directory: &Path) -> Result<Value, String> {
    let attempt = Attempt::load(&s.workflow.state, directory)?;
    let proof = read(&attempt.directory.join("completion.json"))?;
    if proof["category"] != "capacity"
        || proof["settled"] != true
        || proof["known_completion"] != true
    {
        return Err("Capacity receipt lacks positive settled refusal; no retry".into());
    }
    Ok(proof["capacity"].clone())
}
impl Session {
    pub fn development_chat_attempt(&self, turn: &str) -> Result<Option<PathBuf>, String> {
        let record = self.development_record("chat-capacity", turn)?;
        if record
            .as_ref()
            .is_some_and(|r| now() < r["retry_at"].as_f64().unwrap_or(0.0))
        {
            return Ok(None);
        }
        let revision = record
            .as_ref()
            .and_then(|r| r["next_attempt"].as_u64())
            .unwrap_or(1);
        Ok(Some(
            self.workflow
                .state
                .join("session/coordinator")
                .join(turn)
                .join(format!("call-v{revision}")),
        ))
    }
    pub fn development_chat_wait(
        &self,
        turn: &str,
        directory: &Path,
        done: &Value,
    ) -> Result<(), String> {
        let proof = capacity_proof(self, directory)?;
        let previous = self.development_record("chat-capacity", turn)?;
        let revision = previous
            .as_ref()
            .and_then(|r| r["next_attempt"].as_u64())
            .unwrap_or(1);
        let record = json!({"turn_id":turn,"status":"capacity-waiting","refusal":proof,"receipt":done,"next_attempt":revision+1,"retry_at":now()+proof["retry_after_secs"].as_u64().unwrap_or(60)as f64});
        self.development_save("chat-capacity", turn, &record, "conversation.capacity_wait")?;
        if crate::provider_selection::enabled(self){
            let attempt=Attempt::load(&self.workflow.state,directory)?;let proof=read(&attempt.directory.join("completion.json"))?;
            let policy=self.development_records("workflow")?.iter().rev().find(|w|!matches!(w["status"].as_str(),Some("delivered"|"cancelled"))).map(|w|w["policy"].clone()).unwrap_or(crate::provider_selection::discussion_policy(self)?);
            crate::provider_selection::requeue_capacity(self,turn,&proof)?;
            if crate::provider_selection::capacity_switch(self,&policy,&proof,turn)?{
                let mut record=record;record["retry_at"]=json!(now());self.development_save("chat-capacity",turn,&record,"providers.capacity_ready")?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod capacity_tests {
    use super::*;
    #[test]
    fn explicit_capacity_wait_keeps_retry_after_and_never_infers_nonlaunch() {
        let events = vec![
            json!({"type":"turn.failed","error":{"message":"Rate limit reached","status_code":429,"retry_after_secs":180}}),
        ];
        assert_eq!(
            capacity_refusal(&events, Some(1)).unwrap()["retry_after_secs"],
            180
        );
        assert!(capacity_refusal(&events, None).is_none());
        assert!(capacity_refusal(&events, Some(0)).is_none());
        assert!(capacity_refusal(&[json!({"type":"item.completed","item":{"type":"command_execution","aggregated_output":"capacity unavailable"}})],Some(1)).is_none());
        assert!(capacity_refusal(&[], Some(1)).is_none());
        assert!(capacity_refusal(&[json!({"type":"turn.failed","error":{"code":"authentication_error","status_code":401,"message":"Quota unavailable until authentication is corrected"}})],Some(1)).is_none());
        assert!(capacity_refusal(&[json!({"type":"error","message":"Invalid capacity configuration"})],Some(1)).is_none());
    }
}

#[cfg(test)]
mod notification_tests {
    use super::*;
    #[test]
    fn accepted_dependency_notifies_once_without_mutating_active_dirty_consumer() {
        let (_d, service, mut w) = super::tests::fixture();
        let s = &service.session;
        let base = w["head"].as_str().unwrap().to_owned();
        let lib = super::tests::submission(
            s,
            &w,
            "library",
            &base,
            &[("library.txt", "real implementation")],
            json!([]),
        );
        let mut consumer = super::tests::submission(
            s,
            &w,
            "consumer",
            &base,
            &[("consumer.txt", "useful work")],
            json!(["library"]),
        );
        consumer["status"] = json!("running");
        s.development_save(
            "unit",
            consumer["unit_id"].as_str().unwrap(),
            &consumer,
            "test.running",
        )
        .unwrap();
        let file = Path::new(consumer["worktree"].as_str().unwrap()).join("consumer.txt");
        fs::write(&file, "active unsaved edit").unwrap();
        let mut effect = super::tests::effect(&w, &lib);
        service
            .apply_development_effect(&mut w, &mut effect)
            .unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "active unsaved edit");
        assert_eq!(
            git(
                Path::new(consumer["worktree"].as_str().unwrap()),
                &["rev-parse", "HEAD"]
            )
            .unwrap(),
            consumer["result_commit"].as_str().unwrap()
        );
        assert_eq!(s.development_records("notification").unwrap().len(), 1);
        service
            .apply_development_effect(&mut w, &mut effect)
            .unwrap();
        assert_eq!(s.development_records("notification").unwrap().len(), 1);
        assert_eq!(
            s.development_records("notification").unwrap()[0]["state"],
            "pending-safe-point"
        );
    }
}

#[cfg(test)]
mod scope_lifecycle_tests {
    use super::*;
    #[test]
    fn operator_recovery_reaches_coordinator_without_changing_scope_or_replaying_commands() {
        let (_d, service, mut w) = super::tests::fixture();
        let s = &service.session;
        let key = w["work_id"].as_str().unwrap().to_owned();
        w["status"] = json!("attention");
        w["pending"] = Value::Null;
        w["attention"] = json!("Old host sandbox failed");
        s.development_save("workflow", &key, &w, "development.attention")
            .unwrap();
        let command = json!({"command_id":"host-fixed","action":"development.resume",
            "payload":{"work_id":key,"revision":w["revision"],
            "recovery_note":"Operator installed exact profile; sandbox pwd exited 0."}});
        let accepted = s.development_command(&command).unwrap();
        assert_eq!(accepted["status"], "accepted");
        assert_eq!(s.development_command(&command).unwrap(), accepted);
        let mut current = s.development_record("workflow", &key).unwrap().unwrap();
        assert_eq!(current["recoveries"].as_array().unwrap().len(), 1);
        assert_eq!(current["recoveries"][0]["source_command"], "host-fixed");
        assert_eq!(current["scope_base"], w["scope_base"]);
        assert_eq!(
            current["requirement_generation"],
            w["requirement_generation"]
        );
        assert_eq!(current["policy"], w["policy"]);
        assert_eq!(current["authorization"], w["authorization"]);
        service.tick_development_work(&mut current).unwrap();
        let operation = current["pending"]["operation_id"].as_str().unwrap();
        let directory = op_dir(s, &key, operation);
        let packet = read(&directory.join("packet.json")).unwrap();
        assert_eq!(
            packet["workflow"]["recoveries"][0]["note"],
            command["payload"]["recovery_note"]
        );
        assert!(!directory.join("active-host-attempt.json").exists());
        request_history::project(s).unwrap();
        let requests = request_history::requests(s).unwrap();
        let log = fs::read_to_string(requests[0]["log_path"].as_str().unwrap()).unwrap();
        assert!(log.contains("Operator installed exact profile; sandbox pwd exited 0."));
    }
    #[test]
    fn revised_scope_permission_keeps_request_and_commits_but_replaces_requirement_generation() {
        let (_d, service, w) = super::tests::fixture();
        let s = &service.session;
        let key = w["work_id"].as_str().unwrap();
        let scope = s.scope_detail(key).unwrap().unwrap();
        let mut proposal = scope["record"]["proposal"].clone();
        proposal["objective"] = json!("Test contract, actual linkage and clarified errors");
        let payload = json!({"base":scope["base"],"decision_revision":scope["decision_revision"],"authorization_command_id":scope["authorization_command_id"],"proposal":proposal,"rationale":"Explicit revised scope for test"});
        assert_eq!(
            s.scope_command(
                &json!({"command_id":"save-v2","action":"save_proposal_revision","payload":payload})
            )
            .unwrap()["status"],
            "accepted"
        );
        let revised = s.scope_detail(key).unwrap().unwrap();
        assert_eq!(s.scope_command(&json!({"command_id":"allow-v2","action":"authorize_scope","payload":{"base":revised["base"],"decision_revision":revised["decision_revision"],"authorization_command_id":revised["authorization_command_id"],"acknowledged_unresolved_sha256":hash(&json!([])).unwrap(),"operating_policy_sha256":hash(&policy(s).unwrap()).unwrap()}})).unwrap()["status"],"accepted");
        let current = s.development_record("workflow", key).unwrap().unwrap();
        assert_eq!(current["request_id"], w["request_id"]);
        assert_eq!(current["base_commit"], w["base_commit"]);
        assert_eq!(current["scope_base"], revised["base"]);
        assert_eq!(current["requirement_generation"], 1);
    }
    #[test]
    fn cancellation_records_positive_prehost_settlement_and_preserves_repository() {
        let (_d, service, w) = super::tests::fixture();
        let s = &service.session;
        let key = w["work_id"].as_str().unwrap();
        let actual = s.development_record("workflow", key).unwrap().unwrap();
        assert_eq!(s.development_command(&json!({"command_id":"cancel-work","action":"development.cancel","payload":{"work_id":key,"revision":actual["revision"]}})).unwrap()["status"],"accepted");
        let mut current = s.development_record("workflow", key).unwrap().unwrap();
        service.cancel_development_work(&mut current).unwrap();
        assert_eq!(current["status"], "cancelled");
        assert_eq!(
            git(&s.project, &["rev-parse", "HEAD"]).unwrap(),
            w["base_commit"].as_str().unwrap()
        );
        assert!(Path::new(w["integration_worktree"].as_str().unwrap()).exists());
        assert_eq!(request_history::requests(s).unwrap().len(), 1);
    }
}

#[cfg(test)]
mod coordinator_recovery_tests {
    use super::*;
    pub(super) fn fixture()->(tempfile::TempDir,Service,Value){
        let (dir,service,mut w)=super::tests::fixture();let s=&service.session;
        fs::write(s.project.join(".waywr/project.toml"),format!("schema_version = 2\nproject_key = \"{}\"\n[execution]\nhost = \"systemd-user-v1\"\n[conversation]\nschema_version = 5\n",s.key)).unwrap();
        crate::provider_selection::migrate_project(&s.project,&s.workflow.state,&s.key).unwrap();s.development_schema().unwrap();
        w["pending"]=Value::Null;s.development_save("workflow",strv(&w,"work_id").unwrap(),&w,"fixture.idle").unwrap();
        (dir,service,w)
    }
    #[test]
    fn recovery_preparation_waits_for_changed_cause_and_preserves_pause_authority_and_artifacts(){
        let (_dir,service,mut w)=fixture();let s=&service.session;let key=strv(&w,"work_id").unwrap().to_owned();
        let original=s.scope_detail(&key).unwrap().unwrap();let dir=op_dir(s,&key,"refused-preparation");
        let signature=recovery::preparation_signature(s,&w,&[]).unwrap();
        let proof=recovery::preparation_refusal(s,&w,&dir,&signature,"Fixture preparation refusal; no native invocation").unwrap();
        w["recovery"]=recovery::observe(&w,Cause::Preparation,&key,"refused-preparation","Preparation needs changed input",proof).unwrap();
        w["status"]=json!("attention");s.development_save("workflow",&key,&w,"development.recovery").unwrap();
        let before=w.clone();service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w,before);
        // A changed optional preference is a concrete preparation input, not new scope authority.
        let (p,rev)=crate::provider_selection::preferences(s).unwrap();let mut p=p.value();p["quality_preference"]=json!("Retain exact evidence");
        crate::provider_selection::save(s,&json!({"command_id":"fixture-preparation-change","action":"providers.save","payload":{"expected_revision":rev,"preferences":p}})).unwrap();
        s.workflow.connection().unwrap().execute("UPDATE session_metadata SET value=json_set(value,'$.phase','paused') WHERE name='lifecycle_v1'",[]).unwrap();
        service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w["status"],"attention");
        s.workflow.connection().unwrap().execute("UPDATE session_metadata SET value=json_set(value,'$.phase','running') WHERE name='lifecycle_v1'",[]).unwrap();
        service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w["status"],"running");assert_eq!(w["recovery"]["phase"],"resolved");
        assert_eq!(s.scope_detail(&key).unwrap().unwrap()["base"],original["base"]);assert_eq!(original["authorized"],true);
        assert!(dir.join("preparation-refusal.json").exists());assert!(!dir.join("active-host-attempt.json").exists());
        let saved=w.clone();service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w,saved);
        // A possibly sent attempt cannot be converted into preparation/nonlaunch proof.
        let ambiguous=op_dir(s,&key,"possibly-sent");fs::create_dir_all(ambiguous.join("attempts/a")).unwrap();fs::write(ambiguous.join("attempts/a/start-maybe-sent.json"),"{}").unwrap();
        assert!(recovery::preparation_refusal(s,&w,&ambiguous,&signature,"No receipt").is_err());
        w["recovery"]=before["recovery"].clone();w["requirement_generation"]=json!(999);assert!(!recovery::preparation_changed(s,&w,&[]).unwrap());
    }
    #[test]
    fn recovery_completed_check_adopts_through_attention_once_after_restart_without_launch(){
        let (_root,service,mut w)=fixture();let s=&service.session;let key=strv(&w,"work_id").unwrap().to_owned();let op="saved-check";
        let dir=op_dir(s,&key,op);let packet=json!({"development":true,"operation_kind":"development-check","operation_id":op,"run_id":key,"project_key":s.key});
        let result=json!({"checks":[{"success":true}],"passed":true});
        write(&dir.join("operation.json"),&json!({"schema_version":1,"operation_id":op,"packet_sha256":hash(&packet).unwrap(),"launch_token":"fixture"}),true).unwrap();
        write(&dir.join("packet.json"),&packet,true).unwrap();write(&dir.join("result.json"),&result,true).unwrap();
        write(&dir.join("claim.json"),&json!({"runner":{"fixture":true}}),true).unwrap();
        write(&dir.join("receipt.json"),&json!({"operation_id":op,"packet_sha256":hash(&packet).unwrap(),"launch_token":"fixture","runner":{"fixture":true},"status":"completed","result_sha256":execution::hash_file(&dir.join("result.json")).unwrap()}),true).unwrap();
        w["pending"]=json!({"operation_id":op,"kind":"check","checked_commit":w["head"],"scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"]});
        w["status"]=json!("attention");w["attention"]=json!("Fixture interrupted observation");
        s.development_save("workflow",&key,&w,"fixture.attention").unwrap();
        let reopened=Session::open_existing(&s.project,&s.workflow.state,&s.key,&s.release).unwrap();w=reopened.development_record("workflow",&key).unwrap().unwrap();
        service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w["checks"]["result"],result);assert!(w["pending"].is_null());assert_eq!(w["status"],"running");
        let before=w.clone();service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w,before);
        let c=s.workflow.connection().unwrap();assert_eq!(c.query_row("SELECT count(*) FROM session_events WHERE type='development.combined_checks'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        assert!(!dir.join("active-host-attempt.json").exists());
    }
    #[test]
    fn recovery_applied_integration_reconciles_attention_but_never_discards_dirty_or_uncertain_effects(){
        let (_root,service,mut w)=fixture();let s=&service.session;let key=strv(&w,"work_id").unwrap().to_owned();
        let base=strv(&w,"head").unwrap().to_owned();let u=super::tests::submission(s,&w,"one",&base,&[("one.txt","one")],json!([]));
        let e=super::tests::effect(&w,&u);let eid=strv(&e,"effect_id").unwrap();s.development_save("effect",eid,&e,"development.integration_intent").unwrap();
        let tree=PathBuf::from(strv(&w,"integration_worktree").unwrap());
        git(&tree,&["-c","user.name=Test","-c","user.email=test@localhost","merge","--no-ff","-m",&format!("Saved merge\n\nWaywr-Intent: {eid}"),strv(&u,"result_commit").unwrap()]).unwrap();
        w["status"]=json!("attention");s.development_save("workflow",&key,&w,"fixture.attention").unwrap();
        fs::write(tree.join("one.txt"),"unexpected edit").unwrap();service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w["status"],"attention");assert_eq!(fs::read_to_string(tree.join("one.txt")).unwrap(),"unexpected edit");
        fs::write(tree.join("one.txt"),"one").unwrap();let count=git(&tree,&["rev-list","--count","HEAD"]).unwrap();
        service.reconcile_development_attention(&mut w).unwrap();assert_eq!(w["status"],"running");assert_eq!(w["recovery"]["phase"],"resolved");assert_eq!(s.development_record("effect",eid).unwrap().unwrap()["status"],"recorded");
        service.reconcile_development_attention(&mut w).unwrap();assert_eq!(git(&tree,&["rev-list","--count","HEAD"]).unwrap(),count);
    }
}

#[cfg(test)]
mod recovery_progress_tests{
    use super::*;
    fn fixture()->(tempfile::TempDir,Service,Value){super::coordinator_recovery_tests::fixture()}
    #[test]
    fn recovery_reassessment_is_atomically_adopted_and_restart_does_not_repeat_it(){
        let (_dir,service,mut w)=fixture();let s=&service.session;let key=strv(&w,"work_id").unwrap().to_owned();
        w["pending"]=json!({"operation_id":"reassess-once"});s.development_save("workflow",&key,&w,"fixture.pending").unwrap();
        let snapshot=json!({"sha256":"same-code-and-evidence"});let approach=json!({"action":"assign","paths":["model.rs"]});
        assert!(!recovery::reassess(&mut w,&snapshot,&approach,false).unwrap());assert!(recovery::reassess(&mut w,&snapshot,&approach,false).unwrap());w["pending"]=Value::Null;
        s.development_commit_decision(&w,&[],"reassess-once").unwrap();assert!(s.development_commit_decision(&w,&[],"reassess-once").is_err());
        let restarted=Session::open_existing(&s.project,&s.workflow.state,&s.key,&s.release).unwrap();let saved=restarted.development_record("workflow",&key).unwrap().unwrap();assert_eq!(saved["progress"],w["progress"]);assert!(saved["pending"].is_null());
        assert_eq!(s.workflow.connection().unwrap().query_row("SELECT count(*) FROM session_events WHERE type='development.reassessment'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        request_history::project(s).unwrap();let path=request_history::requests(s).unwrap()[0]["log_path"].as_str().unwrap().to_owned();let log=fs::read_to_string(&path).unwrap();assert!(log.contains("Equivalent work"));assert!(log.contains("wake_condition"));assert!(log.contains("development.reassessment"));let bytes=fs::read(&path).unwrap();request_history::project(s).unwrap();assert_eq!(bytes,fs::read(path).unwrap());
    }
    fn ready(s:&Session,w:&Value,name:&str,profile:&str)->Value{
        json!({"unit_id":format!("{}-{name}",w["work_id"].as_str().unwrap()),"work_id":w["work_id"],"request_id":w["request_id"],"name":name,"title":name,"instructions":"Existing authorized fixture assignment","role":"implementation","requested_profile":profile,"status":"planned","paths":[format!("{name}.txt")],"start_after":[],"accept_after":[],"attempt_revision":1,"branch":format!("waywr/{}/tasks/{name}/work",w["work_id"].as_str().unwrap()),"worktree":worktree_path(s,w["work_id"].as_str().unwrap(),name).unwrap(),"base_commit":null})
    }
    #[test]
    fn recovery_scoped_preparation_block_keeps_unaffected_assignment_available_and_no_call_launched(){
        let (_dir,service,w)=fixture();let s=&service.session;
        for (name,profile) in [("a-blocked","missing-profile"),("b-independent","")]{let u=ready(s,&w,name,profile);s.development_save("unit",strv(&u,"unit_id").unwrap(),&u,"fixture.ready").unwrap();}
        service.start_development_units(&w).unwrap();let all=units(s,&w).unwrap();let bad=all.iter().find(|u|u["name"]=="a-blocked").unwrap();let good=all.iter().find(|u|u["name"]=="b-independent").unwrap();
        assert_eq!(bad["status"],"planned");assert_eq!(bad["recovery"]["cause"],"provider_grant");assert_eq!(good["status"],"running");assert!(good["selection"].is_object());assert!(!op_dir(s,strv(&w,"work_id").unwrap(),strv(good,"operation_id").unwrap()).join("active-host-attempt.json").exists());
        let display=s.development_snapshot().unwrap();assert_eq!(display["workflows"][0]["recovery"]["scope"],"assignment");assert_eq!(display["workflows"][0]["recovery"]["cause"],"provider_grant");assert!(w["recovery"].is_null(),"Derived display cannot rewrite workflow authority");
        assert_eq!(s.scope_detail(strv(&w,"work_id").unwrap()).unwrap().unwrap()["authorized"],true);
        // A saved preference may make that exact already-authorized profile
        // selectable. Rechecking the grant must not require operational resume.
        let (p,revision)=crate::provider_selection::preferences(s).unwrap();let mut p=p.value();p["profiles"]["missing-profile"]=p["profiles"]["codex"].clone();p["roles"]["implementation"]["allowed"].as_array_mut().unwrap().push(json!("missing-profile"));
        crate::provider_selection::save(s,&json!({"command_id":"fixture-enable-same-profile","action":"providers.save","payload":{"expected_revision":revision,"preferences":p}})).unwrap();
        service.start_development_units(&w).unwrap();let restored=s.development_record("unit",strv(bad,"unit_id").unwrap()).unwrap().unwrap();assert_eq!(restored["status"],"running");assert_eq!(restored["recovery"]["phase"],"resolved");assert_eq!(restored["selection"]["profile_sha256"],good["selection"]["profile_sha256"]);assert_eq!(restored["recovery"]["profile_sha256"],Value::Null,"Unknown failed selection must not be labelled with the coordinator profile");
    }
    #[test]
    fn recovery_unlaunched_workspace_waits_for_exact_clean_owned_base_before_preparing(){
        let (_dir,service,w)=fixture();let s=&service.session;let u=ready(s,&w,"owned","");let path=PathBuf::from(strv(&u,"worktree").unwrap());let branch=strv(&u,"branch").unwrap();
        ensure_tree(s,&path,branch,strv(&w,"head").unwrap()).unwrap();git(&path,&["checkout","-b","fixture-unexpected-owner"]).unwrap();
        s.development_save("unit",strv(&u,"unit_id").unwrap(),&u,"fixture.ready").unwrap();service.start_development_units(&w).unwrap();
        let blocked=s.development_record("unit",strv(&u,"unit_id").unwrap()).unwrap().unwrap();assert_eq!(blocked["recovery"]["cause"],"workspace");assert_eq!(blocked["status"],"planned");
        service.start_development_units(&w).unwrap();assert_eq!(s.development_record("unit",strv(&u,"unit_id").unwrap()).unwrap().unwrap(),blocked);
        git(&path,&["checkout",branch]).unwrap();fs::write(path.join("unexpected.txt"),"preserved").unwrap();service.start_development_units(&w).unwrap();assert_eq!(s.development_record("unit",strv(&u,"unit_id").unwrap()).unwrap().unwrap()["status"],"planned");assert_eq!(fs::read_to_string(path.join("unexpected.txt")).unwrap(),"preserved");
        // Remove only this fixture-created edit; production recovery never cleans it.
        fs::remove_file(path.join("unexpected.txt")).unwrap();service.start_development_units(&w).unwrap();let prepared=s.development_record("unit",strv(&u,"unit_id").unwrap()).unwrap().unwrap();assert_eq!(prepared["status"],"running");assert_eq!(prepared["recovery"]["phase"],"resolved");assert!(!op_dir(s,strv(&w,"work_id").unwrap(),strv(&prepared,"operation_id").unwrap()).join("active-host-attempt.json").exists());
    }
    #[test]
    fn recovery_persisted_capacity_wait_does_not_reserve_inference_or_change_provider(){
        let (_dir,service,mut w)=fixture();let s=&service.session;
        let mut u=ready(s,&w,"waiting","");u["status"]=json!("capacity-waiting");u["retry_at"]=json!(now()+3600.0);u["wait_reason"]=json!({"retry_after_secs":3600,"source":"settled fixture receipt"});s.development_save("unit",strv(&u,"unit_id").unwrap(),&u,"fixture.capacity").unwrap();
        let bound=crate::provider_selection::current(s).unwrap();let sequence=w["coordinator_sequence"].clone();
        for _ in 0..3{service.tick_development_work(&mut w).unwrap();assert!(w["pending"].is_null());}
        let restarted=Session::open_existing(&s.project,&s.workflow.state,&s.key,&s.release).unwrap();let saved=restarted.development_record("unit",strv(&u,"unit_id").unwrap()).unwrap().unwrap();assert_eq!(saved,u);assert_eq!(w["coordinator_sequence"],sequence);assert_eq!(bound,crate::provider_selection::current(s).unwrap());
    }
}

#[cfg(test)]
mod convergence_tests {
    use super::*;
    fn planned(w:&Value,name:&str)->Value {
        json!({"unit_id":format!("{}-{name}",w["work_id"].as_str().unwrap()),"work_id":w["work_id"],"name":name,"role":"review","instructions":"Review exact frozen changes","status":"planned","requirement_generation":w["requirement_generation"],"attempt_revision":1,"review_target_names":["implementation"],"resolution_target_names":["original-review"],"start_after":[],"accept_after":[]})
    }
    #[test]
    fn convergence_readable_names_are_bounded_isolated_and_legacy_compatible() {
        let a=request_namespace(&"Simple ledger TUI with tag management ".repeat(200),"request-a");
        assert!(a.starts_with("waywr/simple-ledger-tui-with-tag-management"));
        assert!(a.len()<=67);assert!(valid_namespace(&a,"request-a"));assert!(!valid_namespace(&a,"request-b"));
        assert_ne!(a,request_namespace("Simple ledger TUI with tag management","request-b"));
        assert!(valid_namespace("waywr/t-legacy","t-legacy"));
        assert!(request_namespace("../雪/../","id").starts_with("waywr/request-"));
        assert!(!valid_namespace("waywr/../../other","id"));
        let (_d,service,w)=super::tests::fixture();let s=&service.session;
        let namespace=w["branch_namespace"].as_str().unwrap();let name="readable-worker";
        let branch=format!("{namespace}/tasks/{name}/work");let tree=worktree_path(s,strv(&w,"work_id").unwrap(),name).unwrap();
        ensure_tree(s,&tree,&branch,strv(&w,"head").unwrap()).unwrap();
        let packet=json!({"run_id":w["work_id"],"worktree":tree,"git_common_dir":git(&s.project,&["rev-parse","--path-format=absolute","--git-common-dir"]).unwrap(),"assignment":{"name":name,"branch":branch,"branch_namespace":namespace}});
        let roots=worker_write_roots(&packet).unwrap();assert!(roots.iter().any(|p|p.ends_with(format!("refs/heads/{namespace}/tasks/{name}"))));
        let mut wrong=packet.clone();wrong["assignment"]["branch_namespace"]=json!(request_namespace("other","other"));assert!(worker_write_roots(&wrong).is_err());
    }
    fn legacy(service:&Service,w:&mut Value) {
        let old=format!("waywr/{}/integration",w["work_id"].as_str().unwrap());
        git(Path::new(strv(w,"integration_worktree").unwrap()),&["branch","-m",&old]).unwrap();
        w["integration_branch"]=json!(old);for k in ["branch_namespace","result_branch"]{w.as_object_mut().unwrap().remove(k);}
        service.session.development_save("workflow",strv(w,"work_id").unwrap(),w,"fixture.legacy").unwrap();
    }
    #[test]
    fn convergence_result_alias_preserves_legacy_worktree_reconciles_cas_and_fences_collision() {
        let (_d,service,mut w)=super::tests::fixture();let s=&service.session;legacy(&service,&mut w);
        let old=w["integration_branch"].clone();let first=w["head"].clone();let root_main=git(&s.project,&["rev-parse","main"]).unwrap();
        service.sync_result_branch(&mut w).unwrap();let alias=strv(&w,"result_branch").unwrap().to_owned();
        assert!(alias.starts_with("waywr/socket-example-"));assert_eq!(git(&s.project,&["rev-parse",&alias]).unwrap(),first);
        assert_eq!(git(Path::new(strv(&w,"integration_worktree").unwrap()),&["branch","--show-current"]).unwrap(),old);
        let saved=w.clone();service.sync_result_branch(&mut w).unwrap();assert_eq!(w,saved);
        let tree=Path::new(strv(&w,"integration_worktree").unwrap());
        git(tree,&["-c","user.name=Fixture","-c","user.email=fixture@localhost","commit","--allow-empty","-m","fixture combined revision"]).unwrap();w["head"]=json!(git(tree,&["rev-parse","HEAD"]).unwrap());
        w["result_branch_intent"]=json!({"branch":alias,"expected":first,"commit":w["head"],"authority_sha256":recovery::authority(&w).unwrap()});
        s.development_save("workflow",strv(&w,"work_id").unwrap(),&w,"fixture.intent").unwrap();
        git(&s.project,&["update-ref",&format!("refs/heads/{alias}"),strv(&w,"head").unwrap(),first.as_str().unwrap()]).unwrap();
        service.sync_result_branch(&mut w).unwrap();assert!(w["result_branch_intent"].is_null());assert_eq!(w["result_branch_head"],w["head"]);
        assert_eq!(git(&s.project,&["rev-parse","main"]).unwrap(),root_main);
        git(&s.project,&["update-ref",&format!("refs/heads/{alias}"),first.as_str().unwrap()]).unwrap();
        assert!(service.sync_result_branch(&mut w).unwrap_err().contains("externally"));assert_eq!(git(&s.project,&["rev-parse",&alias]).unwrap(),first);
        request_history::project(s).unwrap();let requests=request_history::requests(s).unwrap();let log=fs::read_to_string(requests[0]["log_path"].as_str().unwrap()).unwrap();assert!(log.contains("development.result_branch_updated"));assert!(log.contains(&alias));
    }
    #[test]
    fn convergence_review_refusal_ignores_unrelated_progress_and_keeps_immutable_proof() {
        let (_d,service,w)=super::coordinator_recovery_tests::fixture();let s=&service.session;let mut u=planned(&w,"obsolete-review");
        let mut all=vec![u.clone(),json!({"name":"implementation","unit_id":"impl","result_commit":"a".repeat(40)}),json!({"name":"original-review","unit_id":"old","review":{"requires_changes":true},"finding_ledger":[{"finding_id":"f","target_commit":"b".repeat(40),"disposition":"blocking"}]})];
        let signature=recovery::unit_preparation_signature(s,&w,&u,&all).unwrap();
        let dir=op_dir(s,strv(&w,"work_id").unwrap(),"fixture-target-mismatch");let proof=recovery::preparation_refusal_kind(s,&w,&dir,&signature,"Clarification target commit differs from its finding","assignment-v2").unwrap();
        let bytes=fs::read(dir.join("preparation-refusal.json")).unwrap();
        u["recovery"]=recovery::observe_unit(&w,&u,Cause::Preparation,"fixture-target-mismatch","Clarification target commit differs from its finding",proof).unwrap();u["preparation_revision"]=json!(40);
        let mut changed=w.clone();changed["head"]=json!("c".repeat(40));changed["checks"]=json!({"passed":true});all.push(json!({"name":"unrelated","status":"integrated","result_commit":"d".repeat(40)}));
        assert!(!recovery::unit_preparation_changed(s,&changed,&u,&all).unwrap());
        all[1]["result_commit"]=json!("e".repeat(40));assert!(recovery::unit_preparation_changed(s,&changed,&u,&all).unwrap());
        assert_eq!(fs::read(dir.join("preparation-refusal.json")).unwrap(),bytes);
        let retired=retire_assignments(s,&w,&[u.clone()],&["obsolete-review".into()],"coordinator-retire","Replaced by exact-source clarification and full repair review").unwrap();
        assert_eq!(retired[0]["status"],"superseded");assert_eq!(retired[0]["recovery"]["phase"],"resolved");assert_eq!(retired[0]["resolution_target_names"],u["resolution_target_names"]);
        assert_eq!(fs::read(dir.join("preparation-refusal.json")).unwrap(),bytes);
        fs::create_dir(dir.join("attempts")).unwrap();assert!(retire_assignments(s,&w,&[u],&["obsolete-review".into()],"coordinator-retire","obsolete").is_err());
    }
    #[test]
    fn convergence_saved_retirement_adopts_once_and_history_retains_the_decision() {
        let (_d,service,mut w)=super::coordinator_recovery_tests::fixture();let s=&service.session;let key=strv(&w,"work_id").unwrap().to_owned();let u=planned(&w,"obsolete");
        s.development_save("unit",strv(&u,"unit_id").unwrap(),&u,"fixture.planned").unwrap();
        let op="fixture-retire-receipt";let dir=op_dir(s,&key,op);
        let packet=json!({"development":true,"operation_kind":"development-coordinator","operation_id":op,"run_id":key,"project_key":s.key,"convergence_contract":1,"units":[{"unit_id":u["unit_id"],"source_record":{"sha256":hash(&u).unwrap()}}]});
        let result=json!({"schema_version":1,"assistant_text":"Retiring an obsolete unlaunched assignment; review and human gates remain","action":"retire","tasks":[],"units":["obsolete"],"checks":[],"resolved_obligations":[],"reason":"Replacement exact-target review is retained"});
        write(&dir.join("operation.json"),&json!({"schema_version":1,"operation_id":op,"packet_sha256":hash(&packet).unwrap(),"launch_token":"fixture"}),true).unwrap();
        write(&dir.join("packet.json"),&packet,true).unwrap();write(&dir.join("result.json"),&result,true).unwrap();write(&dir.join("claim.json"),&json!({"runner":{"fixture":true}}),true).unwrap();
        write(&dir.join("receipt.json"),&json!({"operation_id":op,"packet_sha256":hash(&packet).unwrap(),"launch_token":"fixture","runner":{"fixture":true},"status":"completed","result_sha256":execution::hash_file(&dir.join("result.json")).unwrap()}),true).unwrap();
        w["pending"]=json!({"operation_id":op,"kind":"coordinate","scope_base":w["scope_base"],"requirement_generation":w["requirement_generation"]});
        s.development_save("workflow",&key,&w,"fixture.pending").unwrap();service.poll_development_coordinator(&mut w).unwrap();
        let reopened=Session::open_existing(&s.project,&s.workflow.state,&s.key,&s.release).unwrap();w=reopened.development_record("workflow",&key).unwrap().unwrap();
        assert!(w["pending"].is_null());assert_eq!(reopened.development_record("unit",strv(&u,"unit_id").unwrap()).unwrap().unwrap()["status"],"superseded");
        service.tick_development_work(&mut w).unwrap();let c=s.workflow.connection().unwrap();assert_eq!(c.query_row("SELECT count(*) FROM session_events WHERE type='development.assignment_retired'",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        request_history::project(s).unwrap();let requests=request_history::requests(s).unwrap();let log=fs::read_to_string(requests[0]["log_path"].as_str().unwrap()).unwrap();assert!(log.contains("Replacement exact-target review is retained"));assert!(log.contains("development.assignment_retired"));assert!(!dir.join("active-host-attempt.json").exists());
    }
    #[test]
    fn convergence_retirement_preserves_obligations_and_refuses_execution_or_stranded_dependencies() {
        let (_d,service,mut w)=super::tests::fixture();let s=&service.session;let u=planned(&w,"obsolete");w["obligations"]=json!([{"name":"human visual verification","status":"open","owner":u["unit_id"]}]);
        let retired=retire_assignments(s,&w,&[u.clone()],&["obsolete".into()],"saved-decision","Changed approach").unwrap();
        assert_eq!(w["obligations"][0]["status"],"open");assert_eq!(retired[0]["status"],"superseded");
        for (field,value) in [("status",json!("running")),("operation_id",json!("may-have-sent")),("attempt_revision",json!(2)),("result_commit",json!("a".repeat(40))),("finding_ledger",json!([{"disposition":"blocking"}]))] {
            let mut unsafe_u=u.clone();unsafe_u[field]=value;assert!(retire_assignments(s,&w,&[unsafe_u],&["obsolete".into()],"saved-decision","obsolete").is_err(),"{field}");
        }
        let dependent=json!({"name":"consumer","status":"planned","start_after":["obsolete"]});
        assert!(retire_assignments(s,&w,&[u.clone(),dependent],&["obsolete".into()],"saved-decision","obsolete").unwrap_err().contains("strand"));
        let mut changed=w.clone();changed["requirement_generation"]=json!(99);assert!(retire_assignments(s,&changed,&[u.clone()],&["obsolete".into()],"saved-decision","obsolete").is_err());
        let contract=json!({"operation_kind":"development-coordinator","convergence_contract":1});assert!(result_schema(&contract)["properties"]["action"]["enum"].as_array().unwrap().contains(&json!("retire")));
        assert!(!result_schema(&json!({"operation_kind":"development-coordinator"}))["properties"]["action"]["enum"].as_array().unwrap().contains(&json!("retire")));
        let state=convergence(s,&w,&[u]);assert_eq!(state["open_obligation_names"][0],"human visual verification");assert_ne!(integration_checkout(s,&w)["directory"],json!(s.project));
    }
}

#[cfg(test)] mod ordinary_candidate_tests {
 use super::*;
 #[test] fn ordinary_candidate_exact_feedback_checkout_and_retirement(){
 let(_d,service,mut w)=super::coordinator_recovery_tests::fixture();let s=&service.session;
 w["policy"]["project_workflow"]=json!({"human_validation":true,"completion":"checked-local","retention":"retire-clean-owned"});w["pending"]=Value::Null;w["checks"]=json!({"commit":w["head"],"result":{"passed":true,"checks":[{"success":true}]}});w["obligations"]=json!([{"name":"policy:human-validation","status":"open"}]);
 s.workflow.connection().unwrap().execute("UPDATE development_records SET record_json=?1 WHERE kind='workflow' AND id=?2",rusqlite::params![serde_json::to_string(&w).unwrap(),w["work_id"].as_str()]).unwrap();
 let delivery=json!({"environment":"Fixture Git checkout","prerequisites":[],"build":["make"],"run":[],"inspect":[],"expected":["Recorded checks pass"],"shutdown":[],"notes":[]});let c=crate::candidate::prepare(s,&mut w,&delivery,"fixture").unwrap().unwrap();let id=c["candidate_id"].as_str().unwrap();s.development_save("candidate",id,&c,"fixture").unwrap();s.development_save("workflow",w["work_id"].as_str().unwrap(),&w,"fixture").unwrap();
 assert!(crate::candidate::start_handoff(s,&mut w).unwrap());let tree=Path::new(w["integration_worktree"].as_str().unwrap());assert!(git(tree,&["branch","--show-current"]).unwrap().is_empty());assert!(crate::candidate::writes_held(&w));
 let body=|command:&str,action:&str|json!({"command_id":command,"action":action,"payload":{"work_id":w["work_id"],"candidate_id":id,"commit":w["head"],"text":"validated exact candidate"}});
 let mut stale=body("candidate-stale","candidate.accept");stale["payload"]["commit"]=json!("a".repeat(40));assert!(crate::candidate::command(s,&stale).is_err());
 crate::candidate::command(s,&body("candidate-present","candidate.present")).unwrap();let take=body("candidate-checkout","candidate.checkout");crate::candidate::command(s,&take).unwrap();assert_eq!(git(&s.project,&["branch","--show-current"]).unwrap(),w["integration_branch"]);
 crate::candidate::command(s,&body("candidate-end","candidate.end_validation")).unwrap();let ended=s.development_record("workflow",w["work_id"].as_str().unwrap()).unwrap().unwrap();assert!(!crate::candidate::accepted(&ended));assert!(!crate::candidate::writes_held(&ended));crate::candidate::command(s,&body("candidate-checkout-again","candidate.checkout")).unwrap();
 let accept=body("candidate-accept","candidate.accept");let result=crate::candidate::command(s,&accept).unwrap();assert_eq!(result,crate::candidate::command(s,&accept).unwrap());let current=s.development_record("workflow",w["work_id"].as_str().unwrap()).unwrap().unwrap();assert!(crate::candidate::accepted(&current));assert!(!crate::candidate::writes_held(&current));assert!(git(&s.project,&["branch","--show-current"]).unwrap().is_empty());assert_eq!(git(tree,&["branch","--show-current"]).unwrap(),w["integration_branch"]);
 }
}
#[cfg(test)]mod ordinary_resource_tests{
 use super::*;
 #[test]fn ordinary_resource_retirement_preserves_git_objects_and_unknown_files(){
 let(_d,service,mut w)=super::coordinator_recovery_tests::fixture();let s=&service.session;w["policy"]["project_workflow"]=json!({"retention":"retire-clean-owned","generated_outputs":["target"]});let key=w["work_id"].as_str().unwrap();let tree=worktree_path(s,key,"temporary").unwrap();let branch=format!("{}/temporary",w["branch_namespace"].as_str().unwrap());let base=w["head"].as_str().unwrap();ensure_tree(s,&tree,&branch,base).unwrap();
 let excluded=PathBuf::from(git(&tree,&["rev-parse","--path-format=absolute","--git-path","info/exclude"]).unwrap());fs::write(excluded,"target/\n").unwrap();
 fs::create_dir_all(tree.join("target/debug")).unwrap();fs::write(tree.join("target/preexisting"),"preserve preexisting").unwrap();
 let output=op_dir(s,key,"owned-output-inventory");crate::project_resources::begin_generated(&tree,&w["policy"],&output,&Value::Null).unwrap();
 fs::write(tree.join("target/debug/program"),"generated build product").unwrap();fs::write(tree.join("target/changed-after"),"generated then modified").unwrap();
 let previous=crate::project_resources::finish_generated(&tree,&w["policy"],&output).unwrap();fs::write(tree.join("target/changed-after"),"user replacement").unwrap();fs::write(tree.join("target/unknown-after"),"unknown").unwrap();
 // A subsequent authorized build can reuse/overwrite its own known products.
 let again=op_dir(s,key,"owned-output-second-check");crate::project_resources::begin_generated(&tree,&w["policy"],&again,&previous).unwrap();fs::write(tree.join("target/debug/program"),"rebuilt known output").unwrap();let generated=crate::project_resources::finish_generated(&tree,&w["policy"],&again).unwrap();
 let unit=json!({"unit_id":"owned-temporary","work_id":key,"request_id":w["request_id"],"name":"temporary","status":"superseded","role":"implementation","worktree":tree,"branch":branch,"resource_owned":true,"resource_intent":{"path":tree,"branch":branch},"generated_resources":generated,"retirement":{"proof":{"source":"saved first revision before reservation"}},"recovery":null});s.development_save("unit","owned-temporary",&unit,"fixture").unwrap();
 assert!(!crate::project_resources::valid_output_path("target/*"));assert!(!crate::project_resources::valid_output_path("node_modules"));assert!(!crate::project_resources::valid_output_path("../outside"));
 let mut no_outputs=w.clone();no_outputs["policy"]["project_workflow"]["generated_outputs"]=json!([]);crate::project_resources::retire(s,&no_outputs).unwrap();assert!(tree.join("target/debug/program").exists());
 fs::write(tree.join("untracked.txt"),"preserve this").unwrap();crate::project_resources::retire(s,&w).unwrap();assert_eq!(s.development_record("cleanup","owned-temporary").unwrap().unwrap()["status"],"retained");assert!(tree.join("untracked.txt").exists());assert!(!tree.join("target/debug/program").exists());assert!(tree.join("target/preexisting").exists());assert_eq!(fs::read_to_string(tree.join("target/changed-after")).unwrap(),"user replacement");assert!(tree.join("target/unknown-after").exists());
 // Fixture owns these deliberately retained files; normal retirement never removes them.
 fs::remove_file(tree.join("untracked.txt")).unwrap();for name in ["preexisting","changed-after","unknown-after"]{fs::remove_file(tree.join("target").join(name)).unwrap();}fs::remove_dir(tree.join("target")).unwrap();
 crate::project_resources::retire(s,&w).unwrap();let record=s.development_record("cleanup","owned-temporary").unwrap().unwrap();assert_eq!(record["status"],"retired");assert!(!tree.exists());assert!(crate::project_workflow::ref_head(&s.project,&branch).is_err());assert_eq!(git(&s.project,&["rev-parse",record["archive_ref"].as_str().unwrap()]).unwrap(),base);crate::project_resources::retire(s,&w).unwrap();assert_eq!(s.development_record("cleanup","owned-temporary").unwrap().unwrap(),record);
 }
}
