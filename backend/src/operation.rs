//! Durable launch/claim/permit/receipt reconciliation for one frozen operation.
//! Herdr is a terminal provider; immutable runner evidence proves completion.
use crate::{
    codec::{canonical_sha256, sha256},
    context_access::GrantReader,
    execution::{
        FileLock, alive, contained_operation, hash_file, identity_matches, now, random_token, read, write,
    },
    herdr::{CreateError, Herdr},
    output,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

pub const RECOVERY_ACTION: &str = "Inspect status, launch/claim/child records and retained output. Do not delete evidence or retry dispatch; reconcile the original server/worker or request explicit recovery after execution is resolved.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Operational,
    PreSendRefusal,
    Uncertain,
    OutputInvalid,
}
#[derive(Debug)]
pub struct OpError {
    pub kind: ErrorKind,
    pub detail: String,
}
impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.detail)
    }
}
impl std::error::Error for OpError {}
impl From<String> for OpError {
    fn from(detail: String) -> Self {
        Self {
            kind: ErrorKind::Operational,
            detail,
        }
    }
}
impl From<&str> for OpError {
    fn from(detail: &str) -> Self {
        Self {
            kind: ErrorKind::Operational,
            detail: detail.to_owned(),
        }
    }
}
fn uncertain(detail: impl Into<String>) -> OpError {
    OpError {
        kind: ErrorKind::Uncertain,
        detail: detail.into(),
    }
}
fn invalid(detail: impl Into<String>) -> OpError {
    OpError {
        kind: ErrorKind::OutputInvalid,
        detail: detail.into(),
    }
}
pub type Result<T> = std::result::Result<T, OpError>;

pub trait Control {
    fn server_identity(&self) -> std::result::Result<Value, String>;
    fn workspace(&self, id: &str) -> std::result::Result<Value, String>;
    fn create(
        &self,
        workspace: &str,
        label: &str,
        command: &[String],
        cwd: &Path,
        initial_tab: Option<&str>,
    ) -> std::result::Result<Value, CreateError>;
    fn find(&self, token: &str, runner: &Path) -> std::result::Result<Vec<Value>, String>;
    fn pane(&self, id: &str) -> std::result::Result<Value, String>;
}
impl Control for Herdr {
    fn server_identity(&self) -> std::result::Result<Value, String> {
        Herdr::server_identity(self)
    }
    fn workspace(&self, id: &str) -> std::result::Result<Value, String> {
        Herdr::workspace(self, id)
    }
    fn create(
        &self,
        workspace: &str,
        label: &str,
        command: &[String],
        cwd: &Path,
        initial_tab: Option<&str>,
    ) -> std::result::Result<Value, CreateError> {
        Herdr::create(self, workspace, label, command, cwd, initial_tab)
    }
    fn find(&self, token: &str, runner: &Path) -> std::result::Result<Vec<Value>, String> {
        Herdr::find(self, token, runner)
    }
    fn pane(&self, id: &str) -> std::result::Result<Value, String> {
        Herdr::pane(self, id)
    }
}

pub fn prepare(
    state: &Path,
    directory: &Path,
    packet: &Value,
    target: &Value,
    project_key: Option<&str>,
) -> Result<Value> {
    let directory = contained_operation(state, directory)?;
    let packet_hash = canonical_sha256(packet).map_err(|e| e.to_string())?;
    write(&directory.join("packet.json"), packet, true)?;
    let mut op = json!({"schema_version":1,"operation_id":packet["operation_id"],
        "run_id":packet["run_id"],"stage":packet["stage"],"role":packet["role"],
        "provider":packet.get("provider").cloned().unwrap_or_else(||json!("fake")),
        "model":packet["model"],"packet_sha256":packet_hash,
        "launch_token":random_token()?,"target":target,"created":now()});
    if let Some(key) = project_key {
        op["project_key"] = key.into();
        op["protocol_version"] = 1.into();
    }
    if let Some(limit) = packet.get("max_invocations") {
        op["max_invocations"] = limit.clone();
        op["authorization_plan_sha256"] = packet["authorization_plan_sha256"].clone();
    }
    if let Some(version) = packet.get("application_version") {
        op["application_version"] = version.clone();
    }
    let path = directory.join("operation.json");
    if path.exists() {
        let old = read(&path)?;
        if old["packet_sha256"] != op["packet_sha256"] {
            return Err("Existing stage has a different packet".to_owned().into());
        }
        return Ok(old);
    }
    write(&path, &op, true)?;
    Ok(op)
}

/// New logical operation contract. The display target is deliberately absent;
/// the separate host attempt is allocated only after workflow reservation.
pub fn prepare_host(state:&Path,directory:&Path,packet:&Value,project_key:&str)->Result<Value>{
    let directory=contained_operation(state,directory)?;
    if project_key.len()!=32 || !project_key.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err("Invalid host project key".into());
    }
    if packet["execution"]["profile"]=="development-profiles-v1"{
        let modern=packet["context_delivery"]["version"]==1;
        let size=if modern{serde_json::to_vec(packet).map_err(|e|e.to_string())?.len()}else{crate::codec::canonical(packet).map_err(|e|e.to_string())?.len()};
        let limit=if modern{crate::development_context::MEMORY_LIMIT}else{packet["execution"]["context_bytes"].as_u64().unwrap_or(96*1024) as usize};
        if size>limit{return Err("Frozen packet exceeds application memory bound; no host attempt prepared".into());}
    }
    let packet_hash=canonical_sha256(packet).map_err(|e|e.to_string())?;
    if !packet["execution"]["client"].is_null(){
        write(&state.join("native-client-identity-v1.json"),&json!({"version":1,"contract":"native-client-v1"}),true)?;
    }
    if packet["execution"]["runtime"]["protocol"]==1{
        write(&state.join("model-runtime-v1.json"),&json!({"version":1,"contract":"shared-models-v1"}),true)?;
    }
    write(&directory.join("packet.json"),packet,true)?;
    let op=json!({"schema_version":2,"execution_host":"systemd-user-v1",
        "operation_id":packet["operation_id"],"run_id":packet["run_id"],
        "stage":packet["stage"],"role":packet["role"],
        "provider":packet.get("provider").cloned().unwrap_or_else(||json!("fake")),
        "model":packet["model"],"packet_sha256":packet_hash,"project_key":project_key,
        "max_invocations":packet.get("max_invocations"),
        "authorization_plan_sha256":packet.get("authorization_plan_sha256"),
        "application_version":packet.get("application_version"),"created":now()});
    let path=directory.join("operation.json");
    if path.exists(){
        let old=read(&path)?;
        if old["schema_version"]!=2 || old["packet_sha256"]!=packet_hash ||
            old["project_key"]!=project_key || old["execution_host"]!="systemd-user-v1"{
            return Err("Existing host operation differs from frozen input".into());
        }
        return Ok(old);
    }
    write(&path,&op,true)?;Ok(op)
}

pub fn result_path(directory:&Path)->Result<PathBuf>{
    let op=read(&directory.join("operation.json"))?;
    if op["schema_version"]==2{
        let state=directory.ancestors().find(|p|p.join("project-identity.json").exists())
            .ok_or("Host operation has no project state root")?;
        let attempt=crate::host_attempt::Attempt::load(state,directory)?;
        Ok(attempt.directory.join("result.json"))
    }else{Ok(directory.join("result.json"))}
}

pub fn receipt(directory: &Path) -> Result<Option<Value>> {
    let op=read(&directory.join("operation.json"))?;
    if op["schema_version"]==2{
        let state=directory.ancestors().find(|p|p.join("project-identity.json").exists())
            .ok_or("Host operation has no project state root")?;
        if !directory.join("active-host-attempt.json").exists(){return Ok(None);}
        let attempt=crate::host_attempt::Attempt::load(state,directory)?;
        return attempt.validated_receipt().map_err(uncertain);
    }
    let path = directory.join("receipt.json");
    if !path.exists() {
        return Ok(None);
    }
    let done = read(&path)?;
    if ["operation_id", "packet_sha256", "launch_token"]
        .iter()
        .any(|k| done[*k] != op[*k])
    {
        return Err(uncertain("Receipt identity mismatch"));
    }
    let claim =
        read(&directory.join("claim.json")).map_err(|_| uncertain("Receipt lacks runner claim"))?;
    if done["runner"] != claim["runner"] {
        return Err(uncertain("Receipt runner mismatch"));
    }
    match done["status"].as_str() {
        Some("completed") => {
            let raw = fs::read(directory.join("result.json"))
                .map_err(|_| uncertain("Completed receipt lacks result bytes"))?;
            if sha256(&raw) != done["result_sha256"] {
                return Err(uncertain("Result hash mismatch"));
            }
            let value: Value = serde_json::from_slice(&raw)
                .map_err(|e| invalid(format!("Saved output invalid: {e}")))?;
            let packet = read(&directory.join("packet.json"))?;
            output::validate(&value, &packet)
                .map_err(|e| invalid(format!("Saved output invalid: {e}")))?;
        }
        Some("failed" | "cancelled") => {}
        _ => return Err(uncertain("Unknown receipt outcome")),
    }
    Ok(Some(done))
}

#[derive(Clone)]
pub struct OwnerFence {
    pub state: PathBuf,
    pub generation: Option<u64>,
}
impl OwnerFence {
    fn check(&self) -> Result<()> {
        let path = self.state.join("supervision/owner.json");
        match (path.exists(), self.generation) {
            (false, None) => Ok(()),
            (false, Some(_)) => Err("Missing controller owner record; dispatch refused"
                .to_owned()
                .into()),
            (true, None) => Err("Unowned controller; dispatch refused".to_owned().into()),
            (true, Some(expected)) => {
                let owner = read(&path)?;
                if owner["generation"] == expected {
                    Ok(())
                } else {
                    Err("Stale or unowned controller; dispatch refused"
                        .to_owned()
                        .into())
                }
            }
        }
    }
    pub fn with<T>(&self, action: impl FnOnce() -> Result<T>) -> Result<T> {
        let _guard = FileLock::exclusive(&self.state.join("supervision/fence.lock"))?;
        self.check()?;
        action()
    }
}

fn server_matches(control: &dyn Control, expected: Option<&Value>) -> Result<Value> {
    let actual = control.server_identity().map_err(|_| {
        uncertain(format!(
            "Herdr server unavailable; no dispatch/permit. {RECOVERY_ACTION}"
        ))
    })?;
    if expected.is_some_and(|old| !identity_matches(old, &actual)) {
        return Err(uncertain(format!(
            "Herdr server changed; no automatic relaunch. {RECOVERY_ACTION}"
        )));
    }
    Ok(actual)
}

fn verify_imports(state: &Path, op: &Value, packet: &Value) -> Result<()> {
    let run = op["run_id"].as_str().ok_or("missing run ID")?;
    let envelope_path = state.join("runs").join(run).join("inputs/envelope.json");
    let imports = if envelope_path.exists() {
        read(&envelope_path)?["cross_imports"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if imports.is_empty() {
        if packet["cross_project_imports"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
        {
            return Err("Execution packet adds an unauthorized foreign import"
                .to_owned()
                .into());
        }
        return Ok(());
    }
    let expected = imports
        .iter()
        .map(|entry| {
            json!({"source_project_key":entry["item"]["source_project_key"],
        "grant_id":entry["grant_id"],"purpose":entry["purpose"],"item":entry["item"]})
        })
        .collect::<Vec<_>>();
    if packet["cross_project_imports"] != json!(expected) {
        return Err("Execution packet differs from frozen imported context"
            .to_owned()
            .into());
    }
    let key = op["project_key"]
        .as_str()
        .ok_or("Cross-project permit has no selected project")?;
    let _lock = FileLock::exclusive(&state.join("session/admission.lock"))?;
    GrantReader {
        state: state.to_owned(),
        recipient_key: key.into(),
    }
    .check_frozen(&imports)?;
    Ok(())
}

pub struct Ensure<'a> {
    pub state: &'a Path,
    pub directory: &'a Path,
    pub runner_binary: &'a Path,
    pub control: &'a dyn Control,
    pub fence: &'a OwnerFence,
    /// A short project dispatch critical section; never held across Herdr I/O.
    pub launch_gate: Option<&'a Mutex<()>>,
    pub before_launch: Option<&'a dyn Fn() -> Result<()>>,
}
impl Ensure<'_> {
    pub fn run(&self) -> Result<Value> {
        let directory = contained_operation(self.state, self.directory)?;
        let _control_lock = FileLock::exclusive(&directory.join("controller.lock"))?;
        if let Some(done) = receipt(&directory)? {
            return Ok(done);
        }
        let refusal_path=directory.join("pre-send-refusal.json");
        if refusal_path.exists() {
            let refusal=read(&refusal_path)?;
            if refusal["kind"]!="pre-send-refusal" ||
                refusal["launch_sha256"]!=hash_file(&directory.join("launch.json"))? {
                return Err(uncertain("Pre-send refusal evidence changed; no dispatch"));
            }
            return Err(OpError{kind:ErrorKind::PreSendRefusal,
                detail:format!("Herdr refused the bootstrap tab before worker dispatch: {}",
                    refusal["detail"].as_str().unwrap_or("inspect saved refusal"))});
        }
        if directory.join("uncertain.json").exists() {
            return Err(uncertain(
                read(&directory.join("uncertain.json"))?["error"]
                    .as_str()
                    .unwrap_or("Uncertain execution"),
            ));
        }
        let op = read(&directory.join("operation.json"))?;
        let target = &op["target"];
        let workspace = target["workspace"]
            .as_str()
            .ok_or("missing target workspace")?;
        let launch_path = directory.join("launch.json");
        let server = if launch_path.exists() {
            server_matches(self.control, None)?
        } else {
            self.control.server_identity().map_err(|e| {
                OpError::from(format!("Herdr server unavailable before launch: {e}"))
            })?
        };
        let launch = if launch_path.exists() {
            let launch = read(&launch_path)?;
            if !identity_matches(&launch["server"], &server) {
                return Err(uncertain(format!(
                    "Herdr server changed; no automatic relaunch. {RECOVERY_ACTION}"
                )));
            }
            launch
        } else {
            if ["claim.json", "dispatch.json", "child.json", "permit.json"]
                .iter()
                .any(|name| directory.join(name).exists())
            {
                return Err(uncertain(format!(
                    "Execution evidence exists without launch intent. {RECOVERY_ACTION}"
                )));
            }
            self.control
                .workspace(workspace)
                .map_err(|e| OpError::from(format!("Herdr workspace unavailable before launch: {e}")))?;
            let _gate=self.launch_gate.map(|gate|gate.lock().map_err(|e|OpError::from(e.to_string())))
                .transpose()?;
            let launch=self.fence.with(|| {
                if let Some(check)=self.before_launch{check()?;}
                let launch = json!({"status":"may-have-been-sent","server":server,
                    "target":target,"created":now()});
                write(&launch_path, &launch, true)?;
                Ok(launch)
            })?;
            drop(_gate);
                let prefix = op["project_key"]
                    .as_str()
                    .map(|s| format!("{} ", &s[..s.len().min(8)]))
                    .unwrap_or_default();
                let label = format!(
                    "Wayward Wrangler {}{} {} {}/{} {}",
                    prefix,
                    op["run_id"].as_str().unwrap_or(""),
                    op["role"].as_str().unwrap_or(""),
                    op["provider"].as_str().unwrap_or(""),
                    op["model"].as_str().unwrap_or(""),
                    op["stage"].as_str().unwrap_or("")
                );
                let argv = vec![
                    self.runner_binary.to_string_lossy().into_owned(),
                    "run".into(),
                    "--operation-dir".into(),
                    directory.to_string_lossy().into_owned(),
                    "--launch-token".into(),
                    op["launch_token"]
                        .as_str()
                        .ok_or("missing launch token")?
                        .into(),
                    "--state-dir".into(),
                    self.state.to_string_lossy().into_owned(),
                ];
                self.control.create(workspace, &label, &argv, &directory,
                    target["initial_tab_id"].as_str()).map_err(|e| match e {
                    CreateError::PreSend(detail) => {
                        let record=hash_file(&launch_path).and_then(|hash|write(&directory.join("pre-send-refusal.json"),
                            &json!({"schema_version":1,"kind":"pre-send-refusal",
                                "launch_sha256":hash,"detail":detail,"created":now()}),true));
                        match record {
                            Ok(()) => OpError{kind:ErrorKind::PreSendRefusal,
                                detail:format!("Herdr refused the bootstrap tab before worker dispatch: {detail}")},
                            Err(record_error) => uncertain(format!("Pre-send refusal could not be recorded ({record_error}); inspect launch evidence")),
                        }
                    }
                    CreateError::MaybeSent(detail) =>
                        uncertain(format!("Herdr creation may have been sent; no retry: {detail}")),
                })?;
                launch
        };
        let deadline = Instant::now() + Duration::from_secs(8);
        while !directory.join("claim.json").exists() && Instant::now() < deadline {
            if let Some(done) = receipt(&directory)? {
                return Ok(done);
            }
            thread::sleep(Duration::from_millis(100));
        }
        if let Some(done) = receipt(&directory)? {
            return Ok(done);
        }
        if !directory.join("claim.json").exists() {
            return Err(uncertain(format!(
                "Creation may have succeeded, but no runner claim; no retry. {RECOVERY_ACTION}"
            )));
        }
        let claim = read(&directory.join("claim.json"))?;
        if !alive(&claim["runner"]) {
            return Err(uncertain(format!(
                "Runner disappeared without completion evidence. {RECOVERY_ACTION}"
            )));
        }
        let token = op["launch_token"].as_str().ok_or("missing launch token")?;
        let matches = self
            .control
            .find(token, self.runner_binary)
            .map_err(|e| uncertain(format!("Existing worker pane cannot be located: {e}")))?;
        if matches.len() != 1 {
            return Err(uncertain(format!(
                "Expected one matching worker pane, found {}",
                matches.len()
            )));
        }
        let mut binding = matches[0].clone();
        for key in ["pane_id", "tab_id", "workspace_id"] {
            if claim["herdr"][key] != binding[key] {
                return Err(uncertain("Runner and Herdr disagree about pane identity"));
            }
        }
        if binding["workspace_id"] != workspace {
            return Err(uncertain("Worker moved to an unexpected workspace"));
        }
        let pane = self
            .control
            .pane(binding["pane_id"].as_str().ok_or("missing pane ID")?)
            .map_err(|e| uncertain(format!("Worker pane unavailable: {e}")))?;
        binding["terminal_id"] = pane["terminal_id"].clone();
        let binding_path = directory.join("binding.json");
        if binding_path.exists() && read(&binding_path)? != binding {
            return Err(uncertain("Herdr terminal binding changed"));
        }
        write(&binding_path, &binding, true)?;
        server_matches(self.control, Some(&launch["server"]))?;
        self.fence.with(|| {
            let permit_path = directory.join("permit.json");
            if permit_path.exists() {
                let permit = read(&permit_path)?;
                if permit["runner"] != claim["runner"]
                    || permit["launch_token"] != op["launch_token"]
                    || permit["binding"] != binding
                {
                    return Err(uncertain(
                        "Existing dispatch permit differs from verified worker identity",
                    ));
                }
            } else {
                let packet = read(&directory.join("packet.json"))?;
                verify_imports(self.state, &op, &packet)?;
                write(
                    &permit_path,
                    &json!({"runner":claim["runner"],
                    "launch_token":op["launch_token"],"binding":binding}),
                    true,
                )?;
            }
            Ok(json!({"status":"active","binding":binding,"runner":claim["runner"]}))
        })
    }
}

pub fn cancel(directory: &Path, source: &str) -> Result<Value> {
    if let Some(done) = receipt(directory)? {
        return Ok(done);
    }
    let claim = read(&directory.join("claim.json"))
        .map_err(|_| uncertain("Cannot cancel: runner ownership unresolved"))?;
    if !alive(&claim["runner"]) {
        return Err(uncertain("Cannot cancel: runner ownership unresolved"));
    }
    write(
        &directory.join("cancel.json"),
        &json!({"source":source,"runner":claim["runner"],"time":now()}),
        false,
    )?;
    Ok(json!({"status":"cancellation-requested"}))
}

pub fn inspect(directory: &Path, control: Option<&dyn Control>) -> Value {
    match receipt(directory) {
        Ok(Some(done)) => {
            return json!({"status":done["status"],"detail":"Validated immutable completion receipt","operator_action":null});
        }
        Err(error) => {
            return json!({"status":"uncertain","detail":error.detail,"operator_action":RECOVERY_ACTION});
        }
        Ok(None) => {}
    }
    let outcome = (|| -> Result<Value> {
        let op = read(&directory.join("operation.json"))?;
        let refusal=directory.join("pre-send-refusal.json");
        if refusal.exists(){
            let refusal=read(&refusal)?;
            if refusal["kind"]!="pre-send-refusal" ||
                refusal["launch_sha256"]!=hash_file(&directory.join("launch.json"))? ||
                ["claim.json","dispatch.json","child.json","permit.json"]
                    .iter().any(|name|directory.join(name).exists()){
                return Err(uncertain("Pre-send refusal conflicts with execution evidence"));
            }
            return Ok(json!({"status":"blocked","detail":refusal["detail"],
                "operator_action":"Inspect the saved refusal and resolve the bootstrap-tab conflict before requesting a new authorized launch"}));
        }
        let claim = if directory.join("claim.json").exists() {
            Some(read(&directory.join("claim.json"))?)
        } else {
            None
        };
        let child = if directory.join("child.json").exists() {
            Some(read(&directory.join("child.json"))?)
        } else {
            None
        };
        let runner_alive = claim.as_ref().is_some_and(|v| alive(&v["runner"]));
        let child_alive = child.as_ref().is_some_and(|v| alive(&v["identity"]));
        if directory.join("uncertain.json").exists() {
            return Err(uncertain(
                read(&directory.join("uncertain.json"))?["error"]
                    .as_str()
                    .unwrap_or("Uncertain execution"),
            ));
        }
        if claim.is_some() && !runner_alive {
            return Ok(
                json!({"runner_alive":false,"child_alive":child_alive,"status":"uncertain",
                "detail":if child_alive{"Runner disappeared without receipt; recorded child is still alive"}
                    else{"Runner disappeared without receipt"},"operator_action":RECOVERY_ACTION}),
            );
        }
        if directory.join("launch.json").exists() {
            let launch = read(&directory.join("launch.json"))?;
            if let Some(control) = control {
                server_matches(control, Some(&launch["server"]))?;
            }
            if claim.is_none() {
                return Err(uncertain("Launch may have been sent; no runner claim"));
            }
        } else if claim.is_some()
            || ["dispatch.json", "child.json", "permit.json"]
                .iter()
                .any(|n| directory.join(n).exists())
        {
            return Err(uncertain("Execution evidence exists without launch intent"));
        }
        let status = if directory.join("permit.json").exists() {
            "active"
        } else if claim.is_some() {
            "awaiting-permit"
        } else {
            "prepared"
        };
        let _ = op;
        Ok(
            json!({"runner_alive":runner_alive,"child_alive":child_alive,"status":status,
            "detail":"No terminal receipt; resume reconciles existing execution","operator_action":null}),
        )
    })();
    match outcome {
        Ok(value) => value,
        Err(error) => json!({"status":"uncertain","detail":error.detail,
        "operator_action":RECOVERY_ACTION}),
    }
}
