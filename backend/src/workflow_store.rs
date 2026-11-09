//! Schema-2 application workflow authority. SQLite revisions and ledgers decide
//! transitions; immutable operation receipts and artifacts remain evidence.
//! No LangGraph checkpoint is decoded or written here.
use crate::{
    codec::{canonical, canonical_sha256, sha256},
    execution::{read, write, write_bytes},
    db_writer::DbWriter, findings, operation, output, packet, workflow_policy,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},sync::Arc,
};

pub struct Store {
    pub state: PathBuf,
    pub(crate) writer:Option<Arc<DbWriter>>,
}
fn encoded(value: &Value) -> Result<String, String> {
    String::from_utf8(canonical(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn decoded(raw: String) -> Result<Value, String> {
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}
fn id(value: &str) -> Result<&str, String> {
    if value.len() > 64
        || value.is_empty()
        || !value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Use a short alphanumeric ID with hyphens/underscores".into());
    }
    Ok(value)
}
fn tx_read(tx: &Transaction<'_>, run: &str) -> Result<(u64, Value, Value, String), String> {
    let row: (i64, String, String, String) = tx
        .query_row(
            "SELECT revision,state_json,envelope_json,status FROM ww_workflows WHERE run_id=?1",
            [run],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;
    Ok((row.0 as u64, decoded(row.1)?, decoded(row.2)?, row.3))
}
fn tx_update(
    tx: &Transaction<'_>,
    run: &str,
    revision: u64,
    state: &Value,
    status: &str,
    detail: Option<&str>,
) -> Result<(), String> {
    let changed=tx.execute("UPDATE ww_workflows SET revision=?1,state_json=?2,status=?3 WHERE run_id=?4 AND revision=?5",
        params![(revision+1) as i64,encoded(state)?,status,run,revision as i64]).map_err(|e|e.to_string())?;
    if changed != 1 {
        return Err("Stale workflow revision; re-read before acting".into());
    }
    let active = if matches!(status, "approved" | "rejected" | "failed" | "cancelled") {
        0
    } else {
        1
    };
    tx.execute(
        "UPDATE app_runs SET status=?1,active=?2,detail=?3 WHERE run_id=?4",
        params![status, active, detail, run],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
impl Store {
    pub fn open(state: &Path) -> Result<Self, String> {
        fs::create_dir_all(state).map_err(|e| e.to_string())?;
        let state = state.canonicalize().map_err(|e| e.to_string())?;
        let contract=state.join("state-contract.json");
        let expected=if contract.exists(){
            let record=read(&contract)?;
            if record["schema_version"]!=1 || record["execution_host"]!="systemd-user-v1" ||
                !matches!(record["state_schema"].as_u64(),Some(3|4|5|6|7|8|9)){
                return Err("Unknown project execution contract; no execution".into());
            }
            record["state_schema"].as_i64().ok_or("Invalid state schema")?
        }else{2};
        let db = state.join("state.sqlite");
        if db.exists() {
            let conn = Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| e.to_string())?;
            let migration: Option<i64> = conn
                .query_row(
                    "SELECT version FROM ww_schema_migrations ORDER BY version DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| {
                    "Existing state is not schema 2; legacy project must stay on bound release"
                        .to_owned()
                })?;
            if migration != Some(expected) {
                return Err("Unknown application state schema; no execution".into());
            }
        } else {
            let conn = Connection::open(&db).map_err(|e| e.to_string())?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
                CREATE TABLE ww_schema_migrations(version INTEGER PRIMARY KEY);
                INSERT INTO ww_schema_migrations VALUES(2);
                CREATE TABLE ww_workflows(run_id TEXT PRIMARY KEY,engine_kind TEXT NOT NULL,
                    schema_version INTEGER NOT NULL,definition_hash TEXT NOT NULL,
                    revision INTEGER NOT NULL,envelope_json TEXT NOT NULL,state_json TEXT NOT NULL,status TEXT NOT NULL);
                CREATE TABLE ww_effects(run_id TEXT NOT NULL,stage TEXT NOT NULL,operation_id TEXT NOT NULL UNIQUE,
                    packet_sha256 TEXT NOT NULL,applied_revision INTEGER,PRIMARY KEY(run_id,stage));
                CREATE TABLE app_runs(run_id TEXT PRIMARY KEY,envelope TEXT NOT NULL,status TEXT NOT NULL,
                    active INTEGER NOT NULL,detail TEXT);
                CREATE UNIQUE INDEX app_single_active ON app_runs(active) WHERE active=1;
                CREATE TABLE app_operations(run_id TEXT NOT NULL,stage TEXT NOT NULL,
                    operation_id TEXT NOT NULL UNIQUE,packet_hash TEXT NOT NULL,PRIMARY KEY(run_id,stage));
                CREATE TABLE app_requests(run_id TEXT NOT NULL,request_id TEXT NOT NULL,payload TEXT NOT NULL,
                    PRIMARY KEY(run_id,request_id));
            CREATE TABLE app_responses(run_id TEXT NOT NULL,request_id TEXT NOT NULL,payload TEXT NOT NULL,
                    PRIMARY KEY(run_id,request_id));").map_err(|e|e.to_string())?;
            if expected>=3{
                conn.execute_batch("UPDATE ww_schema_migrations SET version=3 WHERE version=2;
                    CREATE TABLE app_host_attempts(operation_id TEXT NOT NULL,attempt_id TEXT PRIMARY KEY,
                        unit_name TEXT NOT NULL UNIQUE,record_sha256 TEXT NOT NULL);
                    CREATE TABLE app_host_recoveries(recovery_id TEXT PRIMARY KEY,operation_id TEXT NOT NULL,
                        source_attempt TEXT NOT NULL,replacement_attempt TEXT NOT NULL,record_sha256 TEXT NOT NULL);
                    CREATE TABLE app_output_indexes(attempt_id TEXT PRIMARY KEY,index_version INTEGER NOT NULL,
                        high_water INTEGER NOT NULL);").map_err(|e|e.to_string())?;
                if expected>=4{
                    conn.execute("UPDATE ww_schema_migrations SET version=?1 WHERE version=3",[expected])
                        .map_err(|e|e.to_string())?;
                }
            }
        }
        Ok(Self { state, writer:None })
    }
    pub(crate) fn connection(&self) -> Result<Connection, String> {
        let conn = Connection::open(self.state.join("state.sqlite")).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_millis(250))
            .map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|e| e.to_string())?;
        Ok(conn)
    }
    pub fn run_dir(&self, run: &str) -> Result<PathBuf, String> {
        Ok(self.state.join("runs").join(id(run)?))
    }
    pub fn snapshot(&self, run: &str) -> Result<Value, String> {
        let conn = self.connection()?;
        let (version,definition,revision,raw,status):(i64,String,i64,String,String)=conn.query_row(
            "SELECT schema_version,definition_hash,revision,state_json,status FROM ww_workflows WHERE run_id=?1",
            [id(run)?],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))
            .map_err(|e|e.to_string())?;
        if version != 2 || definition != workflow_policy::definition_hash() {
            return Err("Unknown state/definition version; no execution".into());
        }
        Ok(json!({"revision":revision,"status":status,"state":decoded(raw)?}))
    }
    pub fn create(&self, envelope: &Value) -> Result<Value, String> {
        if read(&self.state.join("state-contract.json")).ok().is_some_and(|v|matches!(v["state_schema"].as_u64(),Some(5|6|7))){return Err("Worker execution is unavailable in the conversation-only increment".into());}
        if let Some(writer)=&self.writer {let envelope=envelope.clone();return writer.call(move|s|s.workflow.create(&envelope));}
        let run = id(envelope["run_id"].as_str().ok_or("missing run ID")?)?;
        if envelope["workflow_version"] == 2 {
            crate::context_policy::validate_snapshot(&envelope["context_snapshot"])?;
            if canonical_sha256(&envelope["context_snapshot"]).map_err(|e| e.to_string())?
                != envelope["context_digest"]
            {
                return Err("Frozen context digest mismatch".into());
            }
        }
        let state = workflow_policy::initial(run, envelope)?;
        // An orphan immutable input is reconcilable; a committed run without
        // its frozen input file is not a safe dispatch boundary.
        write(
            &self.run_dir(run)?.join("inputs/envelope.json"),
            envelope,
            true,
        )?;
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO app_runs VALUES(?1,?2,'running',1,NULL)",
            params![run, encoded(envelope)?],
        )
        .map_err(|e| format!("Run already exists or another run is active: {e}"))?;
        tx.execute(
            "INSERT INTO ww_workflows VALUES(?1,'rust',2,?2,0,?3,?4,'running')",
            params![
                run,
                workflow_policy::definition_hash(),
                encoded(envelope)?,
                encoded(&state)?
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(state)
    }
    fn envelope(&self, run: &str) -> Result<Value, String> {
        let conn = self.connection()?;
        let raw: String = conn
            .query_row(
                "SELECT envelope_json FROM ww_workflows WHERE run_id=?1",
                [run],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        decoded(raw)
    }
    pub fn effective_target(&self,run:&str)->Result<Value,String>{
        let envelope=self.envelope(run)?;
        let conn=self.connection()?;
        let route_table:Option<i64>=conn.query_row("SELECT 1 FROM sqlite_master WHERE type='table' AND name='ww_route_bindings'",
            [],|row|row.get(0)).optional().map_err(|e|e.to_string())?;
        if route_table.is_none(){return Ok(envelope["target"].clone());}
        let latest:Option<(String,String)>=conn.query_row("SELECT binding_json,binding_sha256 FROM ww_route_bindings WHERE run_id=?1 ORDER BY seq DESC LIMIT 1",
            [id(run)?],|row|Ok((row.get(0)?,row.get(1)?))).optional().map_err(|e|e.to_string())?;
        if let Some((raw,hash))=latest{
            let binding=decoded(raw)?;
            if canonical_sha256(&binding).map_err(|e|e.to_string())?!=hash || binding["schema_version"]!=1 ||
                binding["run_id"]!=run || binding["original_target"]!=envelope["target"]{
                return Err("Run routing binding integrity mismatch; no execution".into());
            }
            Ok(binding["target"].clone())
        }else{Ok(envelope["target"].clone())}
    }
    pub fn append_routing(&self,run:&str,target:&Value,reason:&str,generation:u64)->Result<Value,String>{
        if let Some(writer)=&self.writer {let (run,target,reason)=(run.to_owned(),target.clone(),reason.to_owned());
            return writer.call(move|s|s.workflow.append_routing(&run,&target,&reason,generation));}
        if target["socket"].as_str().is_none_or(str::is_empty) || target["workspace"].as_str().is_none_or(str::is_empty){
            return Err("Routing target requires explicit socket and workspace".into());
        }
        let envelope=self.envelope(run)?;
        let snapshot=self.snapshot(run)?;
        if !["running","awaiting-input"].iter().any(|s|snapshot["status"]==*s){
            return Err("Stopped or completed run cannot be rerouted".into());
        }
        let stage=snapshot["state"]["next_stage"].as_str().ok_or("invalid workflow stage")?;
        let mut conn=self.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if snapshot["status"]=="awaiting-input" {
            let request=snapshot["state"]["pending"]["request_id"].as_str()
                .ok_or("Pending gate lacks request ID")?;
            let answered:Option<i64>=tx.query_row("SELECT 1 FROM app_responses WHERE run_id=?1 AND request_id=?2",
                params![run,request],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            if answered.is_some(){return Err("Human gate already has a recorded answer; rerouting refused".into());}
        }
        tx.execute_batch("CREATE TABLE IF NOT EXISTS ww_route_bindings(
            run_id TEXT NOT NULL,seq INTEGER NOT NULL,schema_version INTEGER NOT NULL,
            binding_json TEXT NOT NULL,binding_sha256 TEXT NOT NULL,PRIMARY KEY(run_id,seq));")
            .map_err(|e|e.to_string())?;
        let effect:i64=tx.query_row("SELECT count(*) FROM ww_effects WHERE run_id=?1 AND stage=?2",
            params![run,stage],|r|r.get(0)).map_err(|e|e.to_string())?;
        if effect!=0{return Err("Current stage already has frozen execution identity; rerouting requires explicit recovery".into());}
        let last:Option<(i64,String)>=tx.query_row("SELECT seq,binding_json FROM ww_route_bindings WHERE run_id=?1 ORDER BY seq DESC LIMIT 1",
            [run],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        let current=last.as_ref().map(|(_,raw)|decoded(raw.clone())).transpose()?.map(|b|b["target"].clone())
            .unwrap_or_else(||envelope["target"].clone());
        if current==*target{return Ok(json!({"status":"already-bound","target":target}));}
        let seq=last.as_ref().map(|(seq,_)|seq+1).unwrap_or(1);
        let binding=json!({"schema_version":1,"run_id":run,"seq":seq,"original_target":envelope["target"],
            "previous_target":current,"target":target,"reason":reason,"owner_generation":generation});
        let hash=canonical_sha256(&binding).map_err(|e|e.to_string())?;
        write(&self.run_dir(run)?.join("routing/bindings").join(format!("{seq:06}.json")),&binding,true)?;
        tx.execute("INSERT INTO ww_route_bindings VALUES(?1,?2,1,?3,?4)",params![run,seq,encoded(&binding)?,hash])
            .map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(json!({"status":"bound","binding":binding,"binding_sha256":hash}))
    }
    fn artifact_text(&self, run: &str, reference: &Value) -> Result<String, String> {
        if reference.is_null() {
            return Ok(String::new());
        }
        let root = self
            .run_dir(run)?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let relative = reference["path"].as_str().ok_or("invalid artifact path")?;
        let path = root
            .join(relative)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("Artifact path leaves selected run".into());
        }
        let raw = fs::read(path).map_err(|e| e.to_string())?;
        if sha256(&raw) != reference["sha256"] {
            return Err("Artifact path/hash validation failed".into());
        }
        String::from_utf8(raw).map_err(|e| e.to_string())
    }
    pub fn reserve_stage(&self, run: &str) -> Result<Value, String> {
        if let Some(writer)=&self.writer {let run=run.to_owned();return writer.call(move|s|s.workflow.reserve_stage(&run));}
        let run = id(run)?;
        let snapshot = self.snapshot(run)?;
        if snapshot["status"] != "running" {
            return Err("Run is not dispatchable".into());
        }
        let state = &snapshot["state"];
        if !state["pending"].is_null()
            || state["questions"].as_array().is_some_and(|a| !a.is_empty())
        {
            return Err("Pending human question cannot dispatch".into());
        }
        let stage = state["next_stage"].as_str().ok_or("missing next stage")?;
        if !workflow_policy::STAGES.contains(&stage) {
            return Err("Third critique or invalid stage refused".into());
        }
        let envelope = self.envelope(run)?;
        let current = self.artifact_text(run, &state["current"])?;
        let previous = self.artifact_text(run, &state["last_reviewed"])?;
        let packet = packet::build(&envelope, state, stage, &current, &previous)?;
        let hash = canonical_sha256(&packet).map_err(|e| e.to_string())?;
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let (revision, active, _, status) = tx_read(&tx, run)?;
        if revision != snapshot["revision"].as_u64().unwrap_or(u64::MAX)
            || active["next_stage"] != stage
            || status != "running"
        {
            return Err("Stale workflow reservation; re-read before dispatch".into());
        }
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT operation_id,packet_sha256 FROM ww_effects WHERE run_id=?1 AND stage=?2",
                params![run, stage],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some((operation, digest)) = existing {
            if operation != packet["operation_id"] || digest != hash {
                return Err("Stage already reserved with different input; no redispatch".into());
            }
        } else {
            let used: i64 = tx
                .query_row(
                    "SELECT count(*) FROM ww_effects WHERE run_id=?1",
                    [run],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let limit = envelope["config"]["max_invocations"].as_i64().unwrap_or(5);
            if used >= limit || used >= 5 {
                return Err("Frozen stage invocation limit reached".into());
            }
            if stage.starts_with("critique") {
                let reviews:i64=tx.query_row("SELECT count(*) FROM ww_effects WHERE run_id=?1 AND stage LIKE 'critique-%'",[run],|r|r.get(0)).map_err(|e|e.to_string())?;
                if reviews >= 2 {
                    return Err("Two-critique limit reached".into());
                }
            }
            tx.execute(
                "INSERT INTO ww_effects VALUES(?1,?2,?3,?4,NULL)",
                params![run, stage, packet["operation_id"].as_str(), hash],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO app_operations VALUES(?1,?2,?3,?4)",
                params![run, stage, packet["operation_id"].as_str(), hash],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        let directory = self.run_dir(run)?.join("operations").join(stage);
        let target = self.effective_target(run)?;
        if target["host"]=="systemd-user-v1"{
            operation::prepare_host(&self.state,&directory,&packet,
                envelope["project_key"].as_str().ok_or("missing project key")?)
                .map_err(|e|e.to_string())?;
        }else{
            operation::prepare(&self.state,&directory,&packet,&target,
                envelope["project_key"].as_str()).map_err(|e| e.to_string())?;
        }
        Ok(json!({"stage":stage,"packet":packet,"directory":directory,"reserved":true}))
    }
    fn publish(
        &self,
        run: &str,
        stage: &str,
        packet: &Value,
        result: &Value,
        state: &Value,
        classified: &Value,
    ) -> Result<Value, String> {
        let run_dir = self.run_dir(run)?;
        let text = result["content"]
            .as_str()
            .ok_or("missing artifact content")?;
        let path = run_dir.join("artifacts").join(format!("{stage}.md"));
        write_bytes(&path, text.as_bytes(), true)?;
        let parents = [&state["current"], &state["last_reviewed"]]
            .iter()
            .filter(|r| !r.is_null())
            .map(|r| r["id"].clone())
            .collect::<Vec<_>>();
        let reference = json!({"schema_version":1,"id":stage,"kind":stage.split('-').next().unwrap_or(stage),
            "path":format!("artifacts/{stage}.md"),"sha256":sha256(text.as_bytes()),
            "packet_sha256":canonical_sha256(packet).map_err(|e|e.to_string())?,
            "operation_id":result["operation_id"],"parents":parents});
        write(
            &run_dir.join("artifacts").join(format!("{stage}.json")),
            &json!({"manifest":reference,"result":result}),
            true,
        )?;
        let ledger = json!({"schema_version":1,"stage":stage,"result_sha256":classified["result_sha256"],
            "new_findings":classified["new"],"assessments":classified["assessments"],
            "missing_assessments":classified["missing"]});
        write(
            &run_dir
                .join("quality")
                .join(format!("{stage}-finding-ledger.json")),
            &ledger,
            true,
        )?;
        Ok(reference)
    }
    pub fn apply_saved(&self, run: &str, stage: &str) -> Result<Value, String> {
        if let Some(writer)=&self.writer {let (run,stage)=(run.to_owned(),stage.to_owned());
            return writer.call(move|s|s.workflow.apply_saved(&run,&stage));}
        self.apply_saved_inner(run, stage, || Ok(()))
    }
    fn apply_saved_inner(
        &self,
        run: &str,
        stage: &str,
        before_state_commit: impl FnOnce() -> Result<(), String>,
    ) -> Result<Value, String> {
        let run = id(run)?;
        let snapshot = self.snapshot(run)?;
        let state = &snapshot["state"];
        if state["completed"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == stage))
        {
            return Ok(snapshot);
        }
        if state["next_stage"] != stage {
            return Err("Stage is not current".into());
        }
        let directory = self.run_dir(run)?.join("operations").join(stage);
        let receipt = operation::receipt(&directory).map_err(|e| {
            let status = if e.kind == operation::ErrorKind::OutputInvalid {
                "output-invalid"
            } else {
                "uncertain"
            };
            let _ = self.stop(run, status, &e.detail);
            format!("{:?}: {}", e.kind, e.detail)
        })?;
        let Some(receipt) = receipt else {
            let inspection = operation::inspect(&directory, None);
            if inspection["status"] == "uncertain" {
                self.stop(
                    run,
                    "uncertain",
                    inspection["detail"]
                        .as_str()
                        .unwrap_or("Unresolved execution"),
                )?;
            }
            return Err(format!(
                "No validated completion receipt: {}",
                inspection["status"]
            ));
        };
        if receipt["status"] != "completed" {
            let status = if receipt["error"]
                .as_str()
                .unwrap_or("")
                .starts_with("Output invalid:")
            {
                "output-invalid"
            } else {
                receipt["status"].as_str().unwrap_or("uncertain")
            };
            self.stop(
                run,
                status,
                receipt["error"]
                    .as_str()
                    .unwrap_or("Worker did not complete"),
            )?;
            return Err(format!("Worker {status}; no automatic retry"));
        }
        let packet = read(&directory.join("packet.json"))?;
        let result = read(&operation::result_path(&directory).map_err(|e|e.to_string())?)?;
        output::validate(&result, &packet).map_err(|e| format!("Saved output invalid: {e}"))?;
        let mut classified = findings::classify(stage, &state["findings"], &packet, &result, None)
            .map_err(|e| {
                let _ = self.stop(run, "output-invalid", &e);
                format!("Output invalid: {e}")
            })?;
        if let Some(order) = state["finding_order"].as_array() {
            let missing = classified["missing"]
                .as_array()
                .ok_or("invalid missing findings")?;
            let reordered = order
                .iter()
                .filter_map(|id| missing.iter().find(|item| item["id"] == *id))
                .cloned()
                .collect::<Vec<_>>();
            if reordered.len() != missing.len() {
                return Err("Finding order omits unresolved evidence".into());
            }
            classified["missing"] = json!(reordered);
        }
        let reference = self.publish(run, stage, &packet, &result, state, &classified)?;
        let envelope = self.envelope(run)?;
        let next = workflow_policy::after_result(
            state,
            &envelope,
            stage,
            &packet,
            &result,
            &reference,
            &classified,
        )?;
        before_state_commit()?;
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let (revision, active, _, status) = tx_read(&tx, run)?;
        if active["completed"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v == stage))
        {
            return self.snapshot(run);
        }
        if revision != snapshot["revision"].as_u64().unwrap_or(u64::MAX) || status != "running" {
            return Err("Workflow revision changed during result publication".into());
        }
        let changed=tx.execute("UPDATE ww_effects SET applied_revision=?1 WHERE run_id=?2 AND stage=?3 AND applied_revision IS NULL AND packet_sha256=?4",
            params![(revision+1) as i64,run,stage,canonical_sha256(&packet).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        if changed != 1 {
            return Err("Result effect was not uniquely reserved".into());
        }
        let new_status = if !next["blocked_reason"].as_str().unwrap_or("").is_empty() {
            "blocked"
        } else {
            "running"
        };
        tx_update(
            &tx,
            run,
            revision,
            &next,
            new_status,
            next["blocked_reason"].as_str().filter(|s| !s.is_empty()),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        self.snapshot(run)
    }
    pub fn stop(&self, run: &str, status: &str, reason: &str) -> Result<(), String> {
        if let Some(writer)=&self.writer {let (run,status,reason)=(run.to_owned(),status.to_owned(),reason.to_owned());
            return writer.call(move|s|s.workflow.stop(&run,&status,&reason));}
        if !matches!(
            status,
            "uncertain" | "failed" | "cancelled" | "output-invalid" | "blocked"
        ) {
            return Err("Unsupported stopped status".into());
        }
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let (revision, state, _, old) = tx_read(&tx, run)?;
        if matches!(old.as_str(), "approved" | "rejected") {
            return Err("Completed human approval cannot be replaced by a stopped state".into());
        }
        if old != status {
            tx_update(&tx, run, revision, &state, status, Some(reason))?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
    /// Reopen only an infrastructure stop whose operation was proven never
    /// dispatched. The service verifies and records that proof before calling.
    pub fn reopen_prelaunch(&self, run:&str)->Result<(),String>{
        if let Some(writer)=&self.writer {let run=run.to_owned();
            return writer.call(move|s|s.workflow.reopen_prelaunch(&run));}
        let mut conn=self.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e|e.to_string())?;
        let (revision,state,_,status)=tx_read(&tx,run)?;
        if status!="blocked"{return Err("Only a blocked prelaunch workflow can reopen".into());}
        tx_update(&tx,run,revision,&state,"running",Some("Explicit proven-no-launch infrastructure retry"))?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn prepare_gate(&self, run: &str) -> Result<Value, String> {
        if let Some(writer)=&self.writer {let run=run.to_owned();return writer.call(move|s|s.workflow.prepare_gate(&run));}
        let snapshot = self.snapshot(run)?;
        let state = &snapshot["state"];
        if !state["pending"].is_null() {
            return Ok(state["pending"].clone());
        }
        let Some(request) = workflow_policy::request(state)? else {
            return Err("No human gate is due".into());
        };
        self.artifact_text(run, &state["current"])?;
        let mut next = state.clone();
        next["pending"] = request.clone();
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let (revision, active, _, status) = tx_read(&tx, run)?;
        if revision != snapshot["revision"].as_u64().unwrap_or(u64::MAX)
            || active["pending"] != Value::Null
            || status != "running"
        {
            return Err("Stale human gate; re-read before acting".into());
        }
        let request_id = request["request_id"].as_str().ok_or("missing request ID")?;
        let raw = encoded(&request)?;
        let prior: Option<String> = tx
            .query_row(
                "SELECT payload FROM app_requests WHERE run_id=?1 AND request_id=?2",
                params![run, request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if prior.as_ref().is_some_and(|old| old != &raw) {
            return Err("Request changed under an existing ID".into());
        }
        tx.execute(
            "INSERT OR IGNORE INTO app_requests VALUES(?1,?2,?3)",
            params![run, request_id, raw],
        )
        .map_err(|e| e.to_string())?;
        tx_update(&tx, run, revision, &next, "awaiting-input", None)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(request)
    }
    pub fn record_answer(
        &self,
        run: &str,
        request_id: &str,
        outcome: &str,
        answer: &str,
        artifact_hash: Option<&str>,
    ) -> Result<Value, String> {
        self.record_answer_mode(run,request_id,outcome,answer,artifact_hash,false)
    }
    pub fn record_answer_mode(
        &self,run:&str,request_id:&str,outcome:&str,answer:&str,
        artifact_hash:Option<&str>,record_only:bool,
    )->Result<Value,String>{
        if let Some(writer)=&self.writer {let (run,request_id,outcome,answer,artifact_hash)=
            (run.to_owned(),request_id.to_owned(),outcome.to_owned(),answer.to_owned(),artifact_hash.map(str::to_owned));
            return writer.call(move|s|s.workflow.record_answer_mode(&run,&request_id,&outcome,&answer,artifact_hash.as_deref(),record_only));}
        let conn = self.connection()?;
        let previous: Option<String> = conn
            .query_row(
                "SELECT payload FROM app_responses WHERE run_id=?1 AND request_id=?2",
                params![run, request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(previous) = previous {
            let previous_value = decoded(previous.clone())?;
            if previous_value["answer"] != answer
                || previous_value["outcome"] != outcome
                || artifact_hash.is_some_and(|hash| previous_value["artifact_sha256"] != hash)
                || previous_value["record_only"].as_bool().unwrap_or(false) != record_only
            {
                return Err("Conflicting response; accepted decisions are immutable".into());
            }
            self.export_response(run, &previous_value, &previous)?;
            return Ok(previous_value);
        }
        let snapshot = self.snapshot(run)?;
        let state = &snapshot["state"];
        let pending = &state["pending"];
        if pending["request_id"] != request_id {
            return Err("Request is not the current pending human gate".into());
        }
        self.artifact_text(run, &state["current"])?;
        if pending["decision_revision"] != state["decision_revision"] {
            return Err("Stale decision revision".into());
        }
        if pending["kind"] == "choice" {
            if outcome != "answered"
                || !pending["question"]["options"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v == answer))
            {
                return Err("Choose one of the documented substantive options".into());
            }
        } else if pending["kind"] == "final-review" {
            if !matches!(outcome, "approved" | "rejected") {
                return Err("Final review requires approval or rejection".into());
            }
            if outcome == "approved" && artifact_hash != pending["artifact_sha256"].as_str() {
                return Err("Stale or missing artifact SHA-256".into());
            }
        } else {
            return Err("Unknown human gate".into());
        }
        if pending.get("context_digest").is_some()
            && pending["context_digest"] != state["context_digest"]
        {
            return Err("Stale approval/answer context".into());
        }
        let mut response = json!({"request_id":request_id,"answer":answer,"outcome":outcome,
            "artifact_sha256":pending["artifact_sha256"],"decision_revision":pending["decision_revision"],
            "record_only":record_only,
            "recorded_by":std::env::var("USER").unwrap_or_else(|_|"local-operator".into()),
            "time":crate::execution::now(),"schema_version":1});
        if pending.get("context_digest").is_some() {
            response["context_digest"] = state["context_digest"].clone();
            response["supersedes"] = pending["question"]["related_decision_refs"]
                .as_array()
                .cloned()
                .map(|a| json!(a))
                .unwrap_or_else(|| json!([]));
        }
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let (revision, active, _, status) = tx_read(&tx, run)?;
        if revision != snapshot["revision"].as_u64().unwrap_or(u64::MAX)
            || active["pending"] != *pending
            || status != "awaiting-input"
        {
            return Err("Stale gate revision; re-read before answering".into());
        }
        let old: Option<String> = tx
            .query_row(
                "SELECT payload FROM app_responses WHERE run_id=?1 AND request_id=?2",
                params![run, request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let raw=if let Some(old_raw) = old {
            let old = decoded(old_raw.clone())?;
            for key in [
                "request_id",
                "answer",
                "outcome",
                "artifact_sha256",
                "decision_revision",
                "context_digest",
                "supersedes",
            ] {
                if old.get(key) != response.get(key) {
                    return Err("Conflicting response; accepted decisions are immutable".into());
                }
            }
            if old["record_only"].as_bool().unwrap_or(false)!=record_only{
                return Err("Conflicting response delivery mode".into());
            }
            response = old;
            old_raw
        } else {
            let raw=encoded(&response)?;
            tx.execute(
                "INSERT INTO app_responses VALUES(?1,?2,?3)",
                params![run, request_id, &raw],
            )
            .map_err(|e| e.to_string())?;
            raw
        };
        tx.commit().map_err(|e| e.to_string())?;
        self.export_response(run, &response, &raw)?;
        Ok(response)
    }
    fn export_response(&self, run: &str, response: &Value, raw: &str) -> Result<(), String> {
        let id = response["request_id"]
            .as_str()
            .ok_or("missing request ID")?;
        let base = self
            .run_dir(run)?
            .join("artifacts")
            .join(format!("decision-{}", self::id(id)?));
        write_bytes(&base.with_extension("json"), raw.as_bytes(), true)?;
        let markdown = format!(
            "# {id}\n\nOutcome: {}\n\n{}\n\nReviewed SHA-256: {}\n",
            response["outcome"].as_str().unwrap_or(""),
            response["answer"].as_str().unwrap_or(""),
            response["artifact_sha256"].as_str().unwrap_or("")
        );
        write_bytes(&base.with_extension("md"), markdown.as_bytes(), true)
    }
    pub fn release_record_only(&self,run:&str,request_id:&str)->Result<Value,String>{
        if let Some(writer)=&self.writer {let (run,request_id)=(run.to_owned(),request_id.to_owned());
            return writer.call(move|s|s.workflow.release_record_only(&run,&request_id));}
        let conn=self.connection()?;
        let raw:String=conn.query_row("SELECT payload FROM app_responses WHERE run_id=?1 AND request_id=?2",
            params![run,request_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        let response=decoded(raw.clone())?;
        if response["record_only"]!=true{return Err("Response was not recorded in hold mode".into());}
        let marker=self.run_dir(run)?.join("artifacts")
            .join(format!("decision-{}.release.json",id(request_id)?));
        let body=json!({"schema_version":1,"run_id":run,"request_id":request_id,
            "response_sha256":sha256(raw.as_bytes())});
        if marker.exists(){if read(&marker)?!=body{return Err("Conflicting response release marker".into());}}
        else{
            let snapshot=self.snapshot(run)?;
            if snapshot["status"]!="awaiting-input" || snapshot["state"]["pending"]["request_id"]!=request_id {
                return Err("Run is not waiting for that recorded response".into());
            }
            write(&marker,&body,true)?;
        }
        Ok(body)
    }
    pub fn response_exists(&self,run:&str,request_id:&str)->Result<bool,String>{
        let conn=self.connection()?;
        let found:Option<i64>=conn.query_row("SELECT 1 FROM app_responses WHERE run_id=?1 AND request_id=?2",
            params![run,request_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        Ok(found.is_some())
    }
    pub fn consume_answer(&self, run: &str) -> Result<Value, String> {
        if let Some(writer)=&self.writer {let run=run.to_owned();return writer.call(move|s|s.workflow.consume_answer(&run));}
        let snapshot = self.snapshot(run)?;
        let state = &snapshot["state"];
        let pending = &state["pending"];
        if pending.is_null() {
            return Ok(snapshot);
        }
        let request_id = pending["request_id"]
            .as_str()
            .ok_or("invalid pending request")?;
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let raw: Option<String> = tx
            .query_row(
                "SELECT payload FROM app_responses WHERE run_id=?1 AND request_id=?2",
                params![run, request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some(raw) = raw else {
            return Ok(snapshot);
        };
        let response = decoded(raw.clone())?;
        if response["record_only"]==true {
            let marker=self.run_dir(run)?.join("artifacts")
                .join(format!("decision-{}.release.json",id(request_id)?));
            if !marker.exists(){return Ok(snapshot);}
            let release=read(&marker)?;
            if release["response_sha256"]!=sha256(raw.as_bytes()) || release["request_id"]!=request_id {
                return Err("Record-only release marker disagrees with preserved response".into());
            }
        }
        let (revision, active, _, status) = tx_read(&tx, run)?;
        if revision != snapshot["revision"].as_u64().unwrap_or(u64::MAX)
            || active["pending"] != *pending
            || status != "awaiting-input"
        {
            return Err("Stale answer consumption; re-read before acting".into());
        }
        if response["artifact_sha256"] != pending["artifact_sha256"]
            || response["decision_revision"] != pending["decision_revision"]
            || response.get("context_digest") != pending.get("context_digest")
        {
            return Err("Recorded answer no longer binds to pending gate".into());
        }
        self.artifact_text(run, &state["current"])?;
        self.export_response(run, &response, &raw)?;
        let mut next = state.clone();
        next["pending"] = Value::Null;
        let resulting = if pending["kind"] == "choice" {
            let mut decisions = state["decisions"]
                .as_array()
                .ok_or("invalid decisions")?
                .clone();
            let mut decision = response.clone();
            decision["question"] = pending["question"].clone();
            decisions.push(decision);
            next["decisions"] = json!(decisions);
            next["decision_revision"] = json!(state["decision_revision"].as_u64().unwrap_or(0) + 1);
            let questions = state["questions"].as_array().ok_or("invalid questions")?;
            if questions.is_empty() || questions[0] != pending["question"] {
                return Err("Pending question changed".into());
            }
            next["questions"] = json!(questions[1..]);
            if state["workflow_version"] == 2 {
                next["revision_required"] = true.into();
                if next["next_stage"] == "final" {
                    next["blocked_reason"]="Answer saved, but no author revision slot remains; separately authorized follow-up required".into();
                }
            }
            if next["blocked_reason"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
            {
                "blocked"
            } else {
                "running"
            }
        } else {
            if response["outcome"] == "approved"
                && response["artifact_sha256"] != state["current"]["sha256"]
            {
                return Err("Approval no longer matches current artifact".into());
            }
            let result = response["outcome"]
                .as_str()
                .ok_or("invalid final response")?;
            next["outcome"] = result.into();
            result
        };
        tx_update(
            &tx,
            run,
            revision,
            &next,
            resulting,
            next["blocked_reason"].as_str().filter(|s| !s.is_empty()),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        self.snapshot(run)
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use tempfile::TempDir;
    #[test]
    fn missing_workspace_rebind_is_append_only_and_refuses_reserved_stage(){
        let root=TempDir::new().unwrap();
        let store=Store::open(root.path()).unwrap();
        let envelope=json!({"schema_version":1,"run_id":"route-test","thread_id":"route-test",
            "brief":"Project-local simulated decision log","config":{"review_limit":2,"roles":{
                "author":{"provider":"fake","model":"author"},"critic":{"provider":"fake","model":"critic"}},
                "fake":{"seconds":0.0}},
            "config_sha256":canonical_sha256(&json!({"review_limit":2,"roles":{
                "author":{"provider":"fake","model":"author"},"critic":{"provider":"fake","model":"critic"}},
                "fake":{"seconds":0.0}})).unwrap(),
            "instructions":[],"rules_sha256":canonical_sha256(&json!([])).unwrap(),
            "skills":{"draft":"deterministic test","critique":"deterministic test","revise":"deterministic test"},
            "target":{"socket":"/tmp/old.sock","workspace":"w1"},"workflow_version":1,"project_key":"test"});
        store.create(&envelope).unwrap();
        let original=fs::read(store.run_dir("route-test").unwrap().join("inputs/envelope.json")).unwrap();
        let target=json!({"socket":"/tmp/new.sock","workspace":"w2"});
        assert_eq!(store.append_routing("route-test",&target,"verified missing workspace",2).unwrap()["status"],"bound");
        assert_eq!(store.effective_target("route-test").unwrap(),target);
        assert_eq!(fs::read(store.run_dir("route-test").unwrap().join("inputs/envelope.json")).unwrap(),original);
        let reserved=store.reserve_stage("route-test").unwrap();
        assert_eq!(read(&PathBuf::from(reserved["directory"].as_str().unwrap()).join("operation.json")).unwrap()["target"],target);
        assert!(store.append_routing("route-test",&json!({"socket":"/tmp/new.sock","workspace":"w3"}),"late",3)
            .unwrap_err().contains("frozen execution identity"));
    }
    #[test]
    fn decision_export_replays_authoritative_json_bytes_without_float_reencoding(){
        let root=TempDir::new().unwrap();let store=Store::open(root.path()).unwrap();
        let raw="{\n  \"answer\": \"Fixture approval\",\n  \"artifact_sha256\": \"abc\",\n  \"outcome\": \"approved\",\n  \"request_id\": \"A1\",\n  \"time\": 1791836896.7048361\n}\n";
        let value:Value=serde_json::from_str(raw).unwrap();
        store.export_response("byte-test",&value,raw).unwrap();
        store.export_response("byte-test",&value,raw).unwrap();
        assert_eq!(fs::read(store.run_dir("byte-test").unwrap().join("artifacts/decision-A1.json")).unwrap(),raw.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_state_commit_after_publication_reuses_exact_saved_output() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let envelope = json!({"schema_version":1,"workflow_version":1,"run_id":"crash",
            "brief":"crash fixture","instructions":[],"skills":{"draft":"fake","critique":"fake","revise":"fake"},
            "rules_sha256":"test","config_sha256":"test","target":{"socket":"/tmp/test-only","workspace":"test"},
            "config":{"roles":{"author":{"provider":"fake","model":"test"},"critic":{"provider":"fake","model":"test"}},
                "fake":{"seconds":0},"max_invocations":5}});
        store.create(&envelope).unwrap();
        let reservation = store.reserve_stage("crash").unwrap();
        let packet = &reservation["packet"];
        let directory = Path::new(reservation["directory"].as_str().unwrap());
        let result = crate::fake::result(packet).unwrap();
        let op = read(&directory.join("operation.json")).unwrap();
        let runner = json!({"pid":0,"start":"synthetic","boot":"synthetic"});
        write(
            &directory.join("claim.json"),
            &json!({"runner":runner,"test_only":true}),
            true,
        )
        .unwrap();
        write(&directory.join("result.json"), &result, true).unwrap();
        write(&directory.join("receipt.json"),&json!({"operation_id":op["operation_id"],
            "packet_sha256":op["packet_sha256"],"launch_token":op["launch_token"],
            "runner":runner,"status":"completed","result_sha256":canonical_sha256(&result).unwrap(),"test_only":true}),true).unwrap();
        assert!(
            store
                .apply_saved_inner("crash", "draft", || Err(
                    "injected persistence failure".into()
                ))
                .unwrap_err()
                .contains("injected persistence failure")
        );
        let artifact = store.run_dir("crash").unwrap().join("artifacts/draft.md");
        let before = fs::read(&artifact).unwrap();
        assert_eq!(
            store.snapshot("crash").unwrap()["state"]["completed"],
            json!([])
        );
        drop(store);
        let fresh = Store::open(temp.path()).unwrap();
        let applied = fresh.apply_saved("crash", "draft").unwrap();
        assert_eq!(applied["state"]["completed"], json!(["draft"]));
        assert_eq!(fs::read(&artifact).unwrap(), before);
        assert_eq!(fresh.apply_saved("crash", "draft").unwrap(), applied);
    }
}
