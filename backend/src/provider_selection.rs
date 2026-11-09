//! Project-local routing preferences. Saved preferences are never execution authority.
use crate::{
    codec::canonical_sha256,
    conversation_store::{commit, encode, event as journal_event, prior},
    execution::{read, write},
    session_store::Session,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn event(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    task: Option<&str>,
    body: &Value,
) -> Result<(i64, u64), String> {
    let active: Option<String> = if task.is_none() {
        tx.query_row("SELECT id FROM development_records WHERE kind='workflow' AND json_extract(record_json,'$.status') NOT IN ('delivered','cancelled') LIMIT 1",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?
    } else {
        None
    };
    let task = task.or(active.as_deref());
    let out = journal_event(tx, kind, task, body)?;
    if let Some(task) = task {
        let request:Option<String>=tx.query_row("SELECT json_extract(record_json,'$.request_id') FROM development_records WHERE kind='workflow' AND id=?1",[task],|r|r.get(0)).optional().map_err(|e|e.to_string())?.flatten();
        if let Some(request) = request {
            crate::request_history::tag(
                tx,
                out.0,
                &request,
                "project provider settings / service",
            )?;
        }
    }
    if task.is_none() {
        if let Some(subject) = body["subject_id"].as_str() {
            let request: Option<String> = tx
                .query_row(
                    "SELECT request_id FROM request_history_requests WHERE subject_id=?1",
                    [subject],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(request) = request {
                crate::request_history::tag(
                    tx,
                    out.0,
                    &request,
                    "project provider settings / service",
                )?;
            }
        }
    }
    Ok(out)
}
pub const ROLES: [&str; 3] = ["coordinator", "implementation", "review"];
pub fn narrow_routing(prefs:&Preferences,role:&str,name:&str)->Result<Value,String>{
    if !ROLES.contains(&role)||!prefs.role_allowed(role).contains(&name.to_owned()){return Err("Profile is outside this configured role".into())}
    let profile=prefs.profile_value(name)?;let hash=digest(&profile)?;
    let mut routing=json!({"profiles":{},"roles":{"coordinator":[],"implementation":[],"review":[]}});
    routing["profiles"][&hash]=json!({"name":name,"profile":profile});routing["roles"][role]=json!([hash]);Ok(routing)
}
pub fn digest(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
fn empty_options()->Value { json!({}) }
fn default_context() -> usize {
    96 * 1024
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub provider: String,
    #[serde(default)]
    pub binary: String,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub transport: Option<Value>,
    pub model: String,
    pub effort: String,
    #[serde(default = "empty_options")]
    pub options: Value,
    #[serde(default)]
    pub binary_sha256: Option<String>,
    #[serde(default = "default_context")]
    pub context_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePreference {
    pub default: String,
    pub allowed: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u64,
    pub profile: String,
    pub profiles: BTreeMap<String, Profile>,
    pub coordinator: String,
    pub worker_default: String,
    pub roles: BTreeMap<String, RolePreference>,
    pub fallback: BTreeMap<String, Vec<String>>,
    pub concurrency: u64,
    pub dependency_network: bool,
    pub main_policy: String,
    #[serde(default)]
    pub quality_preference: String,
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
/// Version strings are diagnostic only; actual required options define the interface.
pub fn native_interface(provider:&str,binary:&Path)->Result<(),String>{
    let root=metadata_help(binary,&[])?;
    let require=|help:&str,options:&[&str]|->Result<(),String>{
        for option in options{
            if !help.split(|c:char|!(c.is_ascii_alphanumeric()||c=='-')).any(|token|token==*option){
                return Err(format!("Selected {provider} client is missing required interface option {option}"));
            }
        }
        Ok(())
    };
    match provider{
        "codex"=>{
            require(&root,&["--no-daemon","--ask-for-approval","--disable"])?;
            let exec=metadata_help(binary,&["exec"])?;
            require(&exec,&["--ignore-user-config","--ignore-rules","--ephemeral","--skip-git-repo-check",
                "--json","--color","--model","--config","--sandbox","--output-schema","--output-last-message","--cd","--add-dir"])
        },
        "claude"=>require(&root,&["--print","--safe-mode","--restricted","--disable-slash-commands",
            "--setting-sources","--strict-mcp-config","--mcp-config","--tools","--permission-mode","--permission-prompts",
            "--settings","--model","--no-session-persistence","--output-format","--verbose",
            "--include-partial-messages","--json-schema","--system-prompt","--effort"]),
        _=>Err("No native adapter for selected provider".into()),
    }
}
fn metadata_help(binary:&Path,arguments:&[&str])->Result<String,String>{
    let output=std::process::Command::new("timeout").arg("3s").arg(binary).args(arguments).arg("--help")
        .env("DISABLE_AUTOUPDATER","1").env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC","1")
        .stdin(std::process::Stdio::null()).output().map_err(|e|format!("Cannot inspect client interface: {e}"))?;
    if !output.status.success(){return Err("Client interface inspection failed or timed out; no model started".into());}
    Ok(String::from_utf8_lossy(&output.stdout).chars().take(256*1024).collect())
}
pub fn client_version(binary:&Path)->Option<String>{
    std::process::Command::new("timeout").arg("3s").arg(binary).arg("--version")
        .env("DISABLE_AUTOUPDATER","1").env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC","1")
        .stdin(std::process::Stdio::null()).output().ok().filter(|o|o.status.success())
        .map(|o|String::from_utf8_lossy(&o.stdout).trim().chars().take(300).collect())
}
/// The configured locator remains approval metadata. Freeze its current resolution
/// separately for each new attempt; never refresh a runner's already-frozen client.
pub fn with_client(mut execution:Value)->Value{
    execution.as_object_mut().unwrap().remove("client");
    execution.as_object_mut().unwrap().remove("client_error");
    match crate::model_runtime::freeze(execution.clone()) {Ok(value)=>value,Err(reason)=>{execution["client_error"]=json!(reason);execution}}
}
pub fn native_binary(execution:&Value)->Result<&str,String>{
    if let Some(reason)=execution["client_error"].as_str(){return Err(reason.into());}
    execution["client"]["binary"].as_str().or_else(||execution["binary"].as_str()).ok_or_else(||"Provider binary missing".into())
}
pub fn efforts(_provider: &str, _model: &str) -> Vec<&'static str> { vec!["provider_default"] }
impl Preferences {
    pub fn parse(value: &Value) -> Result<Self, String> {
        if value["schema_version"]!=3{return Err("Native provider profiles are retired; configure shared model-runtime profiles (schema 3). Project files/history are preserved".into())}
        let p: Self = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if p.schema_version != 3
            || p.profile != "development-profiles-v1"
            || p.profiles.is_empty()
            || p.profiles.len() > 32
            || !(1..=16).contains(&p.concurrency)
            || p.main_policy != "checked-fast-forward"
            || p.quality_preference.len() > 2000
        {
            return Err("Invalid project routing preferences".into());
        }
        for (name, profile) in &p.profiles {
            if !valid_name(name)
                || profile.provider.trim().is_empty() || profile.provider.len()>4096 || profile.provider.chars().any(char::is_control)
                || !Path::new(&profile.binary).is_absolute()
                || profile.model.trim().is_empty()
                || profile.model.len() > 4096
                || profile.model.chars().any(char::is_control)
                || !profile.options.is_object()
                || profile.transport.as_ref().and_then(|v|v["entry"].as_str()).is_none_or(|v|v!="bundled"&&!Path::new(v).is_absolute())
                || !(16 * 1024..=crate::development_context::MEMORY_LIMIT).contains(&profile.context_bytes)
                || profile
                    .binary_sha256
                    .as_ref()
                    .is_some_and(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err(format!(
                    "Invalid shared-runtime profile {name}: choose provider/model, runtime entry and JSON options"
                ));
            }
        }
        let exists = |name: &str| p.profiles.contains_key(name);
        if !exists(&p.coordinator) || !exists(&p.worker_default) {
            return Err("Default profile does not exist".into());
        }
        for (role, r) in &p.roles {
            if !ROLES.contains(&role.as_str())
                || !exists(&r.default)
                || !r.allowed.contains(&r.default)
                || r.allowed.is_empty()
                || r.allowed.len() > 32
                || r.allowed.iter().any(|v| !exists(v))
            {
                return Err(format!("Invalid role preference {role}"));
            }
        }
        if !p.role_allowed("coordinator").contains(&p.coordinator) {
            return Err("Coordinator must belong to its configured allowed set".into());
        }
        for (role, alternatives) in &p.fallback {
            if alternatives
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != alternatives.len()
            {
                return Err("Fallback alternatives must be ordered unique profile names".into());
            }
            if !ROLES.contains(&role.as_str())
                || alternatives.len() > 32
                || alternatives
                    .iter()
                    .any(|v| !p.role_allowed(role).contains(v))
            {
                return Err(format!(
                    "Fallback for {role} must name explicitly allowed profiles"
                ));
            }
        }
        Ok(p)
    }
    pub fn value(&self) -> Value {
        serde_json::to_value(self).expect("serializable preferences")
    }
    pub fn role_default(&self, role: &str) -> String {
        if role == "coordinator" {
            self.coordinator.clone()
        } else {
            self.roles
                .get(role)
                .map(|r| r.default.clone())
                .unwrap_or(self.worker_default.clone())
        }
    }
    pub fn role_allowed(&self, role: &str) -> Vec<String> {
        self.roles
            .get(role)
            .map(|r| r.allowed.clone())
            .unwrap_or_else(|| vec![self.role_default(role)])
    }
    pub fn profile_value(&self, name: &str) -> Result<Value, String> {
        self.profiles
            .get(name)
            .map(|p| serde_json::to_value(p).unwrap())
            .ok_or_else(|| format!("Unknown profile {name}"))
    }
    pub fn allowed_policy(&self) -> Result<Value, String> {
        let mut profiles = serde_json::Map::new();
        let mut roles = serde_json::Map::new();
        for role in ROLES {
            let mut hashes = vec![];
            for name in self.role_allowed(role) {
                let value = self.profile_value(&name)?;
                let hash = digest(&value)?;
                profiles.insert(hash.clone(), json!({"name":name,"profile":value}));
                hashes.push(hash);
            }
            roles.insert(role.into(), json!(hashes));
        }
        Ok(json!({"profiles":profiles,"roles":roles}))
    }
    pub fn resolve(
        &self,
        role: &str,
        override_: &Value,
        requested: Option<&str>,
        policy: &Value,
        revision: u64,
        reason: &str,
    ) -> Result<Value, String> {
        let pin=&policy["request_roles"][role];
        let request_override;
        let override_=if let Some(name)=pin["profile_id"].as_str(){
            if requested.is_some_and(|p|p!=name)||override_["profiles"].as_array().is_some_and(|a|!a.contains(&json!(name))){return Err(format!("This request pins {role} to {name}; coordinator/task routing cannot substitute another profile"))}
            request_override=json!({"mode":"pin","profiles":[name]});&request_override
        }else{override_};
        if override_
            .as_object()
            .is_some_and(|o| o.keys().any(|k| !matches!(k.as_str(), "mode" | "profiles")))
        {
            return Err("Unknown task override fields".into());
        }
        if !ROLES.contains(&role) {
            return Err("Unknown task role".into());
        }
        let candidates = self.role_allowed(role);
        let selected_override = override_["profiles"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if selected_override.len() > 32 {
            return Err("Task allowed set is too large".into());
        }
        for name in &selected_override {
            let profile = self.profile_value(name)?;
            if !policy_allows(policy, role, &digest(&profile)?, &profile) {
                return Err(format!(
                    "Task override includes an unapproved {role} profile: {name}"
                ));
            }
        }
        let (name, source) = match override_["mode"].as_str().unwrap_or("inherit") {
            "pin" if selected_override.len() == 1 => {
                (selected_override[0].clone(), "task override")
            }
            "allow" if !selected_override.is_empty() => {
                if let Some(name) = requested {
                    if !selected_override.iter().any(|p| p == name) {
                        return Err("Coordinator choice is outside the task override".into());
                    }
                    (name.into(), "coordinator within task override")
                } else {
                    let default = self.role_default(role);
                    (
                        if selected_override.contains(&default) {
                            default
                        } else {
                            selected_override[0].clone()
                        },
                        "task allowed set",
                    )
                }
            }
            "inherit" => {
                if let Some(name) = requested {
                    (name.into(), "coordinator choice")
                } else {
                    (self.role_default(role), "role/project default")
                }
            }
            _ => return Err("Invalid task profile override".into()),
        };
        if !matches!(override_["mode"].as_str(), Some("pin" | "allow"))
            && !candidates.contains(&name)
        {
            return Err(format!(
                "Profile {name} is not allowed for role {role} by preferences"
            ));
        }
        let profile = self.profile_value(&name)?;
        let hash = digest(&profile)?;
        if !crate::request_routing::pin_allows(policy,role,&hash,&profile){return Err("Saved request role profile changed; reconcile the choice before execution".into())}
        if !policy_allows(policy, role, &hash, &profile) {
            return Err(format!(
                "Profile {name} for {role} needs explicit provider-policy approval; existing work is retained"
            ));
        }
        Ok(
            json!({"profile_id":name,"profile":profile,"profile_sha256":hash,"preferences_revision":revision,
            "role":role,"source":if pin.is_null(){source}else{"saved human request role"},"reason":reason,"reported_model":null,"reported_effort":null}),
        )
    }
}
pub fn policy_allows(policy: &Value, role: &str, hash: &str, profile: &Value) -> bool {
    if policy["routing"]["roles"][role]
        .as_array()
        .is_some_and(|a| a.contains(&json!(hash)))
        && policy["routing"]["profiles"][hash]["profile"] == *profile
    {
        return true;
    }
    false // Routing authority must identify the selected shared-runtime profile.

}
pub fn find_client(name: &str) -> Option<PathBuf> {
    let mut candidates=std::env::var_os("PATH").map(|paths|std::env::split_paths(&paths)
        .map(|p|p.join(name)).collect::<Vec<_>>()).unwrap_or_default();
    if let Some(home)=std::env::var_os("HOME").map(PathBuf::from){
        for folder in [".local/bin",".npm-global/bin",".npm/bin",".opencode/bin"]{
            candidates.push(home.join(folder).join(name));
        }
    }
    candidates.into_iter().find(|p|p.is_file())
        .map(|p|if p.is_absolute(){p}else{std::env::current_dir().unwrap_or_default().join(p)})
}
pub fn imported(value:&Value)->Result<Value,String>{
    Preferences::parse(value)?;Ok(value.clone())
}
fn metadata(s: &Session, key: &str) -> Result<Option<Value>, String> {
    let raw: Option<String> = s
        .workflow
        .connection()?
        .query_row(
            "SELECT value FROM session_metadata WHERE name=?1",
            [key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    raw.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .transpose()
}
pub fn enabled(s: &Session) -> bool {
    read(&s.workflow.state.join("state-contract.json"))
        .ok()
        .is_some_and(|v| v["state_schema"] == 9 && v["project_key"] == s.key)
}
pub fn schema(s: &Session) -> Result<(), String> {
    s.workflow.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS provider_preference_revisions(revision INTEGER PRIMARY KEY,sha256 TEXT NOT NULL,record_json TEXT NOT NULL);").map_err(|e|e.to_string())?;
    if metadata(s, "provider_preferences")?.is_none() {
        let value = read(&s.project.join(".waywr/coordinator.json"))?;
        Preferences::parse(&value)?;
        let hash = digest(&value)?;
        let p = Preferences::parse(&value)?;
        let profile = p.profile_value(&p.coordinator)?;
        let mut c = s.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO provider_preference_revisions VALUES(1,?1,?2)",
            params![hash, encode(&value)?],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO session_metadata VALUES('provider_preferences',?1)",
            [encode(
                &json!({"revision":1,"sha256":hash,"published_sha256":hash,"value":value}),
            )?],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO session_metadata VALUES('provider_coordinator',?1)",[encode(&json!({"profile_id":p.coordinator,"profile":profile,"profile_sha256":digest(&profile)?}))?]).map_err(|e|e.to_string())?;
        tx.execute(
            "INSERT INTO session_metadata VALUES('provider_context_grants',?1)",
            [encode(&json!([digest(&profile)?]))?],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    if metadata(s,"provider_discovery")?.is_none(){
        let value=read(&s.project.join(".waywr/coordinator.json"))?;
        let p=Preferences::parse(&value)?;
        let discovery=discover_with(Some(&p));
        s.workflow.connection()?.execute("INSERT INTO session_metadata VALUES('provider_discovery',?1)",
            [encode(&discovery)?]).map_err(|e|e.to_string())?;
    }
    Ok(())
}
pub fn preferences(s: &Session) -> Result<(Preferences, u64), String> {
    let head =
        metadata(s, "provider_preferences")?.ok_or("Provider preferences not initialized")?;
    Ok((
        Preferences::parse(&head["value"])?,
        head["revision"]
            .as_u64()
            .ok_or("Preference revision missing")?,
    ))
}
pub fn current(s: &Session) -> Result<Value, String> {
    metadata(s, "provider_coordinator")?.ok_or("Coordinator selection missing".into())
}
pub fn switch(s: &Session) -> Result<Value, String> {
    Ok(metadata(s, "provider_switch")?.unwrap_or(Value::Null))
}
pub fn switch_pending(s: &Session) -> bool {
    switch(s).ok().is_some_and(|v| {
        matches!(
            v["status"].as_str(),
            Some("requested" | "waiting" | "blocked")
        )
    })
}
pub fn effective(s: &Session) -> Result<Value, String> {
    let (prefs, revision) = preferences(s)?;
    let selected = current(s)?;
    let mut value = selected["profile"].clone();
    value["profile"] = json!("development-profiles-v1");
    value["mode"] = json!("native");
    value["concurrency"] = json!(prefs.concurrency);
    value["dependency_network"] = json!(prefs.dependency_network);
    value["main_policy"] = json!(prefs.main_policy);
    value["selection"] = json!({"profile_id":selected["profile_id"],"profile_sha256":selected["profile_sha256"],
        "preferences_revision":revision,"profile":selected["profile"],"role":"coordinator","source":selected["source"].as_str().unwrap_or("project coordinator binding"),"reason":"selected project binding"});
    Ok(with_client(value))
}
/// Reconcile durable file publication first; do not overwrite unexpected external edits.
pub fn reconcile(s: &Session) -> Result<bool, String> {
    if let Some(writer) = &s.workflow.writer {
        return writer.call(|s| reconcile(s));
    }
    let mut head = metadata(s, "provider_preferences")?.ok_or("Preferences missing")?;
    let path = s.project.join(".waywr/coordinator.json");
    let actual = read(&path)?;
    let hash = digest(&actual)?;
    if hash == head["sha256"] {
        if head["publication_pending"] == true {
            head["published_sha256"] = head["sha256"].clone();
            head["publication_pending"] = json!(false);
            s.workflow
                .connection()?
                .execute(
                    "UPDATE session_metadata SET value=?1 WHERE name='provider_preferences'",
                    [encode(&head)?],
                )
                .map_err(|e| e.to_string())?;
        }
        return Ok(false);
    }
    if head["publication_pending"] == true {
        if hash != head["published_sha256"] {
            return Err("Preferences changed outside a pending save; both versions retained for explicit reconciliation".into());
        }
        write(&path, &head["value"], false)?;
        head["published_sha256"] = head["sha256"].clone();
        head["publication_pending"] = json!(false);
        s.workflow
            .connection()?
            .execute(
                "UPDATE session_metadata SET value=?1 WHERE name='provider_preferences'",
                [encode(&head)?],
            )
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }
    // External files are untrusted routing preferences, never a context/policy grant or switch.
    Preferences::parse(&actual)?;
    let revision = head["revision"].as_u64().unwrap() + 1;
    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO provider_preference_revisions VALUES(?1,?2,?3)",
        params![revision as i64, hash, encode(&actual)?],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE session_metadata SET value=?1 WHERE name='provider_preferences'",
        [encode(
            &json!({"revision":revision,"sha256":hash,"published_sha256":hash,"value":actual}),
        )?],
    )
    .map_err(|e| e.to_string())?;
    event(
        &tx,
        "providers.preferences_imported",
        None,
        &json!({"revision":revision,"sha256":hash,"authority":"preferences only; coordinator binding unchanged"}),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}
pub fn save(s: &Session, body: &Value) -> Result<Value, String> {
    if let Some(w) = &s.workflow.writer {
        let b = body.clone();
        return w.call(move |s| save(s, &b));
    }
    crate::conversation_store::valid_command(body, "providers.save").map_err(|e| e.to_string())?;
    let p = &body["payload"];
    let mut value = p["preferences"].clone();
    Preferences::parse(&value)?;
    for (_, profile) in value["profiles"].as_object_mut().unwrap() {
        profile["transport"]["destination"]=crate::model_runtime::destination(profile["provider"].as_str().ok_or("Provider missing")?,&profile["transport"])?;
        if profile["binary_sha256"].is_null() {
            if let Some(binary) = profile["binary"]
                .as_str()
                .map(Path::new)
                .filter(|p| p.is_file())
            {
                profile["binary_sha256"] = json!(crate::execution::hash_file(binary)?);
            }
        }
    }
    let value = &value;
    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    if let Some(v) = prior(&tx, body)? {
        return Ok(v);
    }
    let raw: String = tx
        .query_row(
            "SELECT value FROM session_metadata WHERE name='provider_preferences'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let head: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if p["expected_revision"] != head["revision"] {
        return Err("Preferences changed; reload before saving".into());
    }
    if digest(&read(&s.project.join(".waywr/coordinator.json"))?)? != head["sha256"] {
        return Err("External preferences changed; refresh/import before saving".into());
    }
    let rev = head["revision"].as_u64().unwrap() + 1;
    let hash = digest(value)?;
    tx.execute(
        "INSERT INTO provider_preference_revisions VALUES(?1,?2,?3)",
        params![rev as i64, hash, encode(value)?],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("UPDATE session_metadata SET value=?1 WHERE name='provider_preferences'",[encode(&json!({"revision":rev,"sha256":hash,"published_sha256":head["sha256"],"value":value,"publication_pending":true}))?]).map_err(|e|e.to_string())?;
    event(
        &tx,
        "providers.preferences_saved",
        None,
        &json!({"revision":rev,"sha256":hash,"preferences":value,"subject_id":p["subject_id"],"applies":"unreserved future attempts; active coordinator changes only through Switch"}),
    )?;
    let out = json!({"status":"accepted","preferences_revision":rev});
    commit(&tx, body, &out)?;
    tx.commit().map_err(|e| e.to_string())?;
    reconcile(s)?;
    Ok(out)
}
pub fn snapshot(s: &Session) -> Result<Value, String> {
    let (p, revision) = preferences(s)?;
    Ok(
        json!({"schema_version":1,"preferences":p.value(),"revision":revision,"coordinator":current(s)?,
        "switch":switch(s)?,"context_grants":metadata(s,"provider_context_grants")?,
        "discovery":metadata(s,"provider_discovery")?,"evidence":"local capability metadata is not role/authentication proof"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Preferences { Preferences::parse(&recovery_tests::prefs()).unwrap() }
    #[test]
    fn precedence_and_permission_are_independent() {
        let mut p = config();
        let copy = p.profiles["codex"].clone();
        p.profiles.insert("other".into(), copy);
        p.roles
            .get_mut("implementation")
            .unwrap()
            .allowed
            .push("other".into());
        let policy = json!({"routing":p.allowed_policy().unwrap()});
        let selected = p
            .resolve(
                "implementation",
                &json!({"mode":"pin","profiles":["codex"]}),
                Some("other"),
                &policy,
                3,
                "complexity",
            )
            .unwrap();
        assert_eq!(selected["profile_id"], "codex");
        assert_eq!(selected["source"], "task override");
        let mut narrower = p.clone();
        narrower.roles.get_mut("implementation").unwrap().allowed = vec!["codex".into()];
        assert_eq!(
            narrower
                .resolve(
                    "implementation",
                    &json!({"mode":"pin","profiles":["other"]}),
                    Some("codex"),
                    &policy,
                    3,
                    "explicit task override"
                )
                .unwrap()["profile_id"],
            "other"
        );
        assert!(p
            .resolve(
                "implementation",
                &json!({"mode":"allow","profiles":["codex"]}),
                Some("other"),
                &policy,
                3,
                ""
            )
            .is_err());
        let mut changed = p.value();
        changed["profiles"]["other"]["effort"] = json!("high");
        let changed = Preferences::parse(&changed).unwrap();
        assert!(changed
            .resolve(
                "implementation",
                &Value::Null,
                Some("other"),
                &policy,
                4,
                ""
            )
            .unwrap_err()
            .contains("approval"));
    }
    #[test]
    fn defaults_preserve_codex_and_unknown_effort_never_clamps() {
        let p = config();
        assert_eq!(p.coordinator, "codex");
        assert!(p.fallback.values().all(Vec::is_empty));
        let mut value = p.value();
        value["profiles"]["codex"]["effort"] = json!("none");
        assert!(Preferences::parse(&value).is_err());
        value["profiles"]["codex"]["model"] = json!("custom");
        value["profiles"]["codex"]["effort"] = json!("provider_default");
        assert!(Preferences::parse(&value).is_ok());
        value["profiles"]["codex"]["api_key"] = json!("do not persist");
        assert!(Preferences::parse(&value).is_err());
    }
}

fn put(tx: &rusqlite::Transaction<'_>, key: &str, value: &Value) -> Result<(), String> {
    tx.execute("INSERT INTO session_metadata(name,value) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET value=excluded.value",params![key,encode(value)?]).map_err(|e|e.to_string())?;
    Ok(())
}
/// Explicit commands use the same project writer and durable command IDs as scope approval.
pub fn command(s: &Session, body: &Value) -> Result<Value, String> {
    if !enabled(s) {
        return Err(
            "Provider selection requires a normal project release upgrade to state9".into(),
        );
    }
    if body["action"] == "providers.save" {
        return save(s, body);
    }
    if let Some(w) = &s.workflow.writer {
        let b = body.clone();
        return w.call(move |s| command(s, &b));
    }
    crate::conversation_store::valid_command(body, body["action"].as_str().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    let (prefs, revision) = preferences(s)?;
    let payload = &body["payload"];
    if body["action"] == "providers.refresh" {
        let discovery = discover_with(Some(&prefs));
        let mut c = s.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        if let Some(old) = prior(&tx, body)? {
            return Ok(old);
        }
        put(&tx, "provider_discovery", &discovery)?;
        let out = json!({"status":"accepted","discovery":discovery});
        commit(&tx, body, &out)?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(out);
    }

    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    if let Some(old) = prior(&tx, body)? {
        return Ok(old);
    }
    if payload["expected_revision"] != revision {
        return Err("Provider preferences changed; reload settings".into());
    }
    let out = match body["action"].as_str().unwrap_or("") {
        "providers.switch" => {
            if switch_pending(s) {
                return Err("A coordinator switch is pending; inspect or cancel it first".into());
            }
            let name = payload["profile_id"]
                .as_str()
                .ok_or("Choose a coordinator profile")?;
            if !prefs.role_allowed("coordinator").contains(&name.to_owned()) {
                return Err("Profile is outside the configured coordinator set".into());
            }
            let profile = prefs.profile_value(name)?;
            let hash = digest(&profile)?;
            let already_approved = s.development_records("workflow")?.iter().any(|work| {
                normalized_policy(s, &work["policy"])
                    .is_ok_and(|p| policy_allows(&p, "coordinator", &hash, &profile))
            });
            let mut grants = metadata(s, "provider_context_grants")?.unwrap_or(json!([]));
            if !grants.as_array().is_some_and(|a| a.contains(&json!(hash))) {
                if payload["grant_context"] != true && !already_approved {
                    return Err("New coordinator destination: explicitly allow this exact profile to receive selected project context".into());
                }
                grants
                    .as_array_mut()
                    .ok_or("Invalid context grants")?
                    .push(json!(hash));
                put(&tx, "provider_context_grants", &grants)?;
                event(
                    &tx,
                    "providers.context_granted",
                    None,
                    &json!({"profile_id":name,"profile_sha256":hash,"profile":profile,"command_id":body["command_id"],"subject_id":payload["subject_id"]}),
                )?;
            }
            for work in s
                .development_records("workflow")?
                .iter()
                .filter(|w| !matches!(w["status"].as_str(), Some("delivered" | "cancelled")))
            {
                if !normalized_policy(s, &work["policy"])
                    .is_ok_and(|p| policy_allows(&p, "coordinator", &hash, &profile))
                {
                    return Err("Active request policy excludes this coordinator; approve a narrow provider amendment first".into());
                }
            }
            let generation: i64 = tx
                .query_row(
                    "SELECT generation FROM conversation_bindings WHERE singleton=1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if payload["expected_generation"] != generation {
                return Err("Coordinator binding changed; refresh".into());
            }
            let mode = payload["mode"].as_str().unwrap_or("after_current_turn");
            if !matches!(mode, "after_current_turn" | "interrupt") {
                return Err("Unknown switch mode".into());
            }
            let value = json!({"switch_id":body["command_id"],"status":"requested","mode":mode,"generation":generation,
                "target":{"profile_id":name,"profile":profile,"profile_sha256":hash,"source":"explicit human coordinator switch"},"preferences_revision":revision,"subject_id":payload["subject_id"],"requested_at_ms":(crate::execution::now()*1000.0) as u64});
            if mode == "interrupt" {
                let turn: Option<String> = tx
                    .query_row(
                        "SELECT turn_id FROM conversation_turns WHERE status='reserved'",
                        [],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                if let Some(turn) = turn {
                    tx.execute("UPDATE dialogue_inputs SET interrupt_command_id=?1 WHERE turn_id=?2 AND interrupt_command_id IS NULL",params![body["command_id"].as_str(),turn]).map_err(|e|e.to_string())?;
                    put(
                        &tx,
                        "claude_chat_cancel_v1",
                        &json!({"turn_id":turn,"command_id":body["command_id"]}),
                    )?;
                }
            }
            put(&tx, "provider_switch", &value)?;
            event(&tx, "providers.switch_requested", None, &value)?;
            json!({"status":"accepted","switch":value})
        }
        "providers.cancel_switch" => {
            let mut value = switch(s)?;
            if !switch_pending(s) {
                return Err("No pending switch".into());
            }
            if value["mode"] == "interrupt" {
                let unsettled: i64 = tx.query_row("SELECT count(*) FROM conversation_turns WHERE status IN ('reserved','uncertain')", [], |r|r.get(0)).map_err(|e|e.to_string())?;
                if unsettled > 0
                    || s.development_records("workflow")?.iter().any(|w| {
                        w["pending"]["kind"] == "coordinate" || w["switch_uncertain"] == true
                    })
                {
                    return Err("Interruption is already fenced; positively settle it before cancelling the replacement".into());
                }
            }
            value["status"] = json!("cancelled");
            put(&tx, "provider_switch", &value)?;
            event(&tx, "providers.switch_cancelled", None, &value)?;
            json!({"status":"accepted"})
        }
        "providers.approve_policy" => {
            let id = payload["work_id"].as_str().ok_or("Missing request")?;
            let raw: String = tx
                .query_row(
                    "SELECT record_json FROM development_records WHERE kind='workflow' AND id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let mut work: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            if payload["policy_sha256"] != digest(&work["policy"])? {
                return Err("Request policy changed; review again".into());
            }
            if matches!(work["status"].as_str(), Some("delivered" | "cancelled")) {
                return Err("Closed request policies are retained; configure future defaults and approve the next request through /plan".into());
            }
            let narrow=payload["grant_role"].as_str();
            let routing = if let Some(role)=narrow{narrow_routing(&prefs,role,payload["grant_profile"].as_str().ok_or("Missing granted profile")?)?}else{prefs.allowed_policy()?};
            if payload["routing_sha256"] != digest(&routing)? {
                return Err("Proposed provider policy changed; review again".into());
            }
            // Additive: old frozen profiles/roles remain valid for their existing attempts.
            if work["policy"]["routing"].is_null() {
                work["policy"]["routing"] =
                    normalized_policy(s, &work["policy"])?["routing"].clone();
            }
            for (hash, profile) in routing["profiles"].as_object().unwrap() {
                work["policy"]["routing"]["profiles"][hash] = profile.clone();
            }
            for role in ROLES {
                let mut hashes = work["policy"]["routing"]["roles"][role]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                for hash in routing["roles"][role].as_array().unwrap() {
                    if !hashes.contains(hash) {
                        hashes.push(hash.clone());
                    }
                }
                work["policy"]["routing"]["roles"][role] = json!(hashes);
            }
            if narrow.is_none(){work["policy"]["fallback"] = json!(prefs.fallback);}
            work["policy"]["schema_version"] = json!(2);
            work["revision"] = json!(work["revision"].as_u64().unwrap_or(0) + 1);
            work["last_context_sha256"] = Value::Null;
            if work["provider_blocked"]["kind"] == "policy" && work["question"].is_null() {
                work["status"] = json!("running");
                work["attention"] = Value::Null;
                work["provider_blocked"] = Value::Null;
            }
            tx.execute(
                "UPDATE development_records SET record_json=?1 WHERE kind='workflow' AND id=?2",
                params![encode(&work)?, id],
            )
            .map_err(|e| e.to_string())?;
            event(
                &tx,
                "providers.policy_amended",
                Some(id),
                &json!({"command_id":body["command_id"],"policy":work["policy"],"preserved_scope":work["scope_base"]}),
            )?;
            json!({"status":"accepted","policy_sha256":digest(&work["policy"])?})
        }
        "providers.task_override" => {
            let id = payload["unit_id"].as_str().ok_or("Missing task")?;
            let raw: String = tx
                .query_row(
                    "SELECT record_json FROM development_records WHERE kind='unit' AND id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let mut unit: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            if !matches!(
                unit["status"].as_str(),
                Some("planned" | "ready" | "capacity-waiting")
            ) {
                return Err(
                    "Task already reserved; override affects unreserved attempts only".into(),
                );
            }
            if payload["attempt_revision"] != unit["attempt_revision"] {
                return Err("Task attempt changed; refresh".into());
            }
            let override_ = &payload["override"];
            let role = unit["role"].as_str().unwrap_or("implementation");
            let work = s
                .development_record(
                    "workflow",
                    unit["work_id"].as_str().ok_or("Task lacks request")?,
                )?
                .ok_or("Request missing")?;
            if matches!(
                work["status"].as_str(),
                Some("delivered" | "cancelled" | "cancelling")
            ) {
                return Err("Task belongs to a closed/cancelling request; history retained".into());
            }
            prefs.resolve(
                role,
                override_,
                None,
                &normalized_policy(s, &work["policy"])?,
                revision,
                "human task override",
            )?;
            unit["profile_override"] = override_.clone();
            if unit["status"] == "capacity-waiting" {
                unit["status"] = json!("ready");
                unit["retry_at"] = json!(0);
            }

            unit["fallback_profile"] = Value::Null;
            unit["attempt_revision"] = json!(unit["attempt_revision"].as_u64().unwrap_or(1) + 1);
            tx.execute(
                "UPDATE development_records SET record_json=?1 WHERE kind='unit' AND id=?2",
                params![encode(&unit)?, id],
            )
            .map_err(|e| e.to_string())?;
            event(
                &tx,
                "providers.task_override",
                unit["work_id"].as_str(),
                &unit,
            )?;
            json!({"status":"accepted"})
        }
        _ => return Err("Unknown provider command".into()),
    };
    commit(&tx, body, &out)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(out)
}
/// Invoked by the existing scheduler after receipt adoption. Workers and Git lanes keep running.
pub fn advance_switch(s: &Session) -> Result<(), String> {
    if !enabled(s) || !switch_pending(s) {
        return Ok(());
    }
    if let Some(w) = &s.workflow.writer {
        return w.call(advance_switch);
    }
    let mut value = switch(s)?;
    let reserved = s.chat_reserved()?.is_some();
    let works = s.development_records("workflow")?;
    let pending = works.iter().any(|w| w["pending"]["kind"] == "coordinate");
    // A stopped uncertain call is not settlement. No replacement while any such receipt is unresolved.
    let uncertain: i64 = s
        .workflow
        .connection()?
        .query_row(
            "SELECT count(*) FROM conversation_turns WHERE status='uncertain'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if uncertain > 0 || works.iter().any(|w| w["switch_uncertain"] == true) {
        if value["status"] != "blocked" {
            value["status"] = json!("blocked");
            value["reason"] = json!(
                "Uncertain coordinator execution needs positive reconciliation; no replacement"
            );
            let mut c = s.workflow.connection()?;
            let tx = c.transaction().map_err(|e| e.to_string())?;
            put(&tx, "provider_switch", &value)?;
            event(&tx, "providers.switch_blocked", None, &value)?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    if reserved || pending {
        if value["status"] == "requested" {
            value["status"] = json!("waiting");
            let mut c = s.workflow.connection()?;
            let tx = c.transaction().map_err(|e| e.to_string())?;
            put(&tx, "provider_switch", &value)?;
            event(&tx, "providers.switch_waiting", None, &value)?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let generation: i64 = tx
        .query_row(
            "SELECT generation FROM conversation_bindings WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if value["generation"] != generation {
        value["status"] = json!("blocked");
        value["reason"] =
            json!("Binding generation changed while switch waited; no replacement applied");
        put(&tx, "provider_switch", &value)?;
        event(&tx, "providers.switch_blocked", None, &value)?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let target = value["target"].clone();
    let excluded = works
        .iter()
        .filter(|w| !matches!(w["status"].as_str(), Some("delivered" | "cancelled")))
        .any(|w| {
            normalized_policy(s, &w["policy"]).map_or(true, |policy| {
                !policy_allows(
                    &policy,
                    "coordinator",
                    target["profile_sha256"].as_str().unwrap_or(""),
                    &target["profile"],
                )
            })
        });
    if excluded {
        value["status"] = json!("blocked");
        value["reason"] = json!("Active request policy changed while switch waited; approve the selected destination before replacement");
        put(&tx, "provider_switch", &value)?;
        event(&tx, "providers.switch_blocked", None, &value)?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let p = &target["profile"];
    tx.execute("UPDATE conversation_bindings SET generation=generation+1,provider=?1,model=?2,effort=?3 WHERE singleton=1 AND generation=?4",params![p["provider"].as_str(),p["model"].as_str(),p["effort"].as_str(),generation]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE conversation_turns SET binding_generation=?1 WHERE status='queued' AND project_key=?2",params![generation+1,s.key]).map_err(|e|e.to_string())?;
    let changed = current(s)?["profile_sha256"] != target["profile_sha256"];
    put(&tx, "provider_coordinator", &target)?;
    if changed {
        tx.execute("UPDATE development_records SET record_json=json_set(record_json,'$.retry_at',0) WHERE kind='chat-capacity' AND id IN (SELECT turn_id FROM conversation_turns WHERE status='queued')",[]).map_err(|e|e.to_string())?;
    }

    // Replacement can recover known configuration/policy blocks, never substantive questions.
    for mut work in works {
        if changed
            && work["capacity_refusal"].is_object()
            && work["pending"].is_null()
            && !matches!(work["status"].as_str(), Some("delivered" | "cancelled"))
        {
            work["capacity_retry_at"] = json!(0);
            work["last_context_sha256"] = Value::Null;
            tx.execute(
                "UPDATE development_records SET record_json=?1 WHERE kind='workflow' AND id=?2",
                params![encode(&work)?, work["work_id"].as_str()],
            )
            .map_err(|e| e.to_string())?;
        }
        if matches!(
            work["provider_blocked"]["kind"].as_str(),
            Some("configuration" | "policy")
        ) && work["question"].is_null()
            && work["pending"].is_null()
        {
            work["status"] = json!("running");
            work["attention"] = Value::Null;
            work["provider_blocked"] = Value::Null;
            work["last_context_sha256"] = Value::Null;
            tx.execute(
                "UPDATE development_records SET record_json=?1 WHERE kind='workflow' AND id=?2",
                params![encode(&work)?, work["work_id"].as_str()],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    value["status"] = json!("applied");
    value["applied_generation"] = json!(generation + 1);
    value["applied_at_ms"] = json!((crate::execution::now() * 1000.0) as u64);
    put(&tx, "provider_switch", &value)?;
    event(&tx, "providers.switch_applied", None, &value)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
/// Frozen routing object embedded in the operation packet at reservation, never re-read by a runner.
pub fn execution(
    s: &Session,
    role: &str,
    override_: &Value,
    requested: Option<&str>,
    policy: &Value,
    reason: &str,
) -> Result<(Value, Value), String> {
    let (prefs, revision) = preferences(s)?;
    let policy = normalized_policy(s, policy)?;
    let selected = if role == "coordinator" {
        let bound = current(s)?;
        let profile = &bound["profile"];
        let hash = digest(profile)?;
        if !policy_allows(&policy, role, &hash, profile)||!crate::request_routing::pin_allows(&policy,role,&hash,profile) {
            return Err(
                "Coordinator binding is outside this request's approved provider policy".into(),
            );
        }
        json!({"profile_id":bound["profile_id"],"profile":profile,"profile_sha256":hash,"preferences_revision":revision,"role":role,"source":bound["source"].as_str().unwrap_or("coordinator binding"),"reason":reason})
    } else {
        prefs.resolve(role, override_, requested, &policy, revision, reason)?
    };
    let mut execution = selected["profile"].clone();
    execution["profile"] = json!("development-profiles-v1");
    execution["mode"] = json!("native");
    execution["dependency_network"] = policy["dependency_network"].clone();
    execution["selection"] = selected.clone();
    Ok((with_client(execution), selected))
}

/// Public catalogs only. No login, inference or model-server start.
pub fn discover()->Value{discover_with(None)}
fn discover_with(prefs:Option<&Preferences>)->Value{
    let selected=prefs.and_then(|p|p.profiles.get(&p.coordinator));
    let binary=selected.map(|p|PathBuf::from(&p.binary)).map(Ok).unwrap_or_else(crate::model_runtime::node);
    let transport=selected.and_then(|p|p.transport.clone()).map(Ok).unwrap_or_else(crate::model_runtime::default_transport);
    match binary.and_then(|b|transport.and_then(|t|crate::model_runtime::describe(&b,&t))){
        Ok(mut v)=>{v["inference"]=json!(false);v},Err(reason)=>json!({"models":[],"providers":[],"reason":reason,"inference":false})
    }
}
/// Quiescent release maintenance only. Journal original bytes; never rewrite approvals or old packets.
pub fn migrate_project(project: &Path, state: &Path, key: &str) -> Result<bool, String> {
    let path = project.join(".waywr/provider-migration.json");
    let profile = project.join(".waywr/coordinator.json");
    let config = project.join(".waywr/project.toml");
    let contract = state.join("state-contract.json");
    let mut journal = if path.exists() {
        read(&path)?
    } else {
        let old = read(&contract)?;
        if old["state_schema"] != 8 {
            return Ok(false);
        }
        if old["project_key"] != key {
            return Err("Migration project identity differs".into());
        }
        let old_profile = read(&profile)?;
        let text = std::fs::read_to_string(&config).map_err(|e| e.to_string())?;
        let new_text = text.replace(
            "[conversation]\nschema_version = 5",
            "[conversation]\nschema_version = 6",
        );
        if new_text == text {
            return Err(
                "Unsupported project config formatting for migration; original preserved".into(),
            );
        }
        let new_profile = imported(&old_profile)?;
        let mut new_contract = old.clone();
        new_contract["state_schema"] = json!(9);
        let journal = json!({"schema_version":1,"project_key":key,"status":"prepared","original":{"profile":old_profile,"config":text,"contract":old},"next":{"profile":new_profile,"config":new_text,"contract":new_contract}});
        write(&path, &journal, true)?;
        journal
    };
    if journal["schema_version"] != 1
        || !matches!(journal["status"].as_str(), Some("prepared" | "complete"))
        || journal["original"]["contract"]["state_schema"] != 8
        || journal["next"]["contract"]["state_schema"] != 9
    {
        return Err("Unknown provider migration journal; preserve and reconcile".into());
    }
    if journal["project_key"] != key {
        return Err("Provider migration belongs to another project".into());
    }
    if journal["status"] == "complete" {
        return Ok(false);
    }
    let old = &journal["original"];
    let next = &journal["next"];
    for (target, field) in [(&profile, "profile"), (&contract, "contract")] {
        let actual = read(target)?;
        if actual != old[field] && actual != next[field] {
            return Err(format!(
                "Migration conflict at {}; both versions preserved",
                target.display()
            ));
        }
    }
    let actual = std::fs::read_to_string(&config).map_err(|e| e.to_string())?;
    if actual != old["config"].as_str().unwrap_or("")
        && actual != next["config"].as_str().unwrap_or("")
    {
        return Err("Project config changed during migration; preserved".into());
    }
    // The owner is stopped by release maintenance; retain identity and all application rows.
    // A prepared journal can recover either side of the DB/contract publication boundary.
    let database = state.join("state.sqlite");
    if database.exists() {
        let mut connection = rusqlite::Connection::open(&database).map_err(|e| e.to_string())?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let version: i64 = tx
            .query_row("SELECT max(version) FROM ww_schema_migrations", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        if !matches!(version, 8 | 9) {
            return Err("Provider migration database version changed; preserved".into());
        }
        tx.execute(
            "UPDATE ww_schema_migrations SET version=9 WHERE version=8",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    write(&profile, &next["profile"], false)?;
    crate::execution::write_bytes(
        &config,
        next["config"]
            .as_str()
            .ok_or("Missing migrated config")?
            .as_bytes(),
        false,
    )?;
    write(&contract, &next["contract"], false)?;
    journal["status"] = json!("complete");
    write(&path, &journal, false)?;
    Ok(true)
}

pub fn fallback_choice(
    s: &Session,
    role: &str,
    previous: &Value,
    override_: &Value,
    policy: &Value,
) -> Result<Option<String>, String> {
    let policy = normalized_policy(s, policy)?;
    let (prefs, revision) = preferences(s)?;
    let Some(configured) = prefs.fallback.get(role) else {
        return Ok(None);
    };
    let approved = policy["fallback"][role].as_array();
    let continuing = previous["source"] == "settled capacity fallback";
    let previous = previous["profile_id"].as_str().unwrap_or("");
    let offset = if continuing {
        configured
            .iter()
            .position(|n| n == previous)
            .map_or(0, |i| i + 1)
    } else {
        0
    };
    for candidate in configured.iter().skip(offset) {
        if candidate == previous || !approved.is_some_and(|a| a.contains(&json!(candidate))) {
            continue;
        }
        if let Ok(selected) = prefs.resolve(
            role,
            override_,
            Some(candidate),
            &policy,
            revision,
            "explicit settled capacity fallback",
        ) {
            // A task pin may intentionally resolve back to its pin; it is not an alternative.
            if selected["profile_id"] == *candidate {
                return Ok(Some(candidate.clone()));
            }
        }
    }
    Ok(None)
}
/// Only callers holding a positively settled capacity receipt may request this automatic handover.
pub fn capacity_switch(
    s: &Session,
    policy: &Value,
    proof: &Value,
    source: &str,
) -> Result<bool, String> {
    if !enabled(s) || switch_pending(s) {
        return Ok(false);
    }
    if proof["known_completion"] != true
        || proof["settled"] != true
        || proof["category"] != "capacity"
    {
        return Err("Fallback requires positive settled provider capacity evidence".into());
    }
    if let Some(w) = &s.workflow.writer {
        let (p, r, id) = (policy.clone(), proof.clone(), source.to_owned());
        return w.call(move |s| capacity_switch(s, &p, &r, &id));
    }
    let bound = current(s)?;
    let Some(name) = fallback_choice(s, "coordinator", &bound, &Value::Null, policy)? else {
        return Ok(false);
    };
    let (prefs, revision) = preferences(s)?;
    let profile = prefs.profile_value(&name)?;
    let mut c = s.workflow.connection()?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let generation: i64 = tx
        .query_row(
            "SELECT generation FROM conversation_bindings WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let value = json!({"switch_id":format!("capacity-{source}"),"status":"requested","mode":"after_current_turn","generation":generation,
        "target":{"profile_id":name,"profile_sha256":digest(&profile)?,"profile":profile,"source":"settled capacity fallback"},"preferences_revision":revision,
        "cause":"explicit capacity fallback inside approved alternatives","source_operation":source,"settlement":proof});
    put(&tx, "provider_switch", &value)?;
    event(&tx, "providers.capacity_switch_requested", None, &value)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}
/// Requeue a positively settled discussion attempt, retaining immutable packet/receipt in its call revision.
pub fn requeue_capacity(s: &Session, turn: &str, proof: &Value) -> Result<(), String> {
    if let Some(w) = &s.workflow.writer {
        let (t, p) = (turn.to_owned(), proof.clone());
        return w.call(move |s| requeue_capacity(s, &t, &p));
    }
    if proof["category"] != "capacity"
        || proof["known_completion"] != true
        || proof["settled"] != true
    {
        return Err("Cannot requeue uncertain conversation execution".into());
    }
    let mut c = s.workflow.connection()?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE conversation_turns SET status='queued',packet_sha256=NULL,context_sha256=NULL,budget_bucket=NULL WHERE turn_id=?1 AND status='reserved'",[turn]).map_err(|e|e.to_string())?;
    event(
        &tx,
        "providers.capacity_requeued",
        None,
        &json!({"turn_id":turn,"proof":proof,"message_preserved":true}),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod recovery_tests {
    use super::*;
    pub(crate) fn prefs() -> Value {
        json!({"schema_version":3,"profile":"development-profiles-v1","profiles":{
        "codex":{"provider":"codex","binary":"/bin/true","options":{},"transport":{"entry":"/bin/true","destination":{"provider":"codex","kind":"runtime_builtin"}},"model":"gpt-6.1-sol","effort":"medium","binary_sha256":crate::execution::hash_file(Path::new("/bin/true")).unwrap(),"context_bytes":98304},
        "claude":{"provider":"claude","binary":"/missing/claude","options":{},"transport":{"entry":"/bin/true","destination":{"provider":"claude","kind":"runtime_builtin"}},"model":"claude-sonnet-5-5","effort":"medium","binary_sha256":null,"context_bytes":98304}},
        "coordinator":"codex","worker_default":"codex","roles":{"coordinator":{"default":"codex","allowed":["codex"]},"implementation":{"default":"codex","allowed":["codex"]},"review":{"default":"codex","allowed":["codex"]}},"fallback":{"coordinator":[],"implementation":[],"review":[]},"concurrency":2,"dependency_network":true,"main_policy":"checked-fast-forward","quality_preference":""})
    }
    pub(crate) fn fixture() -> (tempfile::TempDir, Session) {
        let root = tempfile::tempdir().unwrap();
        let project = root.path();
        let state = project.join(".waywr/state");
        std::fs::create_dir_all(&state).unwrap();
        write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":9,"execution_host":"systemd-user-v1","project_key":"providers"}),true).unwrap();
        write(&project.join(".waywr/coordinator.json"), &prefs(), true).unwrap();
        let s = Session::open(project, &state, "providers", "fixture").unwrap();
        (root, s)
    }
    #[test]
    fn provider_save_replay_external_edit_and_publication_recovery() {
        let (_root, s) = fixture();
        let bound = current(&s).unwrap();
        let mut p = prefs();
        p["quality_preference"] = json!("Prefer independent review for risky changes");
        let command = json!({"command_id":"save-1","action":"providers.save","payload":{"expected_revision":1,"preferences":p}});
        let accepted = save(&s, &command).unwrap();
        assert_eq!(accepted, save(&s, &command).unwrap());
        assert_eq!(preferences(&s).unwrap().1, 2);
        assert_eq!(current(&s).unwrap(), bound);
        // Model/file edits import as preferences, never as coordinator/context authority.
        p["coordinator"] = json!("claude");
        p["roles"]["coordinator"]["allowed"] = json!(["codex", "claude"]);
        write(&s.project.join(".waywr/coordinator.json"), &p, false).unwrap();
        assert!(reconcile(&s).unwrap());
        assert_eq!(preferences(&s).unwrap().1, 3);
        assert_eq!(current(&s).unwrap(), bound);
        // Simulate a committed DB save before the atomic file publication.
        let old = metadata(&s, "provider_preferences").unwrap().unwrap();
        let mut next = p.clone();
        next["quality_preference"] = json!("Changed after crash");
        let head = json!({"revision":4,"sha256":digest(&next).unwrap(),"published_sha256":old["sha256"],"value":next,"publication_pending":true});
        s.workflow
            .connection()
            .unwrap()
            .execute(
                "UPDATE session_metadata SET value=?1 WHERE name='provider_preferences'",
                [encode(&head).unwrap()],
            )
            .unwrap();
        assert!(reconcile(&s).unwrap());
        assert_eq!(
            read(&s.project.join(".waywr/coordinator.json")).unwrap(),
            next
        );
        assert!(!reconcile(&s).unwrap());
        // A conflicting file is retained, not overwritten by recovery.
        let mut conflict = head;
        conflict["publication_pending"] = json!(true);
        conflict["sha256"] = json!("different");
        s.workflow
            .connection()
            .unwrap()
            .execute(
                "UPDATE session_metadata SET value=?1 WHERE name='provider_preferences'",
                [encode(&conflict).unwrap()],
            )
            .unwrap();
        assert!(reconcile(&s).is_err());
        assert_eq!(
            read(&s.project.join(".waywr/coordinator.json")).unwrap(),
            next
        );
    }
    fn allow_claude(s: &Session) {
        let mut p = prefs();
        p["roles"]["coordinator"]["allowed"] = json!(["codex", "claude"]);
        save(s,&json!({"command_id":"allow","action":"providers.save","payload":{"expected_revision":1,"preferences":p}})).unwrap();
    }
    fn switch_command(mode: &str) -> Value {
        json!({"command_id":"switch","action":"providers.switch","payload":{"expected_revision":2,"expected_generation":1,"profile_id":"claude","mode":mode,"grant_context":true}})
    }
    #[test]
    fn provider_switch_waits_restarts_and_applies_once_preserving_workers_and_questions() {
        let (_root, s) = fixture();
        allow_claude(&s);
        let sent=s.conversation_submit(&json!({"command_id":"human-fixture","action":"submit_message","payload":{"text":"Fixture only: do not run an agent","subject_id":"discussion"}})).unwrap();
        let turn = sent["turn_id"].as_str().unwrap();
        s.workflow.connection().unwrap().execute("UPDATE conversation_turns SET status='reserved',packet_sha256='fixture-sha' WHERE turn_id=?1",[turn]).unwrap();
        let worker = json!({"unit_id":"worker-fixture","status":"running","work_id":"fixture","attempt_revision":1,"selection":{"profile_id":"codex"}});
        s.development_save("unit", "worker-fixture", &worker, "fixture")
            .unwrap();
        s.workflow.connection().unwrap().execute("INSERT INTO conversation_questions VALUES('q',1,'providers',?1,'Retained question','[]','open',NULL,NULL)",[turn]).unwrap();
        let command = switch_command("after_current_turn");
        let out = command.clone();
        let accepted = super::command(&s, &command).unwrap();
        assert_eq!(accepted, super::command(&s, &out).unwrap());
        advance_switch(&s).unwrap();
        assert_eq!(switch(&s).unwrap()["status"], "waiting");
        assert!(s.dialogue_next().unwrap().is_none());
        assert_eq!(current(&s).unwrap()["profile_id"], "codex");
        // Fixture simulates adoption/positive settlement; no native process is ever created.
        s.workflow
            .connection()
            .unwrap()
            .execute(
                "UPDATE conversation_turns SET status='completed' WHERE turn_id=?1",
                [turn],
            )
            .unwrap();
        let queued=s.conversation_submit(&json!({"command_id":"fixture-capacity-queued","action":"submit_message","payload":{"text":"Retained unreserved fixture input","subject_id":"discussion"}})).unwrap();
        s.development_save("chat-capacity",queued["turn_id"].as_str().unwrap(),&json!({"turn_id":queued["turn_id"],"next_attempt":2,"retry_at":crate::execution::now()+3600.0,"status":"capacity-waiting"}),"fixture").unwrap();
        let reopened = Session::open(&s.project, &s.workflow.state, &s.key, "fixture").unwrap();
        advance_switch(&reopened).unwrap();
        advance_switch(&reopened).unwrap();
        assert_eq!(current(&reopened).unwrap()["profile_id"], "claude");
        assert_eq!(
            reopened
                .development_record("unit", "worker-fixture")
                .unwrap()
                .unwrap(),
            worker
        );
        let question: String = reopened
            .workflow
            .connection()
            .unwrap()
            .query_row(
                "SELECT status FROM conversation_questions WHERE question_id='q'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(question, "open");
        assert_eq!(switch(&reopened).unwrap()["applied_generation"], 2);
        let count: i64 = s
            .workflow
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM session_events WHERE type='providers.switch_applied'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let messages: i64 = s
            .workflow
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM conversation_messages", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(messages, 2);
        assert!(s
            .development_chat_attempt(queued["turn_id"].as_str().unwrap())
            .unwrap()
            .is_some());
    }
    #[test]
    fn provider_switch_cancel_and_uncertain_execution_do_not_replace() {
        let (_root, s) = fixture();
        allow_claude(&s);
        super::command(&s, &switch_command("after_current_turn")).unwrap();
        super::command(&s,&json!({"command_id":"cancel-switch","action":"providers.cancel_switch","payload":{"expected_revision":2}})).unwrap();
        advance_switch(&s).unwrap();
        assert_eq!(current(&s).unwrap()["profile_id"], "codex");
        let mut again = switch_command("interrupt");
        again["command_id"] = json!("interrupt-switch");
        super::command(&s, &again).unwrap();
        s.workflow.connection().unwrap().execute("INSERT INTO conversation_messages VALUES('m',1,'providers',100,'user','retained','sha',NULL,'t')",[]).unwrap();
        s.workflow.connection().unwrap().execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status) VALUES('t',1,'providers','m',1,'uncertain')",[]).unwrap();
        advance_switch(&s).unwrap();
        assert_eq!(switch(&s).unwrap()["status"], "blocked");
        assert_eq!(current(&s).unwrap()["profile_id"], "codex");
        assert!(switch_pending(&s));
    }
    #[test]
    fn provider_fallback_requires_approved_order_and_respects_task_pins() {
        let (_root, s) = fixture();
        let mut p = prefs();
        p["roles"]["implementation"]["allowed"] = json!(["codex", "claude"]);
        p["fallback"]["implementation"] = json!(["claude"]);
        save(&s,&json!({"command_id":"fallback-save","action":"providers.save","payload":{"expected_revision":1,"preferences":p}})).unwrap();
        let prefs = preferences(&s).unwrap().0;
        let policy = json!({"routing":prefs.allowed_policy().unwrap(),"fallback":prefs.fallback});
        let old = json!({"profile_id":"codex"});
        assert_eq!(
            fallback_choice(&s, "implementation", &old, &Value::Null, &policy).unwrap(),
            Some("claude".into())
        );
        assert!(fallback_choice(
            &s,
            "implementation",
            &old,
            &json!({"mode":"pin","profiles":["codex"]}),
            &policy
        )
        .unwrap()
        .is_none());
        assert!(
            fallback_choice(&s, "implementation", &old, &Value::Null, &json!({}))
                .unwrap()
                .is_none()
        );
        assert!(capacity_switch(
            &s,
            &policy,
            &json!({"settled":false,"known_completion":true,"category":"capacity"}),
            "uncertain"
        )
        .is_err());
    }
    #[test]
    fn provider_discussion_uses_frozen_state9_packet_and_version3_actions() {
        let (_root, s) = fixture();
        let sent=s.conversation_submit(&json!({"command_id":"fixture-input","action":"submit_message","payload":{"text":"Fixture discussion only","subject_id":"discussion"}})).unwrap();
        let next = s.dialogue_next().unwrap().unwrap();
        assert_eq!(next["turn_id"], sent["turn_id"]);
        let profile = effective(&s).unwrap();
        let packet = s
            .scope_packet(&next, &profile, &json!(digest(&profile).unwrap()))
            .unwrap();
        assert_eq!(packet["provider"], "codex");
        assert_eq!(packet["development"], true);
        let mut changed = preferences(&s).unwrap().0.value();
        changed["quality_preference"] = json!("Future routing hint");
        save(&s,&json!({"command_id":"fixture-pre-reservation-edit","action":"providers.save","payload":{"expected_revision":1,"preferences":changed}})).unwrap();
        assert_eq!(
            s.conversation_reserve(sent["turn_id"].as_str().unwrap(), &packet, "conversation")
                .unwrap()["code"],
            "busy"
        );
        let profile = effective(&s).unwrap();
        let packet = s
            .scope_packet(&next, &profile, &json!(digest(&profile).unwrap()))
            .unwrap();
        let hash = digest(&packet).unwrap();
        write(
            &s.workflow
                .state
                .join("session/conversation")
                .join(sent["turn_id"].as_str().unwrap())
                .join("packets")
                .join(format!("{hash}.json")),
            &packet,
            true,
        )
        .unwrap();
        assert_eq!(
            s.conversation_reserve(sent["turn_id"].as_str().unwrap(), &packet, "conversation")
                .unwrap()["status"],
            "accepted"
        );
        let result = json!({"schema_version":3,"assistant_text":"Fixture response; no work executed","evidence_refs":[],"action":null});
        crate::output::validate(&result, &packet).unwrap();
        assert_eq!(
            s.dialogue_apply(sent["turn_id"].as_str().unwrap(), &hash, &result)
                .unwrap()["status"],
            "accepted"
        );
        assert!(s.development_records("workflow").unwrap().is_empty());
    }
    #[test]
    fn provider_configuration_stop_allows_switch_without_replaying_input() {
        let (_root, s) = fixture();
        let sent=s.conversation_submit(&json!({"command_id":"fixture-configuration-input","action":"submit_message","payload":{"text":"Fixture only","subject_id":"discussion"}})).unwrap();
        let next = s.dialogue_next().unwrap().unwrap();
        let profile = effective(&s).unwrap();
        let packet = s
            .scope_packet(&next, &profile, &json!(digest(&profile).unwrap()))
            .unwrap();
        let hash = digest(&packet).unwrap();
        let turn = sent["turn_id"].as_str().unwrap();
        assert_eq!(
            s.conversation_reserve(turn, &packet, "conversation")
                .unwrap()["status"],
            "accepted"
        );
        let proof = json!({"category":"configuration","provider_selection_v1":true,"no_provider_process_spawned":true,"known_completion":true,"settled":true,"capabilities_valid":true});
        assert_eq!(
            s.dialogue_stop(turn, &hash, &proof, "Fixture binary unavailable")
                .unwrap()["recoverable"],
            true
        );
        allow_claude(&s);
        super::command(&s, &switch_command("after_current_turn")).unwrap();
        advance_switch(&s).unwrap();
        assert_eq!(current(&s).unwrap()["profile_id"], "claude");
        assert!(s.dialogue_next().unwrap().is_none());
        let status: String = s
            .workflow
            .connection()
            .unwrap()
            .query_row(
                "SELECT status FROM conversation_turns WHERE turn_id=?1",
                [turn],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "blocked");
    }
    #[test]
    fn provider_switch_generation_race_is_durably_blocked() {
        let (_root, s) = fixture();
        allow_claude(&s);
        super::command(&s, &switch_command("after_current_turn")).unwrap();
        s.workflow
            .connection()
            .unwrap()
            .execute("UPDATE conversation_bindings SET generation=2", [])
            .unwrap();
        advance_switch(&s).unwrap();
        assert_eq!(switch(&s).unwrap()["status"], "blocked");
        assert_eq!(current(&s).unwrap()["profile_id"], "codex");
    }
    #[test]
    fn provider_switch_rechecks_policy_and_can_cancel_settled_interruption() {
        let (_root, s) = fixture();
        allow_claude(&s);
        super::command(&s, &switch_command("interrupt")).unwrap();
        // A scope approved while the switch waited must not silently acquire a new provider.
        let work = json!({"work_id":"request","status":"running","policy":crate::development::policy(&s).unwrap()});
        let mut work = work;
        work["policy"]["routing"]["roles"]["coordinator"] =
            json!([current(&s).unwrap()["profile_sha256"]]);
        s.development_save("workflow", "request", &work, "fixture")
            .unwrap();
        advance_switch(&s).unwrap();
        assert_eq!(switch(&s).unwrap()["status"], "blocked");
        assert_eq!(current(&s).unwrap()["profile_id"], "codex");
        super::command(&s, &json!({"command_id":"cancel-settled","action":"providers.cancel_switch","payload":{"expected_revision":2}})).unwrap();
        assert_eq!(switch(&s).unwrap()["status"], "cancelled");
        assert_eq!(
            s.development_record("workflow", "request")
                .unwrap()
                .unwrap(),
            work
        );
    }
    #[test]
    fn provider_scope_policy_includes_actual_binding_while_saved_defaults_change() {
        let (_root, s) = fixture();
        allow_claude(&s);
        let mut p = preferences(&s).unwrap().0.value();
        p["coordinator"] = json!("claude");
        p["roles"]["coordinator"]["default"] = json!("claude");
        p["roles"]["coordinator"]["allowed"] = json!(["claude"]);
        save(&s,&json!({"command_id":"future-coordinator","action":"providers.save","payload":{"expected_revision":2,"preferences":p}})).unwrap();
        let bound = current(&s).unwrap();
        let policy = crate::development::policy(&s).unwrap();
        assert!(policy_allows(
            &policy,
            "coordinator",
            bound["profile_sha256"].as_str().unwrap(),
            &bound["profile"]
        ));
        assert_eq!(bound["profile_id"], "codex");
    }
    #[test]
    fn provider_policy_amendment_is_narrow_and_preserves_scope() {
        let (_root, s) = fixture();
        let original = json!({"work_id":"request","request_id":"fixture","status":"running","revision":1,"scope_base":{"revision":3,"sha256":"original"},"authorization":{"command_id":"fixture-only"},"policy":crate::development::policy(&s).unwrap()});
        s.development_save("workflow", "request", &original, "fixture")
            .unwrap();
        let mut p = prefs();
        p["roles"]["review"]["allowed"] = json!(["codex", "claude"]);
        save(&s,&json!({"command_id":"review-save","action":"providers.save","payload":{"expected_revision":1,"preferences":p}})).unwrap();
        let (prefs, rev) = preferences(&s).unwrap();
        assert!(prefs
            .resolve(
                "review",
                &Value::Null,
                Some("claude"),
                &original["policy"],
                rev,
                ""
            )
            .is_err());
        let command = json!({"command_id":"fixture-policy-amendment","action":"providers.approve_policy","payload":{"expected_revision":rev,"work_id":"request","policy_sha256":digest(&original["policy"]).unwrap(),"routing_sha256":digest(&prefs.allowed_policy().unwrap()).unwrap()}});
        super::command(&s, &command).unwrap();
        let current = s
            .development_record("workflow", "request")
            .unwrap()
            .unwrap();
        assert_eq!(current["scope_base"], original["scope_base"]);
        assert_eq!(current["authorization"], original["authorization"]);
        assert!(policy_extends(&current["policy"], &original["policy"]));
        let mut narrowed = current["policy"].clone();
        narrowed["routing"]["roles"]["implementation"] = json!([]);
        assert!(!policy_extends(&narrowed, &original["policy"]));
        let mut expanded_network = current["policy"].clone();
        expanded_network["dependency_network"] = json!(false);
        assert!(!policy_extends(&expanded_network, &original["policy"]));
        let selected = prefs
            .resolve(
                "review",
                &Value::Null,
                Some("claude"),
                &current["policy"],
                rev,
                "",
            )
            .unwrap();
        assert_eq!(selected["profile_id"], "claude");
        let frozen = selected.clone();
        let mut edited = prefs.value();
        edited["profiles"]["claude"]["model"] = json!("manual-exact-model");
        edited["profiles"]["claude"]["effort"] = json!("provider_default");
        save(&s,&json!({"command_id":"future-edit","action":"providers.save","payload":{"expected_revision":rev,"preferences":edited}})).unwrap();
        assert_eq!(frozen, selected);
        assert!(preferences(&s)
            .unwrap()
            .0
            .resolve(
                "review",
                &Value::Null,
                Some("claude"),
                &current["policy"],
                rev + 1,
                ""
            )
            .is_err());
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn provider_migration_preserves_records_history_and_git_worktrees_and_recovers_publication() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path();
        let state = project.join(".waywr/state");
        std::fs::create_dir_all(&state).unwrap();
        let key = "a".repeat(32);
        let contract = json!({"schema_version":1,"state_schema":8,"execution_host":"systemd-user-v1","project_key":key});
        write(&state.join("state-contract.json"), &contract, true).unwrap();
        let legacy = json!({"schema_version":1,"profile":"codex-development-v1","provider":"codex","mode":"native","binary":"/missing/preserved-codex","model":"gpt-6.1-sol","effort":"medium","concurrency":2,"fallback":[],"dependency_network":true,"main_policy":"checked-fast-forward"});
        write(&project.join(".waywr/coordinator.json"), &legacy, true).unwrap();
        std::fs::write(project.join(".waywr/project.toml"),format!("schema_version = 2\nproject_key = \"{key}\"\n[execution]\nhost = \"systemd-user-v1\"\n[conversation]\nschema_version = 5\n")).unwrap();
        let original = Session::open(project, &state, &key, "fixture-state8").unwrap();
        let row = json!({"work_id":"retained-request","status":"delivered","authorization":{"command_id":"historical-fixture-only"},"policy":{"provider":"codex"},"scope_base":{"revision":1},"head":"recorded"});
        original
            .development_save("workflow", "retained-request", &row, "fixture")
            .unwrap();
        let history = project.join(".waywr/history/fixture.log");
        std::fs::create_dir_all(history.parent().unwrap()).unwrap();
        std::fs::write(&history, "Exact retained fixture history\n").unwrap();
        // Real local Git links and commit identity; no native worker or service executes.
        crate::development::git(project, &["init", "-b", "main"]).unwrap();
        std::fs::write(project.join(".gitignore"), ".waywr/\n").unwrap();
        crate::development::git(project, &["add", ".gitignore"]).unwrap();
        crate::development::git(
            project,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                "Fixture baseline",
            ],
        )
        .unwrap();
        let tree = project.join(".waywr/worktrees/retained");
        crate::development::git(
            project,
            &[
                "worktree",
                "add",
                "-b",
                "waywr/retained",
                tree.to_str().unwrap(),
            ],
        )
        .unwrap();
        let before =
            crate::development::git(project, &["worktree", "list", "--porcelain"]).unwrap();
        let head = crate::development::git(project, &["rev-parse", "HEAD"]).unwrap();
        assert!(migrate_project(project, &state, &key).unwrap());
        assert!(!migrate_project(project, &state, &key).unwrap());
        let imported = Session::open(project, &state, &key, "fixture-state9").unwrap();
        assert_eq!(
            imported
                .development_record("workflow", "retained-request")
                .unwrap()
                .unwrap(),
            row
        );
        assert_eq!(
            std::fs::read_to_string(&history).unwrap(),
            "Exact retained fixture history\n"
        );
        assert_eq!(
            crate::development::git(project, &["worktree", "list", "--porcelain"]).unwrap(),
            before
        );
        assert_eq!(
            crate::development::git(project, &["rev-parse", "HEAD"]).unwrap(),
            head
        );
        let (p, _) = preferences(&imported).unwrap();
        assert_eq!(p.profiles["codex"].binary, "/missing/preserved-codex");
        for role in ROLES {
            assert_eq!(p.role_allowed(role), vec!["codex"]);
            assert!(p.fallback[role].is_empty());
        }
        assert_eq!(
            read(&state.join("state-contract.json")).unwrap()["state_schema"],
            9
        );
        // Half-published DB/config versions resume using the retained original/next journal.
        let path = project.join(".waywr/provider-migration.json");
        let mut journal = read(&path).unwrap();
        journal["status"] = json!("prepared");
        write(&path, &journal, false).unwrap();
        write(&state.join("state-contract.json"), &contract, false).unwrap();
        assert!(migrate_project(project, &state, &key).unwrap());
        journal["status"] = json!("prepared");
        write(&path, &journal, false).unwrap();
        let mut changed = read(&project.join(".waywr/coordinator.json")).unwrap();
        changed["quality_preference"] = json!("Unexpected external edit");
        write(&project.join(".waywr/coordinator.json"), &changed, false).unwrap();
        assert!(migrate_project(project, &state, &key).is_err());
        assert_eq!(
            read(&project.join(".waywr/coordinator.json")).unwrap(),
            changed
        );
    }
}

/// Discussion fallback uses explicit selected-context grants, never worker authority.
pub fn discussion_policy(s: &Session) -> Result<Value, String> {
    let (p, _) = preferences(s)?;
    let mut routing = p.allowed_policy()?;
    let grants = metadata(s, "provider_context_grants")?.unwrap_or(json!([]));
    let allowed = routing["roles"]["coordinator"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|hash| grants.as_array().is_some_and(|g| g.contains(hash)))
        .collect::<Vec<_>>();
    routing["roles"]["coordinator"] = json!(allowed);
    routing["roles"]["implementation"] = json!([]);
    routing["roles"]["review"] = json!([]);
    Ok(
        json!({"routing":routing,"fallback":{"coordinator":p.fallback.get("coordinator").cloned().unwrap_or_default()},"authority":"selected discussion context only; no worker execution"}),
    )
}

/// State8 approvals retain the exact imported Codex profile through revision1, not mutable defaults.
/// This is an authority projection; it never rewrites the historical approval or its policy hash.
pub fn normalized_policy(s: &Session, policy: &Value) -> Result<Value, String> {
    if policy["routing"].is_object() {
        return Ok(policy.clone());
    }
    let raw: String = s
        .workflow
        .connection()?
        .query_row(
            "SELECT record_json FROM provider_preference_revisions WHERE revision=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let initial = Preferences::parse(&serde_json::from_str(&raw).map_err(|e| e.to_string())?)?;
    let mut routing = initial.allowed_policy()?;
    for role in ROLES {
        let hashes = routing["roles"][role]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|hash| {
                hash.as_str().is_some_and(|hash| {
                    policy_allows(policy, role, hash, &routing["profiles"][hash]["profile"])
                })
            })
            .collect::<Vec<_>>();
        routing["roles"][role] = json!(hashes);
    }
    let mut value = policy.clone();
    value["routing"] = routing;
    Ok(value)
}
/// Additive provider amendments preserve prior grants; other execution boundaries must be identical.
pub fn policy_extends(current: &Value, frozen: &Value) -> bool {
    let mut a = current.clone();
    let mut b = frozen.clone();
    for key in ["routing", "fallback", "schema_version"] {
        if let Some(a) = a.as_object_mut() {
            a.remove(key);
        }
        if let Some(b) = b.as_object_mut() {
            b.remove(key);
        }
    }
    if a != b {
        return false;
    }
    for role in ROLES {
        for hash in frozen["routing"]["roles"][role]
            .as_array()
            .into_iter()
            .flatten()
        {
            let Some(hash) = hash.as_str() else {
                return false;
            };
            if !current["routing"]["roles"][role]
                .as_array()
                .is_some_and(|values| values.contains(&json!(hash)))
                || current["routing"]["profiles"][hash]["profile"]
                    != frozen["routing"]["profiles"][hash]["profile"]
            {
                return false;
            }
        }
    }
    true
}
/// Shared by service permit issuance and the runner's once-only commit-spawn guard.
pub fn packet_authorized(s: &Session, work: &Value, packet: &Value) -> Result<bool, String> {
    let current = normalized_policy(s, &work["policy"])?;
    let frozen = normalized_policy(s, &packet["policy"])?;
    if !policy_extends(&current, &frozen) {
        return Ok(false);
    }
    let role = match packet["operation_kind"].as_str() {
        Some("development-worker") => "implementation",
        Some("development-review") => "review",
        Some("development-coordinator") => "coordinator",
        Some("development-check") => return Ok(true),
        _ => return Ok(false),
    };
    if role == "coordinator" && switch_pending(s) && switch(s)?["mode"] == "interrupt" {
        return Ok(false);
    }
    let execution = &packet["execution"];
    let profile = if execution["profile"] == "development-profiles-v1" {
        let selected = &execution["selection"];
        let profile = selected["profile"].clone();
        if selected["profile_sha256"] != digest(&profile)? {
            return Ok(false);
        }
        for field in [
            "provider",
            "binary",
            "model",
            "effort",
            "binary_sha256",
            "context_bytes",
            "transport",
            "options",
        ] {
            if execution[field] != profile[field] {
                return Ok(false);
            }
        }
        if !execution["client"].is_null(){
            let client=&execution["client"];
            if client["provider"]!=profile["provider"] || client["locator"]!=profile["binary"] ||
                client["binary"].as_str().is_none_or(|p|!Path::new(p).is_absolute()) ||
                client["sha256"].as_str().is_none_or(|h|h.len()!=64||!h.bytes().all(|b|b.is_ascii_hexdigit())){
                return Ok(false);
            }
        }
        if execution["dependency_network"] != packet["policy"]["dependency_network"] {
            return Ok(false);
        }
        profile
    } else {
        // Old prepared packets remain immutable. Their binary must match the exact upgrade import.
        let binary = execution["binary"]
            .as_str()
            .ok_or("Legacy packet binary missing")?;
        json!({"provider":execution["provider"],"binary":binary,"model":execution["model"],"effort":execution["effort"],"context_bytes":98304,"binary_sha256":crate::execution::hash_file(Path::new(binary))?})
    };
    Ok(crate::request_routing::pin_allows(&frozen,role,&digest(&profile)?,&profile)
        && packet["provider"] == profile["provider"]
        && packet["model"] == profile["model"]
        && policy_allows(&frozen, role, &digest(&profile)?, &profile))
}
