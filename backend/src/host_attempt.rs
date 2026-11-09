//! Versioned, append-only execution attempt gate for the systemd host.
//! This module owns no workflow decisions. A missing unit, PID, or receipt is
//! never proof that an external invocation did not happen.
use crate::{
    codec::sha256,
    execution::{self, FileLock, read, write},
    systemd_host::{HostStatus, UserManager},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

const VERSION: u64 = 1;

#[derive(Clone)]
pub struct Attempt {
    pub state: PathBuf,
    pub operation: PathBuf,
    pub directory: PathBuf,
    pub id: String,
    pub unit: String,
}

fn hex(input: &str, len: usize) -> bool {
    input.len() == len
        && input
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn lock(path: &Path) -> Result<FileLock, String> {
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        match FileLock::exclusive(path) {
            Ok(guard) => return Ok(guard),
            Err(error) if error.starts_with("Already owned:") && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(2))
            }
            Err(error) => return Err(error),
        }
    }
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().ok_or_else(|| format!("Missing {key}"))
}
fn marker(directory: &Path, name: &str, value: &Value) -> Result<(), String> {
    let path = directory.join(name);
    if path.exists() {
        let old = read(&path)?;
        let Some(fields) = value.as_object() else {
            return Err("Attempt marker must be an object".into());
        };
        if fields.iter().all(|(key, expected)| {
            matches!(key.as_str(), "time" | "observed") || old[key] == *expected
        }) {
            return Ok(());
        }
        return Err(format!(
            "Conflicting immutable attempt marker: {}",
            path.display()
        ));
    }
    write(&path, value, true)
}
fn owner_generation(state: &Path) -> Result<u64, String> {
    read(&state.join("supervision/owner.json"))?["generation"]
        .as_u64()
        .ok_or("Invalid project owner generation".into())
}

impl Attempt {
    /// Create one new host identity. A linked replacement is only possible
    /// after a verified retirement marker; no name or attempt ID is reused.
    pub fn create(
        state: &Path,
        operation: &Path,
        key: &str,
        runner: &Path,
        release: &str,
        generation: u64,
    ) -> Result<Self, String> {
        if !hex(key, 32) {
            return Err("Invalid project key for host unit".into());
        }
        let operation = execution::contained_operation(state, operation)?;
        let state = state.canonicalize().map_err(|e| e.to_string())?;
        let runner = runner.canonicalize().map_err(|e| e.to_string())?;
        if !runner.is_file() {
            return Err("Bound runner executable unavailable".into());
        }
        let _project = lock(&state.join("supervision/fence.lock"))?;
        let _op = lock(&operation.join("host-operation.lock"))?;
        if owner_generation(&state)? != generation {
            return Err("Stale supervisor generation".into());
        }
        let root_op = read(&operation.join("operation.json"))?;
        if root_op["schema_version"] != 2 || root_op["project_key"]!=key {
            return Err("Host attempt requires operation schema 2".into());
        }
        let packet_hash = required(&root_op, "packet_sha256")?;
        // The packet hash in the operation is canonical JSON, not raw bytes.
        let packet = read(&operation.join("packet.json"))?;
        if crate::codec::canonical_sha256(&packet).map_err(|e| e.to_string())? != packet_hash {
            return Err("Frozen packet hash mismatch".into());
        }
        let pointer = operation.join("active-host-attempt.json");
        let previous = if pointer.exists() {
            let old = read(&pointer)?;
            let old_id = required(&old, "attempt_id")?;
            if !hex(old_id, 32)
                || !operation
                    .join("attempts")
                    .join(old_id)
                    .join("retired.json")
                    .exists()
            {
                return Err("Existing host attempt is not proven retired".into());
            }
            Some(old_id.to_owned())
        } else {
            None
        };
        let id = execution::random_token()?;
        let digest = sha256(required(&root_op, "operation_id")?.as_bytes());
        let unit = format!("waywr-op-{key}-{}-{id}.service", &digest[..16]);
        let directory = operation.join("attempts").join(&id);
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let intent = json!({"schema_version":VERSION,"attempt_id":id,"unit":unit,
            "project_key":key,"operation_id":root_op["operation_id"],
            "packet_sha256":packet_hash,"runner":runner,"runner_sha256":execution::hash_file(&runner)?,
            "release_id":release,"owner_generation":generation,"previous_attempt":previous,
            "created":execution::now()});
        marker(&directory, "intent.json", &intent)?;
        write(
            &pointer,
            &json!({"schema_version":VERSION,"attempt_id":id,"unit":unit,
            "previous_attempt":previous}),
            false,
        )?;
        Ok(Self {
            state,
            operation,
            directory,
            id,
            unit,
        })
    }

    pub fn load(state: &Path, operation: &Path) -> Result<Self, String> {
        let state = state.canonicalize().map_err(|e| e.to_string())?;
        let operation = execution::contained_operation(&state, operation)?;
        let pointer = read(&operation.join("active-host-attempt.json"))?;
        let id = required(&pointer, "attempt_id")?.to_owned();
        if !hex(&id, 32) {
            return Err("Invalid host attempt ID".into());
        }
        let directory = operation.join("attempts").join(&id);
        let intent = read(&directory.join("intent.json"))?;
        let unit = required(&intent, "unit")?.to_owned();
        if pointer["unit"] != unit || intent["attempt_id"] != id {
            return Err("Host attempt pointer mismatch".into());
        }
        Ok(Self {
            state,
            operation,
            directory,
            id,
            unit,
        })
    }

    pub fn intent(&self) -> Result<Value, String> {
        read(&self.directory.join("intent.json"))
    }
    /// Repair the append-only link if the supervisor died after advancing the
    /// rebuildable active pointer but before publishing its recovery record.
    pub fn reconcile_recovery_link(&self)->Result<(),String>{
        let intent=self.intent()?;
        let Some(previous)=intent["previous_attempt"].as_str() else{return Ok(());};
        if !hex(previous,32){return Err("Invalid previous attempt identity".into());}
        let old=self.operation.join("attempts").join(previous);
        if !old.join("retired.json").exists(){return Err("Replacement lacks retired source proof".into());}
        let old_intent=read(&old.join("intent.json"))?;
        if old_intent["operation_id"]!=intent["operation_id"] ||
            old_intent["project_key"]!=intent["project_key"]{
            return Err("Replacement source identity differs".into());
        }
        let path=old.join("recovery.json");
        if path.exists(){
            let saved=read(&path)?;
            if saved["source_attempt"]!=previous || saved["replacement_attempt"]!=self.id ||
                saved["source_intent_sha256"]!=execution::hash_file(&old.join("intent.json"))?{
                return Err("Immutable recovery link conflicts with active replacement".into());
            }
            return Ok(());
        }
        write(&path,&json!({"schema_version":1,"source_attempt":previous,
            "replacement_attempt":self.id,"reason":"proven pre-launch infrastructure failure",
            "source_intent_sha256":execution::hash_file(&old.join("intent.json"))?,
            "owner_generation":intent["owner_generation"],"time_unix_ms":(execution::now()*1000.0) as u64}),true)
    }
    fn current(&self) -> Result<(), String> {
        let pointer = read(&self.operation.join("active-host-attempt.json"))?;
        if pointer["attempt_id"] != self.id || pointer["unit"] != self.unit {
            return Err("Host attempt is no longer active".into());
        }
        Ok(())
    }
    fn gate(&self) -> Result<(FileLock, FileLock), String> {
        let project = lock(&self.state.join("supervision/fence.lock"))?;
        let op = lock(&self.operation.join("host-operation.lock"))?;
        Ok((project, op))
    }
    fn validate_owner(&self) -> Result<(), String> {
        let intent = self.intent()?;
        let current = owner_generation(&self.state)?;
        if current
            != intent["owner_generation"]
                .as_u64()
                .ok_or("Invalid owner generation")?
        {
            let endorsement = self.directory.join(format!("endorsement-{current}.json"));
            if !endorsement.exists() || read(&endorsement)?["owner_generation"] != current {
                return Err("Stale owner cannot authorize host attempt".into());
            }
        }
        self.current()?;
        if self.directory.join("retired.json").exists() {
            return Err("Host attempt is retired".into());
        }
        Ok(())
    }
    /// This marker is written before the D-Bus request; transport failure from
    /// here onward is a possibly sent request, never automatic no-launch proof.
    pub fn mark_maybe_sent(&self) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.validate_owner()?;
        if self.directory.join("start-maybe-sent.json").exists()
            || self.directory.join("prelaunch-refusal.json").exists()
        {
            return Err(
                "Host start already attempted; reconcile exact unit before any action".into(),
            );
        }
        marker(
            &self.directory,
            "start-maybe-sent.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"unit":self.unit,"time":execution::now()}),
        )
    }
    fn record_pre_enqueue_refusal(&self, reason: &str) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.validate_owner()?;
        if self.directory.join("start-maybe-sent.json").exists() {
            return Err("Cannot prove pre-enqueue refusal after possible send".into());
        }
        marker(
            &self.directory,
            "prelaunch-refusal.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"proof":"user-bus connection refused before enqueue",
            "reason":reason,"time":execution::now()}),
        )
    }

    /// The old owner cannot enqueue after the new owner takes the project
    /// fence. Absence of the before-send marker is positive no-enqueue proof
    /// only under this versioned start path and that acquired fence.
    pub fn retire_fenced_unsent_owner(&self, new_generation:u64)->Result<(),String>{
        let (_project,_op)=self.gate()?;
        self.current()?;
        let intent=self.intent()?;
        if owner_generation(&self.state)?!=new_generation ||
            intent["owner_generation"]==new_generation ||
            ["start-maybe-sent.json","claim.json","permit.json","spawn-committed.json"]
                .iter().any(|name|self.directory.join(name).exists()) ||
            self.operation.join("invocation-reservation.json").exists(){
            return Err("Unsent-owner retirement lacks positive pre-enqueue proof".into());
        }
        marker(&self.directory,"prelaunch-refusal.json",&json!({"schema_version":VERSION,
            "attempt_id":self.id,"proof":"prior owner fenced before start enqueue",
            "new_owner_generation":new_generation,"time":execution::now()}))?;
        marker(&self.directory,"retired.json",&json!({"schema_version":VERSION,
            "attempt_id":self.id,"proof":"pre-enqueue","reason":"supervisor owner changed before start",
            "time":execution::now()}))
    }
    pub async fn start(&self) -> Result<(), String> {
        let manager = match UserManager::connect().await {
            Ok(manager) => manager,
            Err(error) => {
                self.record_pre_enqueue_refusal(&error)?;
                return Err(error);
            }
        };
        self.mark_maybe_sent()?;
        let intent = self.intent()?;
        let runner = Path::new(required(&intent, "runner")?);
        if execution::hash_file(runner)? != intent["runner_sha256"] {
            return Err(
                "Bound runner hash changed after start intent; outcome needs reconciliation".into(),
            );
        }
        let args = vec![
            "host-run".to_owned(),
            "--state-dir".to_owned(),
            self.state.display().to_string(),
            "--operation-dir".to_owned(),
            self.operation.display().to_string(),
            "--attempt-id".to_owned(),
            self.id.clone(),
        ];
        manager
            .start(&self.unit, runner, &args, &self.directory)
            .await
    }
    pub fn record_claim(&self, runner: &Value, invocation_id: &str) -> Result<(), String> {
        if !hex(invocation_id, 32) {
            return Err("Invalid systemd invocation identity".into());
        }
        let (_project, _op) = self.gate()?;
        self.validate_owner()?;
        if !self.directory.join("start-maybe-sent.json").exists() {
            return Err("Runner has no durable start intent".into());
        }
        marker(
            &self.directory,
            "claim.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"unit":self.unit,"invocation_id":invocation_id,
            "runner":runner,"time":execution::now()}),
        )
    }
    fn verify_host_claim(&self, status: &HostStatus, claim: &Value) -> Result<(), String> {
        if status.name != self.unit
            || status.invocation_id != claim["invocation_id"]
            || status.main_pid != claim["runner"]["pid"]
            || status.exit_type != "cgroup"
            || status.restart != "no"
            || !status.remain_after_exit
            || status.collect_mode != "inactive"
            || status.control_group.is_empty()
            || !execution::alive(&claim["runner"])
        {
            return Err("Host/claim identity mismatch; no permit".into());
        }
        let intent = self.intent()?;
        let pid = status.main_pid;
        let exe = fs::read_link(format!("/proc/{pid}/exe")).map_err(|e| e.to_string())?;
        if exe != Path::new(required(&intent, "runner")?)
            || execution::hash_file(&exe)? != intent["runner_sha256"]
        {
            return Err("Live runner executable differs from bound release".into());
        }
        let cgroup =
            fs::read_to_string(format!("/proc/{pid}/cgroup")).map_err(|e| e.to_string())?;
        if !cgroup
            .lines()
            .any(|line| line.strip_prefix("0::") == Some(status.control_group.as_str()))
        {
            return Err("Live runner is not in the expected operation cgroup".into());
        }
        Ok(())
    }
    pub fn permit(&self, status: &HostStatus, generation: u64) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.validate_owner()?;
        let claim = read(&self.directory.join("claim.json"))?;
        if generation != owner_generation(&self.state)? {
            return Err("Stale supervisor generation".into());
        }
        self.verify_host_claim(status, &claim)?;
        marker(
            &self.directory,
            "host-observation.json",
            &json!({"schema_version":VERSION,
            "unit":status.name,"invocation_id":status.invocation_id,"main_pid":status.main_pid,
            "control_group":status.control_group,"active":status.active,"observed":execution::now()}),
        )?;
        marker(
            &self.directory,
            "permit.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"runner":claim["runner"],"invocation_id":claim["invocation_id"],
            "owner_generation":generation,"time":execution::now()}),
        )
    }
    /// A replacement service can adopt an uncommitted, already claimed host,
    /// after rechecking the same unit and live process. The old permit is not
    /// overwritten; the new generation is an appended authorization record.
    pub fn endorse(&self, status: &HostStatus, generation: u64) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.current()?;
        if self.directory.join("retired.json").exists()
            || self.operation.join("invocation-reservation.json").exists()
        {
            return Err("Committed or retired attempt cannot be re-endorsed".into());
        }
        if owner_generation(&self.state)? != generation {
            return Err("Stale supervisor generation".into());
        }
        let claim = read(&self.directory.join("claim.json"))?;
        self.verify_host_claim(status, &claim)?;
        marker(
            &self.directory,
            &format!("endorsement-{generation}.json"),
            &json!({"schema_version":VERSION,"attempt_id":self.id,
                "owner_generation":generation,"invocation_id":status.invocation_id,
                "runner":claim["runner"],"time":execution::now()}),
        )
    }
    /// Called by the claimed runner immediately before invoking its child.
    /// The owner and operation gate serialize retirement, exit fencing and
    /// one durable invocation reservation. No external I/O runs under them.
    pub fn commit_spawn(&self, runner: &Value) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.validate_owner()?;
        let permit = read(&self.directory.join("permit.json"))?;
        let gate = read(&self.state.join("session/dispatch-gate.json"))?;
        let current = owner_generation(&self.state)?;
        if gate["phase"] != "running" || gate["owner_generation"] != current {
            return Err("Session dispatch is fenced".into());
        }
        if permit["owner_generation"] != current {
            let endorsement = read(&self.directory.join(format!("endorsement-{current}.json")))?;
            if endorsement["runner"] != *runner
                || endorsement["invocation_id"] != permit["invocation_id"]
            {
                return Err("Owner endorsement mismatch".into());
            }
        }
        if permit["runner"] != *runner || self.directory.join("pre-dispatch-refusal.json").exists()
        {
            return Err("Runner not permitted or already refused".into());
        }
        if self.directory.join("cancel.json").exists() {
            return Err("Cancellation fenced dispatch".into());
        }
        if self.operation.join("invocation-reservation.json").exists()
            || self.directory.join("spawn-committed.json").exists()
        {
            return Err("Duplicate operation dispatch refused".into());
        }
        let op = read(&self.operation.join("operation.json"))?;
        let packet = read(&self.operation.join("packet.json"))?;
        if crate::codec::canonical_sha256(&packet).map_err(|e| e.to_string())?
            != op["packet_sha256"]
            || op["packet_sha256"] != self.intent()?["packet_sha256"]
        {
            return Err("Frozen operation input changed; dispatch refused".into());
        }
        if packet["development"]==true && packet["operation_kind"]!="conversation" {
            let project=packet["policy"]["project"].as_str().or_else(||packet["project"].as_str());
            if let Some(project)=project {let session=crate::session_store::Session::open_existing(Path::new(project),&self.state,required(&op,"project_key")?,"")?;
                let current=session.scope_detail(required(&packet,"run_id")?)?.ok_or("Development scope missing")?;
                let work=session.development_record("workflow",required(&packet,"run_id")?)?.ok_or("Development workflow missing")?;
                if current["authorized"]!=true || current["base"]!=packet["scope_base"] || work["requirement_generation"]!=packet["requirement_generation"] || work["status"]!="running" || !(if crate::provider_selection::enabled(&session){crate::provider_selection::packet_authorized(&session,&work,&packet)?}else{crate::development::policy(&session)?==packet["policy"]}) {return Err("Development requirements/policy fenced native dispatch".into())}
            }
        }
        if packet["operation_kind"]=="conversation"&&packet["execution"]["profile"]=="development-profiles-v1"{
            let project=packet["project_context"]["project"].as_str().ok_or("Conversation project context missing")?;
            let session=crate::session_store::Session::open_existing(Path::new(project),&self.state,required(&op,"project_key")?,"")?;
            let allowed:bool=session.workflow.connection()?.query_row("SELECT EXISTS(SELECT 1 FROM conversation_turns t JOIN dialogue_inputs d USING(turn_id) JOIN conversation_bindings b ON b.singleton=1 WHERE t.turn_id=?1 AND t.status='reserved' AND t.packet_sha256=?2 AND t.binding_generation=b.generation AND d.interrupt_command_id IS NULL)",rusqlite::params![packet["turn_id"].as_str(),op["packet_sha256"].as_str()],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !allowed{return Err("Conversation binding/interruption fenced native dispatch".into());}
        }
        let reservation = json!({"schema_version":VERSION,"attempt_id":self.id,
            "operation_id":self.intent()?["operation_id"],"runner":runner,"time":execution::now()});
        marker(&self.operation, "invocation-reservation.json", &reservation)?;
        marker(&self.directory, "spawn-committed.json", &reservation)
    }
    pub fn refuse_before_dispatch(&self, runner: &Value, reason: &str) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.current()?;
        let claim = read(&self.directory.join("claim.json"))?;
        if claim["runner"] != *runner || !execution::alive(runner) {
            return Err("Pre-dispatch refusal requires the verified live runner".into());
        }
        if self.operation.join("invocation-reservation.json").exists()
            || self.directory.join("spawn-committed.json").exists()
        {
            return Err("Invocation may have committed; refusal cannot prove non-launch".into());
        }
        marker(
            &self.directory,
            "pre-dispatch-refusal.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"reason":reason,"time":execution::now()}),
        )
    }
    pub fn retire_proven_no_launch(&self, reason: &str) -> Result<(), String> {
        let (_project, _op) = self.gate()?;
        self.current()?;
        let prelaunch = self.directory.join("prelaunch-refusal.json").exists();
        let predispatch = self.directory.join("pre-dispatch-refusal.json").exists();
        if !(prelaunch || predispatch)
            || self.operation.join("invocation-reservation.json").exists()
            || self.directory.join("spawn-committed.json").exists()
        {
            return Err("No positive proof of non-launch; retirement refused".into());
        }
        marker(
            &self.directory,
            "retired.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"proof":if prelaunch{"pre-enqueue"}else{"runner-pre-dispatch"},
            "reason":reason,"time":execution::now()}),
        )
    }
    /// A receipt is application evidence, not inferred from unit state.
    /// Repeated validation is read-only and can be used after a service crash.
    pub fn validated_receipt(&self) -> Result<Option<Value>, String> {
        let path = self.directory.join("receipt.json");
        if !path.exists() {
            return Ok(None);
        }
        let receipt = read(&path)?;
        let intent = self.intent()?;
        let claim = read(&self.directory.join("claim.json"))?;
        let reservation = read(&self.operation.join("invocation-reservation.json"))?;
        let committed = read(&self.directory.join("spawn-committed.json"))?;
        let op = read(&self.operation.join("operation.json"))?;
        let continuous=read(&self.operation.join("packet.json"))?["execution"]["profile"]=="claude-proposal-v2";
        if receipt["schema_version"] != if continuous{3}else{2}
            || receipt["attempt_id"] != self.id
            || receipt["unit"] != self.unit
            || receipt["operation_id"] != op["operation_id"]
            || receipt["packet_sha256"] != intent["packet_sha256"]
            || receipt["runner"] != claim["runner"]
            || receipt["invocation_id"] != claim["invocation_id"]
            || reservation["attempt_id"] != self.id
            || committed["attempt_id"] != self.id
        {
            return Err("Host receipt identity conflict; execution uncertain".into());
        }
        if continuous {
          let proof=read(&self.directory.join("completion.json"))?;
          if proof["schema_version"]!=1||proof["packet_sha256"]!=intent["packet_sha256"]||proof["attempt_id"]!=self.id||proof["status"]!=receipt["status"]||proof["exit_code"]!=receipt["exit_code"]||crate::codec::canonical_sha256(&proof).map_err(|e|e.to_string())?!=receipt["completion_sha256"] {return Err("Typed completion identity/hash conflict".into())}
          if let Some(hash)=proof["native_events_sha256"].as_str(){if sha256(&fs::read(self.directory.join("native-events.jsonl")).map_err(|e|e.to_string())?)!=hash{return Err("Native events hash conflict".into())}}
          if receipt["status"]=="completed"&&proof["action_valid"]!=true{return Err("Completed receipt lacks validated action envelope".into())}
        }
        match receipt["status"].as_str() {
            Some("completed") => {
                let bytes =
                    fs::read(self.directory.join("result.json")).map_err(|e| e.to_string())?;
                if sha256(&bytes) != receipt["result_sha256"] {
                    return Err("Host result hash mismatch; execution uncertain".into());
                }
                let result: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                let packet = read(&self.operation.join("packet.json"))?;
                crate::output::validate(&result, &packet)?;
            }
            Some("failed" | "cancelled" | "capacity-waiting") if receipt["result_sha256"].is_null() => {}
            Some("output-invalid") if receipt["result_sha256"].is_null() && matches!(read(&self.operation.join("packet.json"))?["execution"]["profile"].as_str(),Some("claude-proposal-v1"|"claude-proposal-v2"))=>{}
            _ => return Err("Invalid host receipt outcome".into()),
        }
        Ok(Some(receipt))
    }
    /// A durable request is made under the same gate as receipt publication.
    /// The runner checks it before commit and while collecting a child. This
    /// method never asserts that an absent process proves cancellation.
    pub fn request_cancel(&self, command_id: &str) -> Result<Value, String> {
        if command_id.is_empty() {
            return Err("Cancellation command ID required".into());
        }
        let (_project, _op) = self.gate()?;
        self.current()?;
        if let Some(receipt) = self.validated_receipt()? {
            return Ok(json!({"status":"completed-race","receipt":receipt}));
        }
        marker(
            &self.directory,
            "cancel.json",
            &json!({"schema_version":VERSION,
            "attempt_id":self.id,"command_id":command_id,"time":execution::now()}),
        )?;
        Ok(json!({"status":"cancellation-requested","attempt_id":self.id}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::canonical_sha256;
    fn fixture() -> (tempfile::TempDir, Attempt, Value) {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state");
        let op = state.join("runs/r/operations/draft");
        fs::create_dir_all(&op).unwrap();
        let runner = std::env::current_exe().unwrap();
        let packet = json!({"schema_version":1,"operation_kind":"coordinator",
            "operation_id":"op-a","run_id":"r","stage":"coordinator","provider":"fake",
            "task_id":"t","task_version":1,"message_sha256":"msg","message":"test"});
        write(&op.join("packet.json"), &packet, true).unwrap();
        write(
            &op.join("operation.json"),
            &json!({"schema_version":2,"operation_id":"op-a",
            "project_key":"a".repeat(32),
            "packet_sha256":canonical_sha256(&packet).unwrap()}),
            true,
        )
        .unwrap();
        write(
            &state.join("supervision/owner.json"),
            &json!({"generation":7}),
            false,
        )
        .unwrap();
        write(
            &state.join("session/dispatch-gate.json"),
            &json!({"phase":"running","owner_generation":7}),
            false,
        )
        .unwrap();
        let attempt = Attempt::create(&state, &op, &"a".repeat(32), &runner, "test", 7).unwrap();
        (
            temp,
            attempt,
            execution::process_identity(std::process::id()).unwrap(),
        )
    }
    fn own_cgroup() -> String {
        fs::read_to_string(format!("/proc/{}/cgroup", std::process::id()))
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("0::").map(str::to_owned))
            .unwrap()
    }
    #[test]
    fn stale_and_unproven_attempts_never_dispatch_or_replace() {
        let (_temp, attempt, runner) = fixture();
        assert!(attempt.retire_proven_no_launch("missing PID").is_err());
        assert!(
            Attempt::create(
                &attempt.state,
                &attempt.operation,
                &"a".repeat(32),
                &std::env::current_exe().unwrap(),
                "test",
                7
            )
            .is_err()
        );
        attempt.mark_maybe_sent().unwrap();
        attempt.record_claim(&runner, &"b".repeat(32)).unwrap();
        let host = HostStatus {
            name: attempt.unit.clone(),
            active: "active".into(),
            sub: "running".into(),
            invocation_id: "b".repeat(32),
            control_group: own_cgroup(),
            main_pid: std::process::id(),
            result: "success".into(),
            restart: "no".into(),
            exit_type: "cgroup".into(),
            remain_after_exit: true,
            collect_mode: "inactive".into(),
        };
        attempt.permit(&host, 7).unwrap();
        write(
            &attempt.state.join("supervision/owner.json"),
            &json!({"generation":8}),
            false,
        )
        .unwrap();
        assert!(attempt.commit_spawn(&runner).is_err());
        write(
            &attempt.state.join("supervision/owner.json"),
            &json!({"generation":7}),
            false,
        )
        .unwrap();
        attempt.commit_spawn(&runner).unwrap();
        assert!(attempt.commit_spawn(&runner).is_err());
        assert!(attempt.refuse_before_dispatch(&runner, "late").is_err());
        assert!(attempt.retire_proven_no_launch("late").is_err());
    }
    #[test]
    fn prelaunch_refusal_retirement_then_linked_attempt_is_one_logical_operation() {
        let (_temp, attempt, _) = fixture();
        attempt.record_pre_enqueue_refusal("bus absent").unwrap();
        attempt.retire_proven_no_launch("bus absent").unwrap();
        assert!(attempt.mark_maybe_sent().is_err());
        let replacement = Attempt::create(
            &attempt.state,
            &attempt.operation,
            &"a".repeat(32),
            &std::env::current_exe().unwrap(),
            "test",
            7,
        )
        .unwrap();
        assert_eq!(
            replacement.intent().unwrap()["previous_attempt"],
            attempt.id
        );
        assert_ne!(replacement.unit, attempt.unit);
        // Simulate a crash after pointer publication, before the recovery
        // record. The next owner derives the immutable link from both intents.
        assert!(!attempt.directory.join("recovery.json").exists());
        replacement.reconcile_recovery_link().unwrap();
        let linked=read(&attempt.directory.join("recovery.json")).unwrap();
        assert_eq!(linked["replacement_attempt"],replacement.id);
        replacement.reconcile_recovery_link().unwrap();
        assert_eq!(read(&attempt.directory.join("recovery.json")).unwrap(),linked);
        assert!(
            !attempt
                .operation
                .join("invocation-reservation.json")
                .exists()
        );
    }
    #[test]
    fn claimed_runner_requires_new_owner_endorsement_after_restart() {
        let (_temp, attempt, runner) = fixture();
        attempt.mark_maybe_sent().unwrap();
        attempt.record_claim(&runner, &"b".repeat(32)).unwrap();
        let host = HostStatus {
            name: attempt.unit.clone(),
            active: "active".into(),
            sub: "running".into(),
            invocation_id: "b".repeat(32),
            control_group: own_cgroup(),
            main_pid: std::process::id(),
            result: "success".into(),
            restart: "no".into(),
            exit_type: "cgroup".into(),
            remain_after_exit: true,
            collect_mode: "inactive".into(),
        };
        attempt.permit(&host, 7).unwrap();
        attempt.permit(&host, 7).unwrap(); // lost permit reply is idempotent
        write(
            &attempt.state.join("supervision/owner.json"),
            &json!({"generation":8}),
            false,
        )
        .unwrap();
        write(
            &attempt.state.join("session/dispatch-gate.json"),
            &json!({"phase":"running","owner_generation":8}),
            false,
        )
        .unwrap();
        assert!(attempt.commit_spawn(&runner).is_err());
        attempt.endorse(&host, 8).unwrap();
        attempt.commit_spawn(&runner).unwrap();
        assert!(attempt.commit_spawn(&runner).is_err());
    }
    #[test]
    fn live_runner_refusal_blocks_late_permit_and_allows_only_linked_replacement() {
        let (_temp, attempt, runner) = fixture();
        attempt.mark_maybe_sent().unwrap();
        attempt.record_claim(&runner, &"b".repeat(32)).unwrap();
        attempt
            .refuse_before_dispatch(&runner, "permit deadline")
            .unwrap();
        attempt
            .retire_proven_no_launch("runner refused before dispatch")
            .unwrap();
        assert!(attempt.commit_spawn(&runner).is_err());
        let next = Attempt::create(
            &attempt.state,
            &attempt.operation,
            &"a".repeat(32),
            &std::env::current_exe().unwrap(),
            "test",
            7,
        )
        .unwrap();
        assert_ne!(next.unit, attempt.unit);
    }
    #[test]
    fn cancellation_before_commit_is_no_launch_and_not_a_budget_spend() {
        let (_temp, attempt, runner) = fixture();
        attempt.mark_maybe_sent().unwrap();
        attempt.record_claim(&runner, &"b".repeat(32)).unwrap();
        assert_eq!(
            attempt.request_cancel("cancel-one").unwrap()["status"],
            "cancellation-requested"
        );
        assert!(attempt.commit_spawn(&runner).is_err());
        attempt
            .refuse_before_dispatch(&runner, "cancelled before invocation")
            .unwrap();
        assert!(
            !attempt
                .operation
                .join("invocation-reservation.json")
                .exists()
        );
    }
    #[test]
    fn replacement_owner_retires_only_a_never_enqueued_attempt(){
        let (_temp,attempt,_runner)=fixture();
        write(&attempt.state.join("supervision/owner.json"),&json!({"generation":8}),false).unwrap();
        attempt.retire_fenced_unsent_owner(8).unwrap();
        assert!(attempt.directory.join("retired.json").exists());
        let replacement=Attempt::create(&attempt.state,&attempt.operation,&"a".repeat(32),
            &std::env::current_exe().unwrap(),"test",8).unwrap();
        assert_eq!(replacement.intent().unwrap()["previous_attempt"],attempt.id);
        replacement.mark_maybe_sent().unwrap();
        write(&attempt.state.join("supervision/owner.json"),&json!({"generation":9}),false).unwrap();
        assert!(replacement.retire_fenced_unsent_owner(9).is_err());
    }
    #[test]
    fn saved_receipt_is_reusable_and_late_cancel_cannot_override_it() {
        let (_temp, attempt, runner) = fixture();
        attempt.mark_maybe_sent().unwrap();
        attempt.record_claim(&runner, &"b".repeat(32)).unwrap();
        let host = HostStatus {
            name: attempt.unit.clone(),
            active: "active".into(),
            sub: "running".into(),
            invocation_id: "b".repeat(32),
            control_group: own_cgroup(),
            main_pid: std::process::id(),
            result: "success".into(),
            restart: "no".into(),
            exit_type: "cgroup".into(),
            remain_after_exit: true,
            collect_mode: "inactive".into(),
        };
        attempt.permit(&host, 7).unwrap();
        attempt.commit_spawn(&runner).unwrap();
        let packet = read(&attempt.operation.join("packet.json")).unwrap();
        let result = crate::fake::result(&packet).unwrap();
        write(&attempt.directory.join("result.json"), &result, true).unwrap();
        write(
            &attempt.directory.join("receipt.json"),
            &json!({"schema_version":2,
            "attempt_id":attempt.id,"unit":attempt.unit,"operation_id":"op-a",
            "packet_sha256":canonical_sha256(&packet).unwrap(),"runner":runner,
            "invocation_id":"b".repeat(32),"status":"completed",
            "result_sha256":canonical_sha256(&result).unwrap()}),
            true,
        )
        .unwrap();
        assert_eq!(
            attempt.validated_receipt().unwrap().unwrap()["status"],
            "completed"
        );
        assert_eq!(
            attempt.request_cancel("too-late").unwrap()["status"],
            "completed-race"
        );
        assert!(!attempt.directory.join("cancel.json").exists());
    }
}
