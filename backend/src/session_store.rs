//! Project-scoped command, task and lifecycle authority for protocol major 1.
//! Scheduling and external execution remain outside these short transactions.
use crate::{
    codec::{canonical, canonical_sha256, sha256},
    context_access::GrantReader,
    db_writer::DbWriter,
    execution::{now, read, write},
    local_context::LocalContext,
    workflow_store::Store,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Session {
    pub workflow: Store,
    pub project: PathBuf,
    pub key: String,
    pub release: String,
}
fn encoded(value: &Value) -> Result<String, String> {
    String::from_utf8(canonical(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn decoded(raw: String) -> Result<Value, String> {
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}
fn reject(code: &str, message: &str) -> Value {
    json!({"status":"rejected","code":code,"message":message})
}
fn meta(tx: &Transaction<'_>, name: &str) -> Result<Value, String> {
    let raw: String = tx
        .query_row(
            "SELECT value FROM session_metadata WHERE name=?1",
            [name],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    decoded(raw)
}
fn revision(tx: &Transaction<'_>) -> Result<u64, String> {
    meta(tx, "state_revision")?
        .as_u64()
        .ok_or("invalid state revision".into())
}
fn event(
    tx: &Transaction<'_>,
    kind: &str,
    task: Option<&str>,
    body: &Value,
) -> Result<(i64, u64), String> {
    tx.execute(
        "INSERT INTO session_events(schema_version,type,task_id,body) VALUES(1,?1,?2,?3)",
        params![kind, task, encoded(body)?],
    )
    .map_err(|e| e.to_string())?;
    let seq = tx.last_insert_rowid();
    let next = revision(tx)? + 1;
    tx.execute(
        "UPDATE session_metadata SET value=?1 WHERE name='state_revision'",
        [encoded(&json!(next))?],
    )
    .map_err(|e| e.to_string())?;
    Ok((seq, next))
}
fn task_row(tx: &Transaction<'_>, id: &str) -> Result<Option<Value>, String> {
    tx.query_row("SELECT task_id,version,state_revision,status,title,body,run_id FROM session_tasks WHERE task_id=?1",[id],|r|{
        Ok(json!({"task_id":r.get::<_,String>(0)?,"version":r.get::<_,i64>(1)?,
            "state_revision":r.get::<_,i64>(2)?,"status":r.get::<_,String>(3)?,
            "title":r.get::<_,String>(4)?,"body":r.get::<_,String>(5)?,
            "run_id":r.get::<_,Option<String>>(6)?}))
    }).optional().map_err(|e|e.to_string())
}
fn command_prior(tx: &Transaction<'_>, body: &Value) -> Result<Option<Value>, String> {
    let id = body["command_id"].as_str().ok_or("missing command ID")?;
    let old: Option<(String, String)> = tx
        .query_row(
            "SELECT body,outcome FROM session_commands WHERE command_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((old, outcome)) = old {
        if old == encoded(body)? {
            return Ok(Some(decoded(outcome)?));
        }
        return Ok(Some(json!({"status":"rejected","code":"idempotency_conflict"})));
    }
    Ok(None)
}
fn command_commit(tx: &Transaction<'_>, body: &Value, outcome: &Value) -> Result<(), String> {
    tx.execute(
        "INSERT INTO session_commands VALUES(?1,?2,?3)",
        params![
            body["command_id"].as_str(),
            encoded(body)?,
            encoded(outcome)?
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

impl Session {
    pub fn attach_writer(&mut self,writer:Arc<DbWriter>){self.workflow.writer=Some(writer);}
    fn selected_file(&self, input: &str) -> Result<PathBuf, String> {
        let candidate = Path::new(input);
        let path = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.project.join(candidate)
        };
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&self.project) || !path.is_file() {
            return Err("Selected configuration must be a project-local file".into());
        }
        Ok(path)
    }
    pub fn plan(
        &self,
        task_id: &str,
        profile_input: &str,
        selection_input: Option<&str>,
        workspace: &str,
        socket: &str,
    ) -> Result<Value, String> {
        let task = self.task(task_id)?.ok_or("Task not found")?;
        if task["status"] != "proposed" {
            return Err("Task has no validated coordinator proposal".into());
        }
        let proposal = self
            .proposal(task_id)?
            .ok_or("Coordinator proposal missing")?;
        let body = &proposal["body"];
        if body["task_version"] != task["version"]
            || body["message_sha256"]
                != sha256(task["body"].as_str().ok_or("invalid task body")?.as_bytes())
        {
            return Err("Coordinator proposal is stale".into());
        }
        let profile = self.selected_file(profile_input)?;
        let bytes = fs::read(&profile).map_err(|e| e.to_string())?;
        let parsed: toml::Value = toml::from_slice(&bytes).map_err(|e| e.to_string())?;
        let config = serde_json::to_value(parsed).map_err(|e| e.to_string())?;
        if config["review_limit"] != 2
            || config["roles"]["author"]["provider"] != "fake"
            || config["roles"]["critic"]["provider"] != "fake"
        {
            return Err(
                "Rust service acceptance currently permits the two-role simulated profile".into(),
            );
        }
        if workspace.is_empty() || socket.is_empty() {
            return Err("Bound Herdr workspace and socket required".into());
        }
        let requested_run:Option<String>=self.workflow.connection()?.query_row(
            "SELECT run_id FROM session_run_requests WHERE task_id=?1",[task_id],|r|r.get(0))
            .optional().map_err(|e|e.to_string())?;
        let run = requested_run.unwrap_or_else(||format!("r-{task_id}"));
        let mut plan = json!({"schema_version":1,"project_key":self.key,"task_id":task_id,
            "task_version":task["version"],"task_state_revision":task["state_revision"],
            "run_id":run,"brief":body["brief"],"brief_sha256":sha256(body["brief"].as_str().ok_or("Invalid proposal brief")?.as_bytes()),
            "source_message_sha256":body["message_sha256"],
            "profile":profile.strip_prefix(&self.project).map_err(|e|e.to_string())?.to_string_lossy(),
            "profile_sha256":sha256(&bytes),"workspace":workspace,"herdr_socket":socket,
            "config":config,"proposal_result_sha256":proposal["result_sha256"],
            "max_worker_invocations":5,"review_limit":2});
        if config["workflow_version"] == 2 {
            let selection = self.selected_file(
                selection_input.unwrap_or(".waywr/context/selection.toml"),
            )?;
            let snapshot = LocalContext::new(&self.project, &self.workflow.state, &self.key)?
                .resolve(&selection, Some(&run))?;
            plan["context"] = json!(
                selection
                    .strip_prefix(&self.project)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
            );
            plan["context_snapshot_sha256"] =
                json!(canonical_sha256(&snapshot).map_err(|e| e.to_string())?);
            plan["context_snapshot"] = snapshot;
        }
        let grants = GrantReader {
            state: self.workflow.state.clone(),
            recipient_key: self.key.clone(),
        }
        .active_for_task(task_id)?;
        if !grants.is_empty() {
            let imports = grants
                .iter()
                .map(|g| {
                    Ok(json!({"grant_id":g["grant_id"],
                "item_sha256":canonical_sha256(&g["item"]).map_err(|e|e.to_string())?,
                "purpose":g["purpose"],"item":g["item"]}))
                })
                .collect::<Result<Vec<_>, String>>()?;
            if canonical(&json!(imports)).map_err(|e| e.to_string())?.len() > 48 * 1024 {
                return Err("Selected imports exceed 48 KiB".into());
            }
            plan["cross_imports"] = json!(imports);
        }
        if self.conversation_enabled()?{
            let handoff=self.conversation_handoff()?;
            plan["workflow_profile"]=json!(crate::static_files::PROFILE);
            plan["max_worker_invocations"]=json!(4);
            plan["review_limit"]=json!(2);
            plan["context_handoff_sha256"]=json!(canonical_sha256(&handoff).map_err(|e|e.to_string())?);
            plan["decision_sha256"]=json!(canonical_sha256(&handoff["accepted_decisions"]).map_err(|e|e.to_string())?);
        }
        Ok(plan)
    }
    pub fn plan_host(&self,task_id:&str,profile_input:&str,selection_input:Option<&str>)
        ->Result<Value,String>{
        // Reuse the existing configuration/context/authorization resolver.
        // Display routing is removed before the returned plan is frozen.
        let mut plan=self.plan(task_id,profile_input,selection_input,"host-only","host-only")?;
        let object=plan.as_object_mut().ok_or("Invalid plan object")?;
        object.remove("workspace");object.remove("herdr_socket");
        plan["schema_version"]=2.into();
        plan["execution"]=json!({"host":"systemd-user-v1"});
        Ok(plan)
    }
    pub fn envelope(&self, plan: &Value) -> Result<Value, String> {
        if plan["project_key"] != self.key {
            return Err("Plan belongs to another project".into());
        }
        let config = &plan["config"];
        let host=plan["schema_version"]==2 && plan["execution"]["host"]=="systemd-user-v1";
        let mut envelope = json!({"schema_version":if host{2}else{1},"run_id":plan["run_id"],"thread_id":plan["run_id"],
            "brief":plan["brief"],"config":config,
            "config_sha256":canonical_sha256(config).map_err(|e|e.to_string())?,
            "instructions":[],"rules_sha256":canonical_sha256(&json!([])).map_err(|e|e.to_string())?,
            "skills":{"draft":"deterministic test","critique":"deterministic test","revise":"deterministic test"},
            "target":if host{json!({"host":"systemd-user-v1"})}
                else{json!({"socket":plan["herdr_socket"],"workspace":plan["workspace"]})},
            "workflow_version":config["workflow_version"].as_u64().unwrap_or(1),
            "project_key":self.key,"cross_imports":plan["cross_imports"].as_array().cloned().unwrap_or_default()});
        if plan.get("context_snapshot").is_some() {
            envelope["context_snapshot"] = plan["context_snapshot"].clone();
            envelope["context_digest"] = plan["context_snapshot_sha256"].clone();
        }
        Ok(envelope)
    }
    pub fn open(project: &Path, state: &Path, key: &str, release: &str) -> Result<Self, String> {
        let workflow = Store::open(state)?;
        let project = project.canonicalize().map_err(|e| e.to_string())?;
        let marker = workflow.state.join("project-identity.json");
        if marker.exists() && read(&marker)? != json!({"schema_version":1,"project_key":key}) {
            return Err("Project state belongs to another project".into());
        }
        write(
            &marker,
            &json!({"schema_version":1,"project_key":key}),
            true,
        )?;
        let db = workflow.connection()?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS session_metadata(name TEXT PRIMARY KEY,value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_commands(command_id TEXT PRIMARY KEY,body TEXT NOT NULL,outcome TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_tasks(task_id TEXT PRIMARY KEY,version INTEGER NOT NULL,
                state_revision INTEGER NOT NULL,status TEXT NOT NULL,title TEXT NOT NULL,body TEXT NOT NULL,run_id TEXT);
            CREATE TABLE IF NOT EXISTS session_run_requests(task_id TEXT PRIMARY KEY,run_id TEXT NOT NULL UNIQUE);
            CREATE UNIQUE INDEX IF NOT EXISTS session_one_workflow ON session_tasks((1)) WHERE run_id IS NOT NULL
                AND status IN ('active','awaiting-input','recovering','blocked','uncertain','output-invalid','paused');
            CREATE TABLE IF NOT EXISTS session_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,
                schema_version INTEGER NOT NULL,type TEXT NOT NULL,task_id TEXT,body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_coordinator_calls(call_id TEXT PRIMARY KEY,task_id TEXT NOT NULL UNIQUE,
                packet_sha256 TEXT NOT NULL,status TEXT NOT NULL,result_sha256 TEXT);
            CREATE TABLE IF NOT EXISTS session_proposals(task_id TEXT PRIMARY KEY,call_id TEXT NOT NULL,
                result_sha256 TEXT NOT NULL,body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS session_admissions(task_id TEXT PRIMARY KEY,command_id TEXT NOT NULL UNIQUE,
                plan_sha256 TEXT NOT NULL,plan_json TEXT NOT NULL);").map_err(|e|e.to_string())?;
        db.execute(
            "INSERT OR IGNORE INTO session_metadata VALUES('state_revision','0')",
            [],
        )
        .map_err(|e| e.to_string())?;
        db.execute(
            "INSERT OR IGNORE INTO session_metadata VALUES('lifecycle_v1',?1)",
            [encoded(
                &json!({"schema_version":1,"phase":"running","generation":0,
                "exit_id":null,"deadline":null,"notice":null}),
            )?],
        )
        .map_err(|e| e.to_string())?;
        db.execute("INSERT OR IGNORE INTO session_metadata VALUES('service_attention_v1','null')",[])
            .map_err(|e|e.to_string())?;
        let session=Self {
            workflow,
            project,
            key: key.into(),
            release: release.into(),
        };
        if session.conversation_enabled()?{session.conversation_schema()?;session.static_schema()?;}
        Ok(session)
    }
    /// Inspect an initialized session without schema writes or marker creation.
    /// Live mutations still go through the owning project service and writer.
    pub fn open_existing(project:&Path,state:&Path,key:&str,release:&str)->Result<Self,String>{
        if !state.join("state.sqlite").is_file(){return Err("Project state is not initialized".into());}
        let workflow=Store::open(state)?;
        let project=project.canonicalize().map_err(|e|e.to_string())?;
        if read(&workflow.state.join("project-identity.json"))?
            !=json!({"schema_version":1,"project_key":key}){
            return Err("Project state belongs to another project".into());
        }
        let conn=workflow.connection()?;
        let revision:Option<String>=conn.query_row("SELECT value FROM session_metadata WHERE name='state_revision'",
            [],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        if revision.is_none(){return Err("Project session metadata is incomplete".into());}
        Ok(Self{workflow,project,key:key.into(),release:release.into()})
    }
    /// Cheap projection watermark; do not parse retained event bodies to see if
    /// anything changed. Full snapshots and history remain separate reads.
    pub(crate) fn current_revision(&self)->Result<u64,String>{
        let mut connection=self.workflow.connection()?;
        let tx=connection.transaction_with_behavior(TransactionBehavior::Deferred).map_err(|e|e.to_string())?;
        revision(&tx)
    }
    pub fn snapshot(&self) -> Result<Value, String> {
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|e| e.to_string())?;
        let revision = revision(&tx)?;
        let cursor: i64 = tx
            .query_row("SELECT coalesce(max(seq),0) FROM session_events", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        let mut tasks = Vec::new();
        for query in [
            "SELECT task_id,version,state_revision,status,title,body,run_id FROM session_tasks WHERE status NOT IN ('approved','rejected','failed','cancelled') ORDER BY rowid LIMIT 100",
            "SELECT task_id,version,state_revision,status,title,body,run_id FROM session_tasks WHERE status IN ('approved','rejected','failed','cancelled') ORDER BY rowid DESC LIMIT 50",
        ] {
            let mut stmt = tx.prepare(query).map_err(|e| e.to_string())?;
            let items=stmt.query_map([],|r|Ok(json!({"task_id":r.get::<_,String>(0)?,"version":r.get::<_,i64>(1)?,
                "state_revision":r.get::<_,i64>(2)?,"status":r.get::<_,String>(3)?,"title":r.get::<_,String>(4)?,
                "body":r.get::<_,String>(5)?,"run_id":r.get::<_,Option<String>>(6)?}))).map_err(|e|e.to_string())?;
            let mut group = items
                .map(|x| x.map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            if query.contains("DESC") {
                group.reverse();
            }
            tasks.extend(group);
        }
        let mut stmt = tx
            .prepare("SELECT seq,type,task_id,body FROM session_events ORDER BY seq DESC LIMIT 200")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut events = rows
            .map(|row| row.map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        events.reverse();
        let mut recent = events
            .into_iter()
            .map(|(seq, kind, task, body)| {
                Ok(json!({"seq":seq,"type":kind,"task_id":task,
            "body":decoded(body)?}))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if self.scope_enabled(){recent.retain(|v|v["type"]!="conversation.message");let mut bytes=0usize;recent=recent.into_iter().rev().take_while(|v|{bytes+=serde_json::to_vec(v).map_or(usize::MAX,|v|v.len());bytes<=128*1024}).collect();recent.reverse();}
        crate::request_history::add_event_times(&tx, &mut recent)?;
        let phase = meta(&tx, "lifecycle_v1")?;
        Ok(
            json!({"state_revision":revision,"cursor":cursor,"tasks":tasks,"recent_events":recent,
            "project_key":self.key,"session":phase,"service_attention":meta(&tx,"service_attention_v1")?}),
        )
    }
    pub fn events(&self, after: u64, limit: u64) -> Result<Value, String> {
        if !(1..=64).contains(&limit) {
            return Err("Invalid event limit".into());
        }
        let conn = self.workflow.connection()?;
        let mut stmt=conn.prepare("SELECT seq,type,task_id,body FROM session_events WHERE seq>?1 ORDER BY seq LIMIT ?2")
            .map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(params![after as i64, limit as i64], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let items = rows
            .map(|row| row.map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;

        let mut values = items
            .into_iter()
            .map(|(seq, kind, task, body)| {
                Ok(json!({"seq":seq,"type":kind,"task_id":task,
            "body":decoded(body)?}))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if self.scope_enabled(){let mut bytes=0usize;let count=values.iter().take_while(|v|{bytes+=serde_json::to_vec(v).map_or(usize::MAX,|v|v.len());bytes<=256*1024}).count();if count==0&&!values.is_empty(){return Err("Event exceeds bounded page; inspect retained record".into())}values.truncate(count);}
        crate::request_history::add_event_times(&conn, &mut values)?;
        let next=values.last().and_then(|v|v["seq"].as_u64()).unwrap_or(after);
        let revision: Option<String> = conn
            .query_row(
                "SELECT value FROM session_metadata WHERE name='state_revision'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(
            json!({"events":values,"next_seq":next,"state_revision":revision.and_then(|v|v.parse::<u64>().ok()).unwrap_or(0)}),
        )
    }
    pub fn command_get(&self, id: &str) -> Result<Value, String> {
        let conn = self.workflow.connection()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT outcome FROM session_commands WHERE command_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        raw.map(decoded)
            .transpose()
            .map(|v| v.unwrap_or_else(|| json!({"status":"not_found"})))
    }
    pub fn command_prior(&self,body:&Value)->Result<Option<Value>,String>{
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction().map_err(|e|e.to_string())?;
        command_prior(&tx,body)
    }
    pub fn task(&self, id: &str) -> Result<Option<Value>, String> {
        let mut conn = self.workflow.connection()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        task_row(&tx, id)
    }
    pub fn task_by_run(&self,run:&str)->Result<Option<Value>,String>{
        let conn=self.workflow.connection()?;
        let task:Option<String>=conn.query_row("SELECT task_id FROM session_tasks WHERE run_id=?1",
            [run],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        task.map(|id|self.task(&id)).transpose().map(|found|found.flatten())
    }
    pub fn proposal(&self, id: &str) -> Result<Option<Value>, String> {
        let conn = self.workflow.connection()?;
        let raw: Option<(String, String, String)> = conn
            .query_row(
                "SELECT call_id,result_sha256,body FROM session_proposals WHERE task_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        raw.map(|(call, result, body)| {
            Ok(json!({"call_id":call,"result_sha256":result,"body":decoded(body)?}))
        })
        .transpose()
    }
    pub fn task_detail(&self,id:&str)->Result<Value,String>{
        let task=self.task(id)?.ok_or("Task not found in this project")?;
        let mut detail=json!({"task":task,"proposal":self.proposal(id)?,"pending":null,
            "current":null,"artifact_path":null,"completed":[],"findings":{}});
        if self.scope_enabled(){detail["scope"]=self.scope_detail(id)?.unwrap_or(Value::Null);detail["execution_available"]=json!(self.development());return Ok(detail);}
        let conn=self.workflow.connection()?;
        let coordinator:Option<(String,String,String,Option<String>)>=conn.query_row(
            "SELECT call_id,packet_sha256,status,result_sha256 FROM session_coordinator_calls WHERE task_id=?1",
            [id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
            .optional().map_err(|e|e.to_string())?;
        if let Some((call,packet,status,result))=coordinator {
            let last:Option<String>=conn.query_row(
                "SELECT body FROM session_events WHERE task_id=?1 AND type='task.state_changed' ORDER BY seq DESC LIMIT 1",
                [id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
            let last=last.map(decoded).transpose()?;
            detail["coordinator"]=json!({"call_id":call,"packet_sha256":packet,
                "status":status,"result_sha256":result,
                "last_error":last.as_ref().and_then(|v|v.get("coordinator_execution"))
                    .and_then(Value::as_str).map(|_|last.as_ref().unwrap()["detail"].clone())});
        }
        if let Some(run)=task["run_id"].as_str(){
            if let Some(static_run)=self.static_snapshot(run)?{
                detail["run"]=json!({"run_id":run,"status":static_run["status"],"profile":crate::static_files::PROFILE});
                if task["status"]=="awaiting-input"{detail["pending"]=static_run["pending"].clone();}
                detail["current"]=static_run["candidate"].clone();
                detail["completed"]=static_run["completed"].clone();
                detail["findings"]=static_run["findings"].clone();
                detail["assessments"]=static_run["assessments"].clone();
                detail["budget_used"]=static_run["used"].clone();
                if let Some(stage)=static_run["candidate"]["stage"].as_str(){
                    detail["artifact_path"]=json!(self.workflow.state.join("runs").join(run)
                        .join("static/candidates").join(stage).join("manifest.json"));
                }
                return Ok(detail);
            }
            let snapshot=self.workflow.snapshot(run)?;
            let state=&snapshot["state"];
            detail["run"]=json!({"run_id":run,"status":snapshot["status"]});
            // Gate publication and task projection are separate short commits.
            // Do not advertise an actionable gate until the session task has
            // reached the state accepted by answer_task.
            if task["status"]=="awaiting-input"{detail["pending"]=state["pending"].clone();}
            detail["current"]=state["current"].clone();
            detail["completed"]=state["completed"].clone();
            detail["findings"]=state["findings"].clone();
            if let Some(path)=state["current"]["path"].as_str(){
                let full=self.workflow.run_dir(run)?.join(path);
                let root=self.workflow.run_dir(run)?.canonicalize().map_err(|e|e.to_string())?;
                let full=full.canonicalize().map_err(|e|e.to_string())?;
                if !full.starts_with(root){return Err("Artifact path leaves project run".into());}
                let bytes=fs::read(&full).map_err(|e|e.to_string())?;
                if sha256(&bytes)!=state["current"]["sha256"]{return Err("Artifact integrity mismatch".into());}
                detail["artifact_path"]=json!(full);
            }
        }
        Ok(detail)
    }
    pub fn phase(&self) -> Result<Value, String> {
        let mut conn = self.workflow.connection()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        meta(&tx, "lifecycle_v1")
    }
    pub fn service_attention(&self, issue: Option<&str>, attempts: u8) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let issue=issue.map(str::to_owned);
            return writer.call(move|s|s.service_attention(issue.as_deref(),attempts));}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let prior=meta(&tx,"service_attention_v1")?;
        let next=issue.map(|reason|json!({"schema_version":1,"status":if attempts>=3{"stopped"}else{"recovering"},
            "reason":reason,"attempts":attempts,
            "operator_action":if attempts>=3{"Inspect this project's retained service error records; restart only this project's service after resolving the cause"}
                else{"Inspect this project's retained service error records"}})).unwrap_or(Value::Null);
        if prior!=next{
            tx.execute("UPDATE session_metadata SET value=?1 WHERE name='service_attention_v1'",[encoded(&next)?])
                .map_err(|e|e.to_string())?;
            if issue.is_none() || attempts==1 || attempts>=3{
                event(&tx,"session.attention",None,&next)?;
            }
        }
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn submit(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {let body=body.clone();return writer.call(move|s|s.submit(&body));}
        let command = body["command_id"].as_str().ok_or("missing command ID")?;
        if command.is_empty() || command.len() > 128 {
            return Ok(reject("invalid_request", "Invalid command ID"));
        }
        if body["action"] != "submit_message" || !body["payload"].is_object() {
            return Ok(reject("invalid_request", "Expected submit_message"));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(old) = command_prior(&tx, body)? {
            return Ok(old);
        }
        let current = revision(&tx)?;
        if body["expected_state_revision"] != current {
            return Ok(json!({"status":"rejected","code":"stale_state","state_revision":current}));
        }
        if meta(&tx, "lifecycle_v1")?["phase"] != "running" {
            return Ok(reject(
                "session_paused",
                "Resume this project session before adding work",
            ));
        }
        let text = body["payload"]["text"]
            .as_str()
            .ok_or("missing task text")?;
        if text.trim().is_empty() || text.len() > 32768 {
            return Ok(reject("invalid_request", "Task text missing or too large"));
        }
        let requested_run=body["payload"]["run_id"].as_str();
        if let Some(run)=requested_run
            && (run.is_empty() || run.len()>64 || !run.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'||c==b'_')) {
            return Ok(reject("invalid_request","Invalid requested run ID"));
        }
        let count:i64=tx.query_row("SELECT count(*) FROM session_tasks WHERE status NOT IN ('approved','rejected','failed','cancelled')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if count >= 100 {
            return Ok(reject(
                "queue_full",
                "This project already has 100 unresolved tasks",
            ));
        }
        let id = format!("t-{}", &sha256(command.as_bytes())[..24]);
        let title = text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(100)
            .collect::<String>();
        tx.execute(
            "INSERT INTO session_tasks VALUES(?1,1,1,'queued',?2,?3,NULL)",
            params![id, title, text],
        )
        .map_err(|e| e.to_string())?;
        if let Some(run)=requested_run {
            tx.execute("INSERT INTO session_run_requests(task_id,run_id) VALUES(?1,?2)",params![id,run])
                .map_err(|e|format!("Requested run ID is already claimed: {e}"))?;
        }
        let (seq, next) = event(
            &tx,
            "task.queued",
            Some(&id),
            &json!({"task_id":id,"version":1,"status":"queued","text":text}),
        )?;
        let outcome =
            json!({"status":"accepted","task_id":id,"event_seq":seq,"state_revision":next});
        command_commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn current_revision_reads_only_the_watermark_not_retained_event_bodies(){
        let root=TempDir::new().unwrap();let s=session(&root,"project","revision-reader");
        assert_eq!(s.current_revision().unwrap(),0);
        let connection=s.workflow.connection().unwrap();
        connection.execute("INSERT INTO session_events(schema_version,type,body) VALUES(1,'fixture.retained','invalid old event body')",[]).unwrap();
        connection.execute("UPDATE session_metadata SET value='7\n' WHERE name='state_revision'",[]).unwrap();
        assert_eq!(s.current_revision().unwrap(),7,"Watermark reads must be independent of retained history parsing");
    }

    fn session(root:&TempDir,name:&str,key:&str)->Session{
        let project=root.path().join(name);
        fs::create_dir_all(project.join(".waywr")).unwrap();
        fs::write(project.join(".waywr/workflow.toml"),
            "review_limit = 2\n[roles.author]\nprovider = 'fake'\nmodel = 'deterministic-author'\n[roles.critic]\nprovider = 'fake'\nmodel = 'deterministic-critic'\n[fake]\nseconds = 0.0\n").unwrap();
        let state=project.join(".waywr/state");
        Session::open(&project,&state,key,"test-release").unwrap()
    }

    #[test]
    fn repeated_exit_accepts_existing_fence_without_replacing_drain_or_resuming_work(){
        let root=TempDir::new().unwrap();
        let store=session(&root,"repeated-exit","key-exit");
        let first=store.lifecycle_command(&json!({"command_id":"first-exit","action":"stop_session",
            "expected_state_revision":0,"payload":{}})).unwrap();
        assert_eq!(first["status"],"accepted");
        let generation=first["session"]["generation"].as_u64().unwrap();
        for phase in ["exit-requested","draining","paused"] {
            if phase!="exit-requested" {
                store.update_exit("first-exit",generation,phase,None).unwrap().unwrap();
            }
            let before=store.snapshot().unwrap();
            let command=json!({"command_id":format!("repeat-{phase}"),"action":"stop_session",
                "expected_state_revision":0,"payload":{}});
            let result=store.lifecycle_command(&command).unwrap();
            assert_eq!(result["status"],"accepted");
            assert_eq!(result["already_stopped"],true);
            assert_eq!(result["session"],before["session"]);
            assert_eq!(store.lifecycle_command(&command).unwrap(),result);
            let after=store.snapshot().unwrap();
            assert_eq!(after["session"],before["session"]);
            assert_eq!(after["cursor"],before["cursor"]);
        }
        let revision=store.snapshot().unwrap()["state_revision"].clone();
        let resumed=store.lifecycle_command(&json!({"command_id":"resume-once","action":"resume_session",
            "expected_state_revision":revision,"payload":{}})).unwrap();
        assert_eq!(resumed["status"],"accepted");
        let stale=store.lifecycle_command(&json!({"command_id":"stale-old-exit","action":"stop_session",
            "expected_state_revision":0,"payload":{}})).unwrap();
        assert_eq!(stale["code"],"stale_state");
        assert_eq!(store.phase().unwrap(),resumed["session"]);
    }

    #[test]
    fn stopped_coordinator_remains_visible_in_task_detail() {
        let root=TempDir::new().unwrap();
        let store=session(&root,"inspect","key-inspect");
        let accepted=store.submit(&json!({"command_id":"inspect-1","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"Draft a proposal"}})).unwrap();
        let id=accepted["task_id"].as_str().unwrap();
        let task=store.task(id).unwrap().unwrap();
        store.coordinator_reserved(&task,"packet-hash").unwrap();
        store.stop_coordinator(id,"pre-send-refusal","Bootstrap tab stayed busy before launch").unwrap();
        let detail=store.task_detail(id).unwrap();
        assert_eq!(detail["task"]["status"],"blocked");
        assert_eq!(detail["coordinator"]["status"],"pre-send-refusal");
        assert_eq!(detail["coordinator"]["packet_sha256"],"packet-hash");
        assert_eq!(detail["coordinator"]["last_error"],"Bootstrap tab stayed busy before launch");
    }
    fn proposal(session:&Session,task:&Value){
        let id=task["task_id"].as_str().unwrap();
        let packet=json!({"operation_kind":"coordinator","task_id":id,"task_version":task["version"],
            "message":task["body"],"message_sha256":sha256(task["body"].as_str().unwrap().as_bytes()),
            "operation_id":format!("c-{id}-v1"),"mode":"success"});
        session.coordinator_reserved(task,&canonical_sha256(&packet).unwrap()).unwrap();
        let result=crate::fake::result(&packet).unwrap();
        session.publish_proposal(task,&result,&canonical_sha256(&result).unwrap()).unwrap();
    }
    #[test]
    fn two_projects_same_run_id_are_isolated_and_commands_replay(){
        let root=TempDir::new().unwrap();
        let a=session(&root,"a","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let b=session(&root,"b","bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let command=json!({"command_id":"same","action":"submit_message","expected_state_revision":0,
            "payload":{"text":"Prepare a decision log specification"}});
        let a_first=a.submit(&command).unwrap();let b_first=b.submit(&command).unwrap();
        assert_eq!(a_first["task_id"],b_first["task_id"]);
        assert_eq!(a.submit(&command).unwrap(),a_first);
        let mut different=command.clone();different["payload"]["text"]="other".into();
        assert_eq!(a.submit(&different).unwrap()["code"],"idempotency_conflict");
        let id=a_first["task_id"].as_str().unwrap();
        proposal(&a,&a.task(id).unwrap().unwrap());
        assert_eq!(b.task(id).unwrap().unwrap()["status"],"queued");
        let plan=a.plan(id,".waywr/workflow.toml",None,"w1","/tmp/isolated-herdr.sock").unwrap();
        assert_eq!(plan["project_key"],a.key);
        let admission=json!({"command_id":"authorize","action":"admit_task","expected_task_version":1,
            "expected_state_revision":a.task(id).unwrap().unwrap()["state_revision"],
            "payload":{"task_id":id,"plan_digest":canonical_sha256(&plan).unwrap()}});
        let accepted=a.admit(&admission,&plan).unwrap();
        assert_eq!(accepted["status"],"accepted");
        assert_eq!(a.admit(&admission,&plan).unwrap(),accepted);
        let stop=json!({"command_id":"exit","action":"stop_session",
            "expected_state_revision":a.snapshot().unwrap()["state_revision"],"payload":{}});
        let exiting=a.lifecycle_command(&stop).unwrap();
        assert_eq!(exiting["session"]["phase"],"exit-requested");
        assert!(a.activate(id,plan["run_id"].as_str().unwrap()).is_err());
        assert_eq!(b.phase().unwrap()["phase"],"running");
        let resume=json!({"command_id":"resume","action":"resume_session",
            "expected_state_revision":a.snapshot().unwrap()["state_revision"],"payload":{}});
        let resumed=a.lifecycle_command(&resume).unwrap();
        assert_eq!(resumed["session"]["phase"],"running");
        assert!(a.update_exit("exit",1,"paused",None).unwrap().is_none());
        // Admission reserves an ID; the service publishes the frozen run
        // before marking the task active. Match that order in this fixture.
        a.workflow.create(&a.envelope(&plan).unwrap()).unwrap();
        a.activate(id,plan["run_id"].as_str().unwrap()).unwrap();
        let checkpoint=a.checkpoint().unwrap();
        assert!(Path::new(checkpoint["path"].as_str().unwrap()).exists());
        assert_eq!(b.snapshot().unwrap()["tasks"][0]["status"],"queued");
        a.service_attention(Some("test-only scheduler fault"),1).unwrap();
        a.service_attention(Some("test-only scheduler fault"),3).unwrap();
        assert_eq!(a.snapshot().unwrap()["service_attention"]["status"],"stopped");
        assert!(b.snapshot().unwrap()["service_attention"].is_null());
        let cursor=a.snapshot().unwrap()["cursor"].as_u64().unwrap();
        a.service_attention(Some("test-only scheduler fault"),3).unwrap();
        assert_eq!(a.snapshot().unwrap()["cursor"],cursor);
        a.service_attention(None,0).unwrap();
        assert!(a.snapshot().unwrap()["service_attention"].is_null());
    }
    #[test]
    fn cli_requested_run_id_is_frozen_with_submission(){
        let root=TempDir::new().unwrap();
        let session=session(&root,"cli","cccccccccccccccccccccccccccccccc");
        let input=json!({"command_id":"cli-start","action":"submit_message","expected_state_revision":0,
            "payload":{"text":"Describe an app","run_id":"chosen-run"}});
        let accepted=session.submit(&input).unwrap();
        assert_eq!(session.submit(&input).unwrap(),accepted);
        let task=session.task(accepted["task_id"].as_str().unwrap()).unwrap().unwrap();
        proposal(&session,&task);
        let plan=session.plan(task["task_id"].as_str().unwrap(),".waywr/workflow.toml",
            None,"w1","/tmp/test-herdr.sock").unwrap();
        assert_eq!(plan["run_id"],"chosen-run");
        let other=json!({"command_id":"second","action":"submit_message",
            "expected_state_revision":session.snapshot().unwrap()["state_revision"],
            "payload":{"text":"Another app","run_id":"chosen-run"}});
        assert!(session.submit(&other).is_err());
    }
    #[test]
    fn quiescent_revision_accepts_canonical_json_newline_after_completed_task(){
        let root=TempDir::new().unwrap();
        let session=session(&root,"quiescent","dddddddddddddddddddddddddddddddd");
        let queued=session.submit(&json!({"command_id":"submit","action":"submit_message",
            "expected_state_revision":0,"payload":{"text":"A disposable task"}})).unwrap();
        let cancelled=session.cancel_undispatched(&json!({"command_id":"cancel","action":"cancel_task",
            "expected_task_version":1,"expected_state_revision":1,
            "payload":{"task_id":queued["task_id"]}})).unwrap();
        assert_eq!(cancelled["status"],"accepted");
        assert!(session.quiescent_for_release_change().unwrap()>=2);
    }
    #[test]
    fn release_change_refuses_pending_delivery_and_live_ui(){
        let root=TempDir::new().unwrap();
        let owner=session(&root,"a","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        owner.quiescent_for_release_change().unwrap();
        let outbox=owner.workflow.state.join("session/outbox");
        fs::create_dir_all(&outbox).unwrap();
        fs::write(outbox.join("pending.json"),"{}").unwrap();
        assert!(owner.quiescent_for_release_change().unwrap_err().contains("client delivery"));
        fs::remove_file(outbox.join("pending.json")).unwrap();
        let claims=owner.workflow.state.join("session/ui-claims");
        fs::create_dir_all(&claims).unwrap();
        write(&claims.join("test.json"),&json!({"process":crate::execution::process_identity(std::process::id()).unwrap()}),true).unwrap();
        assert!(owner.quiescent_for_release_change().unwrap_err().contains("live UI"));
    }
}

impl Session {
    pub fn quiescent_for_release_change(&self)->Result<u64,String>{
        if crate::coordination::enabled(self)&&crate::coordination::pending_intents(self)?{return Err("Pending conversational intent or answer continuation; settle it before release change".into())}
        let outbox=self.workflow.state.join("session/outbox");
        if outbox.is_dir(){
            for entry in fs::read_dir(&outbox).map_err(|e|e.to_string())?{
                if entry.map_err(|e|e.to_string())?.path().extension().is_some_and(|ext|ext=="json"){
                    return Err("Pending client delivery; release change refused".into());
                }
            }
        }
        let claims=self.workflow.state.join("session/ui-claims");
        if claims.is_dir(){
            for entry in fs::read_dir(&claims).map_err(|e|e.to_string())?{
                let path=entry.map_err(|e|e.to_string())?.path();
                if path.extension().is_some_and(|ext|ext=="json") &&
                    crate::execution::alive(&read(&path)?["process"]){
                    return Err("Verified live UI claim; release change refused".into());
                }
            }
        }
        let conn=self.workflow.connection()?;
        let unresolved:i64=conn.query_row("SELECT count(*) FROM session_tasks WHERE status NOT IN ('approved','rejected','failed','cancelled')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let workflows:i64=conn.query_row("SELECT count(*) FROM ww_workflows WHERE status NOT IN ('approved','rejected','failed','cancelled')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let coordinators:i64=conn.query_row("SELECT count(*) FROM session_coordinator_calls WHERE status='reserved'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let native_active=if self.conversation_enabled()?{conn.query_row("SELECT count(*) FROM conversation_turns WHERE status IN ('reserved','queued')",[],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?}else{0};
        if self.development(){let unsettled:i64=conn.query_row("SELECT count(*) FROM development_records WHERE (kind='workflow' AND json_type(record_json,'$.pending')='object') OR (kind='unit' AND json_extract(record_json,'$.status')='running')",[],|r|r.get(0)).map_err(|e|e.to_string())?;if unsettled>0{return Err("Development has pending operations to settle; no active work interrupted for upgrade".into())}}
        // Compatible state8 upgrades preserve proposed/approved scopes and quiescent work.
        // Exact attempts are checked separately; unresolved human decisions are not running processes.
        if (!self.development() && unresolved>0) || workflows>0 || coordinators>0 || native_active>0{
            return Err("Project has unresolved task, workflow or coordinator execution; release change refused".into());
        }
        for root in [self.workflow.state.join("session/coordinator"),self.workflow.state.join("runs")]{
            if !root.exists(){continue;}
            for entry in fs::read_dir(root).map_err(|e|e.to_string())?{
                let item=entry.map_err(|e|e.to_string())?.path();
                let children=if item.join("operations").is_dir(){item.join("operations")}else{item};
                for directory in fs::read_dir(children).map_err(|e|e.to_string())?{
                    let path=directory.map_err(|e|e.to_string())?.path();
                    if ["launch.json","claim.json","permit.json","dispatch.json","child.json"].iter()
                        .any(|name|path.join(name).exists()) && !path.join("receipt.json").exists(){
                        return Err(format!("Unresolved execution evidence at {}; release change refused",path.display()));
                    }
                }
            }
        }
        let revision:String=conn.query_row("SELECT value FROM session_metadata WHERE name='state_revision'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        revision.trim().parse().map_err(|_|"Invalid state revision".into())
    }
    pub fn transition(
        &self,
        task: &str,
        run: &str,
        status: &str,
        detail: Option<&str>,
    ) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let (task,run,status,detail)=
            (task.to_owned(),run.to_owned(),status.to_owned(),detail.map(str::to_owned));
            return writer.call(move|s|s.transition(&task,&run,&status,detail.as_deref()));}
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let Some(row) = task_row(&tx, task)? else {
            return Err("Task not found".into());
        };
        if row["run_id"] != run {
            return Err("Driver task/run binding changed".into());
        }
        if row["status"] == status {
            return Ok(());
        }
        tx.execute(
            "UPDATE session_tasks SET status=?1,state_revision=state_revision+1 WHERE task_id=?2",
            params![status, task],
        )
        .map_err(|e| e.to_string())?;
        event(
            &tx,
            "task.state_changed",
            Some(task),
            &json!({"task_id":task,"run_id":row["run_id"],
            "status":status,"detail":detail}),
        )?;
        tx.commit().map_err(|e| e.to_string())
    }
    fn first_status(&self, status: &str) -> Result<Option<Value>, String> {
        let conn = self.workflow.connection()?;
        let id: Option<String> = conn
            .query_row(
                "SELECT task_id FROM session_tasks WHERE status=?1 ORDER BY rowid LIMIT 1",
                [status],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        id.map(|id| self.task(&id)).transpose().map(Option::flatten)
    }
    pub fn queued(&self) -> Result<Option<Value>, String> {
        self.first_status("queued")
    }
    pub fn coordinating(&self) -> Result<Option<Value>, String> {
        self.first_status("coordinating")
    }
    pub fn ready(&self) -> Result<Option<Value>, String> {
        self.first_status("ready")
    }
    pub fn ready_plan(&self) -> Result<Option<(Value, Value)>, String> {
        let Some(task) = self.ready()? else {
            return Ok(None);
        };
        let plan = self
            .admitted_plan(task["task_id"].as_str().ok_or("invalid task ID")?)?
            .ok_or("Authorized plan missing")?;
        Ok(Some((task, plan)))
    }
    pub fn active(&self) -> Result<Option<Value>, String> {
        let conn = self.workflow.connection()?;
        let id:Option<String>=conn.query_row("SELECT task_id FROM session_tasks WHERE run_id IS NOT NULL AND status IN ('active','awaiting-input','recovering','blocked','uncertain','output-invalid','paused') LIMIT 1",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        id.map(|id| self.task(&id)).transpose().map(Option::flatten)
    }
    pub fn coordinator_reserved(&self, task: &Value, packet_hash: &str) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let (task,packet_hash)=(task.clone(),packet_hash.to_owned());
            return writer.call(move|s|s.coordinator_reserved(&task,&packet_hash));}
        let id = task["task_id"].as_str().ok_or("missing task ID")?;
        let call = format!(
            "{id}-v{}",
            task["version"].as_i64().ok_or("invalid task version")?
        );
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let row = task_row(&tx, id)?.ok_or("Task disappeared")?;
        if row["status"] != "queued" || row["version"] != task["version"] {
            return Err("Coordinator proposal input became stale before reservation".into());
        }
        if meta(&tx,"lifecycle_v1")?["phase"]!="running"{
            return Err("Project session exit pause fences coordinator reservation".into());
        }
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT call_id,packet_sha256 FROM session_coordinator_calls WHERE task_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some((old, hash)) = existing {
            if old != call || hash != packet_hash {
                return Err("Coordinator call identity conflict".into());
            }
        } else {
            tx.execute(
                "INSERT INTO session_coordinator_calls VALUES(?1,?2,?3,'reserved',NULL)",
                params![call, id, packet_hash],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute("UPDATE session_tasks SET status='coordinating',state_revision=state_revision+1 WHERE task_id=?1",[id]).map_err(|e|e.to_string())?;
        event(
            &tx,
            "task.state_changed",
            Some(id),
            &json!({"task_id":id,"status":"coordinating","detail":
                if self.workflow.state.join("state-contract.json").exists(){"Coordinator call hosted by this project"}
                else{"Coordinator worker visible in Herdr"}}),
        )?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn publish_proposal(&self, task: &Value, result: &Value, hash: &str) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let (task,result,hash)=(task.clone(),result.clone(),hash.to_owned());
            return writer.call(move|s|s.publish_proposal(&task,&result,&hash));}
        let id = task["task_id"].as_str().ok_or("missing task ID")?;
        let proposal = &result["proposal"];
        if proposal["task_id"] != id
            || proposal["task_version"] != task["version"]
            || proposal["message_sha256"] != sha256(task["body"].as_str().unwrap_or("").as_bytes())
        {
            return Err("Coordinator proposal refers to stale task".into());
        }
        let call = format!(
            "{id}-v{}",
            task["version"].as_i64().ok_or("invalid task version")?
        );
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let row = task_row(&tx, id)?.ok_or("Task disappeared")?;
        if row["status"] == "proposed" {
            let old: Option<String> = tx
                .query_row(
                    "SELECT result_sha256 FROM session_proposals WHERE task_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            return if old.as_deref() == Some(hash) {
                Ok(())
            } else {
                Err("Proposal changed under completed call".into())
            };
        }
        if row["status"] != "coordinating" || row["version"] != task["version"] {
            return Err("Coordinator result is stale".into());
        }
        tx.execute(
            "INSERT INTO session_proposals VALUES(?1,?2,?3,?4)",
            params![id, call, hash, encoded(proposal)?],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("UPDATE session_coordinator_calls SET status='completed',result_sha256=?1 WHERE call_id=?2",params![hash,call]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE session_tasks SET status='proposed',state_revision=state_revision+1 WHERE task_id=?1",[id]).map_err(|e|e.to_string())?;
        event(
            &tx,
            "proposal.ready",
            Some(id),
            &json!({"task_id":id,"status":"proposed","brief":proposal["brief"]}),
        )?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn admitted_plan(&self, task: &str) -> Result<Option<Value>, String> {
        let conn = self.workflow.connection()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT plan_json FROM session_admissions WHERE task_id=?1",
                [task],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        raw.map(decoded).transpose()
    }
    pub fn activate(&self, task: &str, run: &str) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let (task,run)=(task.to_owned(),run.to_owned());
            return writer.call(move|s|s.activate(&task,&run));}
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if meta(&tx, "lifecycle_v1")?["phase"] != "running" {
            return Err("Project session exit pause fences task activation".into());
        }
        if task_row(&tx, task)?
            .as_ref()
            .is_none_or(|r| r["status"] != "ready" || !r["run_id"].is_null())
        {
            return Err("Authorized task is no longer ready".into());
        }
        let occupied:i64=tx.query_row("SELECT count(*) FROM session_tasks WHERE run_id IS NOT NULL AND status IN ('active','awaiting-input','recovering','blocked','uncertain','output-invalid','paused')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if occupied != 0 {
            return Err("Another workflow in this project is unresolved".into());
        }
        tx.execute("UPDATE session_tasks SET status='active',run_id=?1,state_revision=state_revision+1 WHERE task_id=?2",params![run,task]).map_err(|e|e.to_string())?;
        event(
            &tx,
            "task.active",
            Some(task),
            &json!({"task_id":task,"run_id":run,"status":"active"}),
        )?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn admit(&self, body: &Value, plan: &Value) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {let (body,plan)=(body.clone(),plan.clone());
            return writer.call(move|s|s.admit(&body,&plan));}
        if self.scope_enabled(){return Ok(reject("outside_increment","Scope authorization cannot admit execution"));}
        let task = body["payload"]["task_id"]
            .as_str()
            .ok_or("missing task ID")?;
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(prior) = command_prior(&tx, body)? {
            return Ok(prior);
        }
        let Some(row) = task_row(&tx, task)? else {
            return Ok(reject("not_found", "Task not found"));
        };
        if row["status"] != "proposed"
            || row["version"] != body["expected_task_version"]
            || row["state_revision"] != body["expected_state_revision"]
        {
            return Ok(
                json!({"status":"rejected","code":"stale_state","task_version":row["version"],
                "task_status":row["status"]}),
            );
        }
        let hash = canonical_sha256(plan).map_err(|e| e.to_string())?;
        if body["payload"]["plan_digest"] != hash
            || plan["task_id"] != task
            || plan["task_version"] != row["version"]
            || plan["task_state_revision"] != row["state_revision"]
            || plan["project_key"] != self.key
        {
            return Ok(reject("stale_plan", "Frozen task plan changed"));
        }
        let run = plan["run_id"].as_str().ok_or("missing run ID")?;
        if !run
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
            || run.len() > 64
            || run.is_empty()
        {
            return Ok(reject("invalid_request", "Invalid run ID"));
        }
        let used:i64=tx.query_row("SELECT count(*) FROM session_admissions WHERE json_extract(plan_json,'$.run_id')=?1 AND task_id!=?2",
            params![run,task],|r|r.get(0)).map_err(|e|e.to_string())?;
        if used != 0 {
            return Ok(reject(
                "run_claimed",
                "Run ID belongs to another task in this project",
            ));
        }
        let command = body["command_id"].as_str().ok_or("missing command ID")?;
        tx.execute(
            "INSERT INTO session_admissions VALUES(?1,?2,?3,?4)",
            params![task, command, hash, encoded(plan)?],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("UPDATE session_tasks SET status='ready',state_revision=state_revision+1 WHERE task_id=?1",[task])
            .map_err(|e|e.to_string())?;
        let (seq, next) = event(
            &tx,
            "task.authorized",
            Some(task),
            &json!({"task_id":task,"run_id":run,
            "status":"ready","plan_sha256":hash}),
        )?;
        let outcome = json!({"status":"accepted","task_id":task,"run_id":run,"task_status":"ready",
            "event_seq":seq,"state_revision":next});
        command_commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }

    pub fn cancel_undispatched(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {let body=body.clone();
            return writer.call(move|s|s.cancel_undispatched(&body));}
        let task = body["payload"]["task_id"]
            .as_str()
            .ok_or("missing task ID")?;
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(prior) = command_prior(&tx, body)? {
            return Ok(prior);
        }
        let Some(row) = task_row(&tx, task)? else {
            return Ok(reject("stale_or_started", "Task unavailable"));
        };
        if !["queued", "proposed", "ready"]
            .iter()
            .any(|s| row["status"] == *s)
            || !row["run_id"].is_null()
            || row["version"] != body["expected_task_version"]
            || row["state_revision"] != body["expected_state_revision"]
        {
            return Ok(reject("stale_or_started", "Task has already advanced"));
        }
        let call: Option<String> = tx
            .query_row(
                "SELECT status FROM session_coordinator_calls WHERE task_id=?1",
                [task],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if call.as_deref() == Some("reserved") {
            return Ok(reject(
                "coordinator_active",
                "Wait for the visible coordinator call to resolve",
            ));
        }
        tx.execute("UPDATE session_tasks SET status='cancelled',state_revision=state_revision+1 WHERE task_id=?1",[task])
            .map_err(|e|e.to_string())?;
        let (seq, next) = event(
            &tx,
            "task.state_changed",
            Some(task),
            &json!({"task_id":task,"status":"cancelled",
            "detail":"Cancelled before workflow dispatch"}),
        )?;
        let outcome = json!({"status":"accepted","task_id":task,"task_status":"cancelled","event_seq":seq,"state_revision":next});
        command_commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }

    pub fn steer_queued(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {let body=body.clone();
            return writer.call(move|s|s.steer_queued(&body));}
        let task = body["payload"]["task_id"]
            .as_str()
            .ok_or("missing task ID")?;
        let text = body["payload"]["text"].as_str().ok_or("missing text")?;
        if text.trim().is_empty() || text.len() > 32768 {
            return Ok(reject("invalid_request", "Task text missing or too large"));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(prior) = command_prior(&tx, body)? {
            return Ok(prior);
        }
        let Some(row) = task_row(&tx, task)? else {
            return Ok(reject("stale_or_started", "Task unavailable"));
        };
        let call: Option<String> = tx
            .query_row(
                "SELECT call_id FROM session_coordinator_calls WHERE task_id=?1",
                [task],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if row["status"] != "queued"
            || row["version"] != body["expected_task_version"]
            || row["state_revision"] != body["expected_state_revision"]
            || call.is_some()
        {
            return Ok(reject(
                "stale_or_started",
                "Frozen work needs a new authorized task",
            ));
        }
        let version = row["version"].as_i64().ok_or("invalid task version")? + 1;
        let title = text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(100)
            .collect::<String>();
        tx.execute("UPDATE session_tasks SET version=?1,state_revision=state_revision+1,title=?2,body=?3 WHERE task_id=?4",
            params![version,title,text,task]).map_err(|e|e.to_string())?;
        let (seq, next) = event(
            &tx,
            "task.steered",
            Some(task),
            &json!({"task_id":task,"version":version,"text":text}),
        )?;
        let outcome = json!({"status":"accepted","task_id":task,"task_version":version,"state_revision":next,"event_seq":seq});
        command_commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }

    pub fn lifecycle_command(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {let body=body.clone();
            return writer.call(move|s|s.lifecycle_command(&body));}
        let action = body["action"].as_str().unwrap_or("");
        if !matches!(action, "stop_session" | "resume_session")
            || body["command_id"].as_str().is_none_or(str::is_empty)
        {
            return Ok(reject(
                "invalid_request",
                "Expected a session lifecycle command",
            ));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(prior) = command_prior(&tx, body)? {
            return Ok(prior);
        }
        let current = revision(&tx)?;
        let previous = meta(&tx, "lifecycle_v1")?;
        if previous["schema_version"] != 1
            || !["running", "exit-requested", "draining", "paused"]
                .iter()
                .any(|s| previous["phase"] == *s)
        {
            return Err("Unsupported project lifecycle record; no dispatch".into());
        }
        if action == "stop_session" && previous["phase"] != "running" {
            // The requested dispatch fence already exists. A second interface
            // may close successfully without replacing the original drain ID,
            // deadline or generation. Unrelated events cannot invalidate this
            // no-op; a concurrent resume still takes the normal stale-write path.
            let outcome = json!({"status":"accepted","already_stopped":true,
                "session":previous,"state_revision":current});
            command_commit(&tx, body, &outcome)?;
            tx.commit().map_err(|e|e.to_string())?;
            return Ok(outcome);
        }
        if body["expected_state_revision"] != current {
            return Ok(json!({"status":"rejected","code":"stale_state","state_revision":current}));
        }
        if action == "resume_session" && previous["phase"] == "running" {
            return Ok(json!({"status":"rejected","code":"already_running","session":previous}));
        }
        let generation = previous["generation"]
            .as_u64()
            .ok_or("invalid lifecycle generation")?
            + 1;
        let phase = if action == "stop_session" {
            json!({"schema_version":1,"phase":"exit-requested","generation":generation,
                "exit_id":body["command_id"],"requested_at":now(),"deadline":now()+30.0,"notice":null})
        } else {
            json!({"schema_version":1,"phase":"running","generation":generation,
                "exit_id":null,"requested_at":null,"deadline":null,"notice":previous["notice"]})
        };
        tx.execute(
            "UPDATE session_metadata SET value=?1 WHERE name='lifecycle_v1'",
            [encoded(&phase)?],
        )
        .map_err(|e| e.to_string())?;
        let (seq, next) = event(
            &tx,
            "session.state_changed",
            None,
            &json!({"phase":phase["phase"],
            "generation":generation,"exit_id":phase["exit_id"],"supersedes":previous["exit_id"]}),
        )?;
        let outcome =
            json!({"status":"accepted","session":phase,"state_revision":next,"event_seq":seq});
        command_commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }

    pub fn update_exit(
        &self,
        exit_id: &str,
        generation: u64,
        phase_name: &str,
        notice: Option<&Value>,
    ) -> Result<Option<Value>, String> {
        if let Some(writer)=&self.workflow.writer {let (exit_id,phase_name,notice)=
            (exit_id.to_owned(),phase_name.to_owned(),notice.cloned());
            return writer.call(move|s|s.update_exit(&exit_id,generation,&phase_name,notice.as_ref()));}
        if !matches!(phase_name, "draining" | "paused") {
            return Err("Invalid drain phase".into());
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let previous = meta(&tx, "lifecycle_v1")?;
        if previous["exit_id"] != exit_id
            || previous["generation"] != generation
            || !["exit-requested", "draining"]
                .iter()
                .any(|s| previous["phase"] == *s)
        {
            return Ok(None);
        }
        if previous["phase"] == phase_name && previous["notice"] == json!(notice) {
            return Ok(Some(previous));
        }
        let mut updated = previous.clone();
        updated["phase"] = phase_name.into();
        updated["notice"] = json!(notice);
        tx.execute(
            "UPDATE session_metadata SET value=?1 WHERE name='lifecycle_v1'",
            [encoded(&updated)?],
        )
        .map_err(|e| e.to_string())?;
        event(
            &tx,
            "session.state_changed",
            None,
            &json!({"phase":phase_name,"generation":generation,
            "exit_id":exit_id,"notice":notice}),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Some(updated))
    }

    pub fn record_external(&self, body: &Value, outcome: &Value) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {let (body,outcome)=(body.clone(),outcome.clone());
            return writer.call(move|s|s.record_external(&body,&outcome));}
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(prior) = command_prior(&tx, body)? {
            return Ok(prior);
        }
        command_commit(&tx, body, outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome.clone())
    }

    pub fn pending_coordinator(&self) -> Result<Option<Value>, String> {
        let conn = self.workflow.connection()?;
        let raw:Option<(String,String,String,String)>=conn.query_row("SELECT call_id,task_id,packet_sha256,status FROM session_coordinator_calls WHERE status='reserved' ORDER BY rowid LIMIT 1",[],
            |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?;
        Ok(raw.map(|(call,task,hash,status)|json!({"call_id":call,"task_id":task,"packet_sha256":hash,"status":status})))
    }

    pub fn stop_coordinator(&self, task: &str, status: &str, detail: &str) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let (task,status,detail)=
            (task.to_owned(),status.to_owned(),detail.to_owned());
            return writer.call(move|s|s.stop_coordinator(&task,&status,&detail));}
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE session_coordinator_calls SET status=?1 WHERE task_id=?2",
            params![status, task],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("UPDATE session_tasks SET status='blocked',state_revision=state_revision+1 WHERE task_id=?1",[task]).map_err(|e|e.to_string())?;
        event(
            &tx,
            "task.state_changed",
            Some(task),
            &json!({"task_id":task,"status":"blocked",
            "detail":detail,"coordinator_execution":status}),
        )?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn reopen_prelaunch_coordinator(&self,task:&str)->Result<(),String>{
        if let Some(writer)=&self.workflow.writer {let task=task.to_owned();
            return writer.call(move|s|s.reopen_prelaunch_coordinator(&task));}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e|e.to_string())?;
        let current:String=tx.query_row("SELECT status FROM session_tasks WHERE task_id=?1",
            [task],|row|row.get(0)).map_err(|e|e.to_string())?;
        if current!="blocked"{return Err("Only a blocked prelaunch coordinator can reopen".into());}
        let changed=tx.execute("UPDATE session_coordinator_calls SET status='reserved' WHERE task_id=?1 AND status='pre-send-refusal'",
            [task]).map_err(|e|e.to_string())?;
        if changed!=1{return Err("Coordinator stop is not a proven prelaunch refusal".into());}
        tx.execute("UPDATE session_tasks SET status='queued',state_revision=state_revision+1 WHERE task_id=?1",
            [task]).map_err(|e|e.to_string())?;
        event(&tx,"task.state_changed",Some(task),&json!({"task_id":task,"status":"queued",
            "detail":"Explicit proven-no-launch infrastructure retry; original failure retained"}))?;
        tx.commit().map_err(|e|e.to_string())
    }

    pub fn block_unbound(&self, task: &str, detail: &str) -> Result<(), String> {
        if let Some(writer)=&self.workflow.writer {let (task,detail)=(task.to_owned(),detail.to_owned());
            return writer.call(move|s|s.block_unbound(&task,&detail));}
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let Some(row) = task_row(&tx, task)? else {
            return Err("Task not found".into());
        };
        if !["queued", "proposed", "ready"]
            .iter()
            .any(|s| row["status"] == *s)
            || !row["run_id"].is_null()
        {
            return Err("Only an undispatched task can be blocked".into());
        }
        tx.execute("UPDATE session_tasks SET status='blocked',state_revision=state_revision+1 WHERE task_id=?1",[task])
            .map_err(|e|e.to_string())?;
        event(
            &tx,
            "task.state_changed",
            Some(task),
            &json!({"task_id":task,"status":"blocked","detail":detail}),
        )?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn checkpoint(&self) -> Result<Value, String> {
        if let Some(writer)=&self.workflow.writer {return writer.call(move|s|s.checkpoint());}
        let snapshot = self.snapshot()?;
        let conn = self.workflow.connection()?;
        let mut decisions = Vec::new();
        let mut pending = Vec::new();
        let mut stmt=conn.prepare("SELECT r.run_id,r.request_id,r.payload,a.payload FROM app_requests r LEFT JOIN app_responses a ON a.run_id=r.run_id AND a.request_id=r.request_id ORDER BY r.rowid DESC LIMIT 100")
            .map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (run, id, request, response) = row.map_err(|e| e.to_string())?;
            if let Some(response) = response {
                decisions.push(
                    json!({"run_id":run,"request_id":id,"record_sha256":sha256(response.as_bytes()),
                "record":format!("runs/{run}/artifacts/decision-{id}.json")}),
                );
            } else {
                pending.push(json!({"run_id":run,"request_id":id,"request_sha256":sha256(request.as_bytes())}));
            }
        }
        let mut context = Vec::new();
        let mut budgets = Vec::new();
        let mut stmt=conn.prepare("SELECT task_id,plan_json,plan_sha256 FROM session_admissions ORDER BY rowid DESC LIMIT 100")
            .map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (task, raw, hash) = row.map_err(|e| e.to_string())?;
            let plan = decoded(raw)?;
            context.push(json!({"task_id":task,"plan_sha256":hash,"selection":plan["context"],
                "selection_sha256":plan["context_snapshot_sha256"],
                "grants":plan["cross_imports"].as_array().into_iter().flatten().map(|g|json!({"grant_id":g["grant_id"],
                    "item_sha256":g["item_sha256"]})).collect::<Vec<_>>()}));
            budgets.push(json!({"task_id":task,"run_id":plan["run_id"],"max_worker_invocations":plan["max_worker_invocations"],
                "review_limit":plan["review_limit"]}));
        }
        let tasks = snapshot["tasks"]
            .as_array()
            .ok_or("invalid task snapshot")?
            .iter()
            .map(|t| {
                json!({"task_id":t["task_id"],
            "status":t["status"],"run_id":t["run_id"],"title":t["title"],"version":t["version"]})
            })
            .collect::<Vec<_>>();
        let mut artifacts=Vec::new();let mut open_findings=Vec::new();
        let mut seen=std::collections::BTreeSet::new();
        for task in snapshot["tasks"].as_array().ok_or("invalid task snapshot")?{
            let Some(run)=task["run_id"].as_str() else{continue;};
            if !seen.insert(run.to_owned()){continue;}
            if let Some(static_run)=self.static_snapshot(run)?{
                if !static_run["candidate"].is_null(){
                    // A broken candidate remains visible as retained evidence;
                    // checkpointing must still succeed so the issue can be inspected.
                    artifacts.push(json!({"run_id":run,"path":format!("runs/{run}/{}",
                        static_run["candidate"]["path"].as_str().ok_or("Static candidate path missing")?),
                        "sha256":static_run["candidate"]["sha256"]}));
                }
                for finding in static_run["findings"].as_array().into_iter().flatten(){
                    if finding["status"]!="addressed"{
                        open_findings.push(json!({"run_id":run,"finding_id":finding["id"],
                            "evidence_sha256":canonical_sha256(finding).map_err(|e|e.to_string())?}));
                    }
                }
                if static_run["status"]=="awaiting-input"{
                    pending.push(json!({"run_id":run,"request_id":static_run["pending"]["request_id"],
                        "manifest_sha256":static_run["pending"]["artifact_sha256"]}));
                }
                if !static_run["decision"].is_null(){
                    decisions.push(json!({"run_id":run,"request_id":static_run["decision"]["request_id"],
                        "manifest_sha256":static_run["decision"]["manifest_sha256"]}));
                }
                continue;
            }
            let state=self.workflow.snapshot(run)?["state"].clone();
            if let Some(findings)=state["findings"].as_object(){
                for (id,finding) in findings{
                    if finding["status"]=="open"{
                        open_findings.push(json!({"run_id":run,"finding_id":id,
                            "evidence_sha256":canonical_sha256(finding).map_err(|e|e.to_string())?}));
                    }
                }
            }
            for stage in state["completed"].as_array().into_iter().flatten(){
                let Some(stage)=stage.as_str() else{continue;};
                let manifest=self.workflow.run_dir(run)?.join("artifacts").join(format!("{stage}.json"));
                if !manifest.is_file(){continue;}
                let reference=read(&manifest)?["manifest"].clone();
                let Some(path)=reference["path"].as_str() else{continue;};
                let full=self.workflow.run_dir(run)?.join(path);
                let bytes=fs::read(&full).map_err(|e|e.to_string())?;
                if sha256(&bytes)!=reference["sha256"]{return Err("Checkpoint artifact integrity mismatch".into());}
                artifacts.push(json!({"run_id":run,"path":format!("runs/{run}/{path}"),
                    "sha256":reference["sha256"]}));
            }
        }
        let mut projection = json!({"schema_version":2,"project_key":self.key,"release_id":self.release,
            "state_revision":snapshot["state_revision"],"event_cursor":snapshot["cursor"],
            "session":snapshot["session"],"tasks":tasks,"decision_refs":decisions,"pending_questions":pending,
            "context_refs":context,"budget_refs":budgets,"artifact_refs":artifacts,"open_findings":open_findings,
            "finding_state_ref":"selected project's schema-2 workflow record and finding ledger",
            "next_actions":["Inspect pending task questions and reviews","Resume explicitly if exited"]});
        if self.conversation_native(){projection["conversation_handoff"]=self.conversation_handoff()?;}
        if self.development(){projection["development_context"]=self.development_context()?;}
        if canonical(&projection).map_err(|e| e.to_string())?.len() > 32768 {
            projection["decision_refs"] = json!([]);
            projection["context_refs"] = json!([]);
            projection["budget_refs"] = json!([]);
            projection["artifact_refs"] = json!([]);
            projection["open_findings"] = json!([]);
            projection["decision_query"] = "app_responses in selected project state.sqlite".into();
            projection["context_query"] =
                "session_admissions in selected project state.sqlite".into();
        }
        if canonical(&projection).map_err(|e| e.to_string())?.len() > 32768 {
            projection["tasks"] = json!([]);
            projection["task_query"] = "session_tasks in selected project state.sqlite".into();
        }
        let hash = canonical_sha256(&projection).map_err(|e| e.to_string())?;
        let filename = format!("{}-{}.json", snapshot["state_revision"], &hash[..16]);
        let root = self.workflow.state.join("session/checkpoints");
        write(&root.join(&filename), &projection, true)?;
        let pointer = json!({"schema_version":2,"project_key":self.key,"state_revision":snapshot["state_revision"],
            "file":filename,"sha256":hash});
        write(&root.join("latest.json"), &pointer, false)?;
        Ok(
            json!({"path":root.join(filename),"sha256":hash,"state_revision":snapshot["state_revision"]}),
        )
    }
}
