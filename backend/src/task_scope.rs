//! Project-owned immutable scope revisions and human decisions, never admissions.
use crate::{
    codec::{canonical_sha256, sha256},
    conversation_store::{commit, encode, event, prior, reject, valid_command},
    scope_contract::{self, Action, Proposal},
    session_store::Session,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
pub(crate) fn epoch(tx: &Transaction<'_>, task: Option<&str>) -> Result<u64, String> {
    let key = format!("scope_epoch_v1:{}", task.unwrap_or("new"));
    let raw: Option<String> = tx
        .query_row(
            "SELECT value FROM session_metadata WHERE name=?1",
            [key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    raw.map(|s| serde_json::from_str::<u64>(&s).map_err(|e| e.to_string()))
        .transpose()
        .map(|v| v.unwrap_or(0))
}
pub(crate) fn bump(tx: &Transaction<'_>, task: Option<&str>, reason: &str) -> Result<u64, String> {
    let next = epoch(tx, task)? + 1;
    tx.execute("INSERT INTO session_metadata(name,value) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET value=excluded.value",params![format!("scope_epoch_v1:{}",task.unwrap_or("new")),encode(&json!(next))?]).map_err(|e|e.to_string())?;
    if let Some(task) = task {
        tx.execute(
            "UPDATE task_proposal_heads SET decision_revision=?1 WHERE task_id=?2",
            params![i64::try_from(next).map_err(|e| e.to_string())?, task],
        )
        .map_err(|e| e.to_string())?;
        let changed=tx.execute("UPDATE session_tasks SET status='proposed',state_revision=state_revision+1 WHERE task_id=?1 AND status='scope-authorized'",[task]).map_err(|e|e.to_string())?;
        if changed > 0 {
            event(
                tx,
                "scope.authorization_superseded",
                Some(task),
                &json!({"reason":reason,"decision_revision":next}),
            )?;
        }
    }
    Ok(next)
}
pub(crate) fn head(tx: &Transaction<'_>, key: &str, task: &str) -> Result<Option<Value>, String> {
    let raw:Option<(i64,String,String,i64)>=tx.query_row("SELECT h.revision,r.sha256,r.record_json,h.decision_revision FROM task_proposal_heads h JOIN task_proposal_revisions r ON r.task_id=h.task_id AND r.revision=h.revision WHERE h.task_id=?1 AND r.project_key=?2",params![task,key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?;
    raw.map(|(rev,sha,raw,dep)|{
  let record:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
  if canonical_sha256(&record).map_err(|e|e.to_string())?!=sha{return Err("Proposal record integrity mismatch".into())}
  let auth:Option<String>=tx.query_row("SELECT record_json FROM task_scope_decisions WHERE task_id=?1 AND proposal_revision=?2 AND proposal_sha256=?3 AND decision_revision=?4 ORDER BY rowid DESC LIMIT 1",params![task,rev,sha,dep],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
  let authorized=auth.is_some()&&record["decision_revision"]==dep;
  Ok(json!({"base":{"task_id":task,"revision":rev,"sha256":sha},"record":record,"decision_revision":dep,
   "authorization_command_id":auth.as_ref().and_then(|v|serde_json::from_str::<Value>(v).ok()).map(|v|v["command_id"].clone()),"authorization":auth.map(|v|serde_json::from_str::<Value>(&v)).transpose().map_err(|e|e.to_string())?,
   "authorized":authorized,"needs_revision":record["decision_revision"]!=dep,"execution_available":false}))
 }).transpose()
}
fn questions(tx: &Transaction<'_>, task: Option<&str>) -> Result<Vec<Value>, String> {
    let mut s=tx.prepare("SELECT q.question_id,q.prompt,q.options_json,q.status,q.answer,q.answer_command_id,a.body_json FROM conversation_questions q JOIN conversation_actions a ON a.turn_id=q.turn_id WHERE a.kind='ask_question' AND json_extract(a.body_json,'$.target_task_id') IS ?1 ORDER BY q.rowid DESC LIMIT 16").map_err(|e|e.to_string())?;
    s.query_map([task],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,String>(6)?))).map_err(|e|e.to_string())?.map(|r|{
  let(id,prompt,options,status,answer,command,body)=r.map_err(|e|e.to_string())?;
  Ok(json!({"question_id":id,"prompt":prompt,"options":serde_json::from_str::<Value>(&options).map_err(|e|e.to_string())?,"status":status,"answer":answer,"answer_command_id":command,"answer_sha256":answer.as_ref().map(|s|sha256(s.as_bytes())),"source":serde_json::from_str::<Value>(&body).map_err(|e|e.to_string())?}))
 }).collect()
}
pub(crate) fn linked_tasks(tx:&Transaction<'_>,question:&str)->Result<Vec<String>,String>{
    let mut stmt=tx.prepare("SELECT DISTINCT h.task_id FROM task_proposal_heads h JOIN task_proposal_revisions r ON r.task_id=h.task_id AND r.revision=h.revision JOIN json_each(r.record_json,'$.context_refs.questions') q WHERE json_extract(q.value,'$.question_id')=?1").map_err(|e|e.to_string())?;
    stmt.query_map([question],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}
pub(crate) fn capture(tx: &Transaction<'_>, key: &str, task: Option<&str>) -> Result<Value, String> {
    let current = task.map(|id| head(tx, key, id)).transpose()?.flatten();
    if task.is_some() && current.is_none() {
        return Err("Selected proposal not found in this project".into());
    }
    let watermark: i64 = tx
        .query_row(
            "SELECT coalesce(max(seq),0) FROM conversation_messages WHERE role='user'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let mut selected = std::collections::BTreeMap::new();
    if let Some(v) = &current {
        if let Some(items) = v["record"]["context_refs"]["questions"].as_array() {
            for q in items {
                let mut current=q.clone();let id=q["question_id"].as_str().unwrap_or("");
                let row:Option<(String,Option<String>,Option<String>)>=tx.query_row("SELECT status,answer,answer_command_id FROM conversation_questions WHERE question_id=?1 AND project_key=?2",params![id,key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
                if let Some((status,answer,command))=row{current["status"]=json!(status);current["answer"]=json!(answer);current["answer_command_id"]=json!(command);current["answer_sha256"]=json!(answer.as_ref().map(|a|sha256(a.as_bytes())));}
                selected.insert(id.to_owned(), current);
            }
        }
    }
    let continuous:bool=tx.query_row("SELECT exists(SELECT 1 FROM session_metadata WHERE name='dialogue_pause_v1')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    for q in if continuous{vec![]}else{questions(tx, task)?} {
        selected.insert(q["question_id"].as_str().unwrap_or("").to_owned(), q);
    }
    if continuous {for q in selected.values_mut(){q["revision"]=json!(tx.query_row("SELECT revision FROM dialogue_questions WHERE question_id=?1",[q["question_id"].as_str()],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?);}}
    let refs = current
        .as_ref()
        .map(|v| v["record"]["context_refs"]["selected_context_refs"].clone())
        .unwrap_or(json!([]));
    Ok(
        json!({"target_task_id":task,"base":current.as_ref().map(|v|v["base"].clone()),"proposal":current,
  "decision_revision":epoch(tx,task)?,"questions":selected.into_values().collect::<Vec<_>>(),"selected_context_refs":refs,"user_watermark":watermark}),
    )
}
fn expected(payload: &Value, current: &Value) -> bool {
    payload["base"] == current["base"]
        && payload["decision_revision"] == current["decision_revision"]
        && payload["authorization_command_id"] == current["authorization_command_id"]
}
pub(crate) fn save(
    tx: &Transaction<'_>,
    key: &str,
    task: &str,
    p: &Proposal,
    context: &Value,
    source: Value,
    rationale: &str,
) -> Result<Value, String> {
    let old = head(tx, key, task)?;
    let content = serde_json::to_value(p).map_err(|e| e.to_string())?;
    let dep = epoch(tx, Some(task))?;
    let refs = json!({"questions":context["questions"],"selected_context_refs":context["selected_context_refs"]});
    if let Some(old) = &old {
        if old["record"]["proposal"] == content
            && old["record"]["context_refs"] == refs
            && old["record"]["decision_revision"] == dep
        {
            return Ok(json!({"base":old["base"],"unchanged":true}));
        }
    }
    let rev = old
        .as_ref()
        .and_then(|v| v["base"]["revision"].as_u64())
        .unwrap_or(0)
        + 1;
    let record = json!({"schema_version":1,"project_key":key,"task_id":task,"revision":rev,
  "parent_sha256":old.as_ref().map(|v|v["base"]["sha256"].clone()),"proposal":content,"decision_revision":dep,
  "context_refs":refs,"source":source,"change_rationale":rationale});
    let sha = canonical_sha256(&record).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO task_proposal_revisions VALUES(?1,?2,?3,?4,?5)",
        params![
            task,
            i64::try_from(rev).map_err(|e| e.to_string())?,
            key,
            sha,
            encode(&record)?
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO task_proposal_heads VALUES(?1,?2,?3) ON CONFLICT(task_id) DO UPDATE SET revision=excluded.revision,decision_revision=excluded.decision_revision",params![task,i64::try_from(rev).map_err(|e|e.to_string())?,i64::try_from(dep).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
    if old.is_some() {
        event(
            tx,
            "scope.authorization_superseded",
            Some(task),
            &json!({"reason":"Saved proposal changed","prior":old.as_ref().unwrap()["base"]}),
        )?;
        tx.execute("UPDATE session_tasks SET version=?1,state_revision=state_revision+1,status='proposed',title=?2 WHERE task_id=?3",params![i64::try_from(rev).map_err(|e|e.to_string())?,p.title,task]).map_err(|e|e.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO session_tasks VALUES(?1,1,1,'proposed',?2,?3,NULL)",
            params![task, p.title, p.objective],
        )
        .map_err(|e| e.to_string())?;
    }
    event(
        tx,
        "scope.proposed",
        Some(task),
        &json!({"revision":rev,"execution_available":false}),
    )?;
    Ok(json!({"base":{"task_id":task,"revision":rev,"sha256":sha},"unchanged":false}))
}
impl Session {
    pub fn scope_enabled(&self) -> bool {
        crate::execution::read(&self.workflow.state.join("state-contract.json"))
            .ok()
            .is_some_and(|v| matches!(v["state_schema"].as_u64(),Some(6|7|8|9)) && v["project_key"] == self.key)
    }
    pub fn conversation_native(&self) -> bool {
        self.conversation_chat() || self.scope_enabled()
    }
    pub fn scope_schema(&self) -> Result<(), String> {
        self.workflow.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS task_proposal_revisions(task_id TEXT NOT NULL,revision INTEGER NOT NULL,project_key TEXT NOT NULL,sha256 TEXT NOT NULL,record_json TEXT NOT NULL,PRIMARY KEY(task_id,revision));
   CREATE TABLE IF NOT EXISTS task_proposal_heads(task_id TEXT PRIMARY KEY,revision INTEGER NOT NULL,decision_revision INTEGER NOT NULL);
   CREATE TABLE IF NOT EXISTS task_scope_decisions(command_id TEXT PRIMARY KEY,task_id TEXT NOT NULL,proposal_revision INTEGER NOT NULL,proposal_sha256 TEXT NOT NULL,decision_revision INTEGER NOT NULL,record_json TEXT NOT NULL);") .map_err(|e|e.to_string())
    }
    pub fn scope_detail(&self, id: &str) -> Result<Option<Value>, String> {
        let mut c = self.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let Some(mut v) = head(&tx, &self.key, id)? else {
            return Ok(None);
        };
        v["questions"] = capture(&tx,&self.key,Some(id))?["questions"].clone();
        v["pending_turns"]=json!(tx.query_row("SELECT count(*) FROM conversation_turns t JOIN conversation_messages m ON m.message_id=t.message_id WHERE t.status IN ('queued','reserved') AND m.task_id=?1",[id],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?);
        if self.continuous(){v["pending_turns"]=json!(tx.query_row("SELECT count(*) FROM dialogue_inputs d JOIN conversation_turns t USING(turn_id) JOIN dialogue_subjects s USING(subject_id) WHERE s.task_id=?1 AND d.request_revision=1 AND t.status IN ('queued','reserved')",[id],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?);}
        let mut s = tx
            .prepare("SELECT record_json FROM task_scope_decisions WHERE task_id=?1 ORDER BY rowid DESC LIMIT 32")
            .map_err(|e| e.to_string())?;
        v["decision_history"] = json!(
            s.query_map([id], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .map(
                    |r| serde_json::from_str::<Value>(&r.map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())
                )
                .collect::<Result<Vec<_>, _>>()?
        );
        if self.development(){v["execution_available"]=json!(true);v["operating_policy"]=self.development_record("workflow",id)?.map(|w|w["policy"].clone()).unwrap_or(crate::request_routing::policy(self,id)?);v["development"]=self.development_snapshot()?;}
        Ok(Some(v))
    }
    pub fn scope_capture(&self, task: Option<&str>) -> Result<Value, String> {
        let mut c = self.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        capture(&tx, &self.key, task)
    }
    pub fn scope_command(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer) = &self.workflow.writer {
            let b = body.clone();
            return writer.call(move |s| s.scope_command(&b));
        }
        let action = body["action"].as_str().unwrap_or("");
        if !self.scope_enabled() {
            return Ok(reject(
                "capability_unavailable",
                "Scope authority requires schema 6",
            ));
        }
        if let Err(e) = valid_command(body, action) {
            return Ok(e);
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(old) = prior(&tx, body)? {
            return Ok(old);
        }
        let p = &body["payload"];
        let task = p["base"]["task_id"].as_str().unwrap_or("");
        let Some(current) = head(&tx, &self.key, task)? else {
            return Ok(reject("not_found", "Proposal not in selected project"));
        };
        if !expected(p, &current) {
            return Ok(reject(
                "stale_proposal",
                "Saved scope or decisions changed; your draft is retained",
            ));
        }
        let value = match action {
            "save_proposal_revision" => {
                let proposal: Proposal =
                    serde_json::from_value(p["proposal"].clone()).map_err(|e| e.to_string())?;
                proposal.validate()?;
                let rationale = p["rationale"].as_str().unwrap_or("");
                if rationale.trim().is_empty() || rationale.len() > 4096 {
                    return Ok(reject(
                        "invalid_request",
                        "Supply a bounded change rationale",
                    ));
                }
                let context = capture(&tx, &self.key, Some(task))?;
                save(
                    &tx,
                    &self.key,
                    task,
                    &proposal,
                    &context,
                    json!({"kind":"human","command_id":body["command_id"]}),
                    rationale,
                )?
            }
            "authorize_scope" => {
                let conversational_source=crate::coordination::approval_source(&tx,self,body)?;
                if self.development() && p["operating_policy_sha256"]!=canonical_sha256(&crate::request_routing::policy(self,task)?).map_err(|e|e.to_string())? {return Ok(reject("stale_policy","Operating policy changed; review it again"));}

                let proposal: Proposal =
                    serde_json::from_value(current["record"]["proposal"].clone())
                        .map_err(|e| e.to_string())?;
                let pending:i64=tx.query_row("SELECT count(*) FROM conversation_turns t JOIN conversation_messages m ON m.message_id=t.message_id WHERE t.status IN ('queued','reserved') AND (m.task_id=?1 OR m.task_id IS NULL)",[task],|r|r.get(0)).map_err(|e|e.to_string())?;
                let pending=if self.continuous(){tx.query_row("SELECT count(*) FROM dialogue_inputs d JOIN conversation_turns t USING(turn_id) JOIN dialogue_subjects s USING(subject_id) WHERE t.status IN ('queued','reserved') AND d.request_revision=1 AND s.task_id=?1",[task],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?}else{pending};
                let deferred_missing=self.continuous()&&capture(&tx,&self.key,Some(task))?["questions"].as_array().is_some_and(|a|a.iter().any(|q|q["status"]=="deferred"&&!proposal.unresolved_decisions.iter().any(|s|Some(s.as_str())==q["prompt"].as_str())));
                if deferred_missing || !proposal.eligible()
                    || current["needs_revision"] == true
                    || capture(&tx,&self.key,Some(task))?["questions"].as_array().ok_or("Missing linked questions")?
                        .iter()
                        .any(|q| q["status"] == "open")
                    || pending > 0
                {
                    return Ok(reject(
                        "scope_not_ready",
                        "Complete scope/answers and settle pending coordinator changes first",
                    ));
                }
                let unresolved = canonical_sha256(&json!(proposal.unresolved_decisions))
                    .map_err(|e| e.to_string())?;
                if p["acknowledged_unresolved_sha256"] != unresolved {
                    return Ok(reject(
                        "acknowledgement_required",
                        "Acknowledge this exact unresolved list",
                    ));
                }
                let dep = current["decision_revision"]
                    .as_u64()
                    .ok_or("Invalid decision revision")?;
                let mut record = json!({"schema_version":1,"project_key":self.key,"task_id":task,"proposal_revision":current["base"]["revision"],"proposal_sha256":current["base"]["sha256"],"decision_revision":current["decision_revision"],"command_id":body["command_id"],"acknowledged_unresolved_sha256":unresolved,"context_epoch":dep,"execution_available":self.development(),"operating_policy":if self.development(){crate::request_routing::policy(self,task)?}else{Value::Null}});
                if let Some(source)=conversational_source{record["conversational_source"]=source;}
                if self.development(){crate::development::approve(&tx,self,task,&current,&record)?;}
                tx.execute(
                    "INSERT INTO task_scope_decisions VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        body["command_id"].as_str(),
                        task,
                        current["base"]["revision"].as_i64(),
                        current["base"]["sha256"].as_str(),
                        current["decision_revision"].as_i64(),
                        encode(&record)?
                    ],
                )
                .map_err(|e| e.to_string())?;
                tx.execute("UPDATE session_tasks SET status='scope-authorized',state_revision=state_revision+1 WHERE task_id=?1",[task]).map_err(|e|e.to_string())?;
                event(
                    &tx,
                    "scope.authorized",
                    Some(task),
                    &json!({"revision":current["base"]["revision"],"label":if self.development(){"Scope and operating policy authorized · development enabled"}else{"Scope authorized · execution unavailable"}}),
                )?;
                record
            }
            _ => {
                return Ok(reject(
                    "outside_increment",
                    "Only scope save and authorization are available",
                ));
            }
        };
        let mut outcome = json!({"status":"accepted","scope":value,"execution_available":self.development()});
        if self.continuous(){let current:String=tx.query_row("SELECT value FROM session_metadata WHERE name='state_revision'",[],|r|r.get(0)).map_err(|e|e.to_string())?;outcome["state_revision"]=serde_json::from_str(&current).map_err(|e|e.to_string())?;}
        commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }
    pub fn scope_question_context(&self, id: &str) -> Result<Value, String> {
        let mut c = self.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let raw:String=tx.query_row("SELECT a.body_json FROM conversation_actions a JOIN conversation_questions q ON q.turn_id=a.turn_id WHERE q.question_id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
        let link: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        Ok(
            json!({"subject_id":if self.continuous(){json!(tx.query_row("SELECT subject_id FROM dialogue_questions WHERE question_id=?1",[id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?)}else{Value::Null},"question_revision":if self.continuous(){tx.query_row("SELECT revision FROM dialogue_questions WHERE question_id=?1",[id],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?}else{0},"linked_task_ids":linked_tasks(&tx,id)?,"rationale":link["action"]["payload"]["rationale"],"target_task_id":link["target_task_id"],"decision_revision":epoch(&tx,link["target_task_id"].as_str())?,"continuation_available":tx.query_row("SELECT status='answered' FROM conversation_actions WHERE turn_id=(SELECT turn_id FROM conversation_questions WHERE question_id=?1)",[id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())?}),
        )
    }
    pub fn scope_answer(&self, body: &Value) -> Result<Value, String> {
        if self.continuous(){return self.dialogue_command(body)}
        if let Some(w) = &self.workflow.writer {
            let b = body.clone();
            return w.call(move |s| s.scope_answer(&b));
        }
        if let Err(e) = valid_command(body, "answer_conversation") {
            return Ok(e);
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(old) = prior(&tx, body)? {
            return Ok(old);
        }
        let id = body["payload"]["question_id"].as_str().unwrap_or("");
        let answer = body["payload"]["answer"].as_str().unwrap_or("");
        if answer.trim().is_empty() || answer.len() > 16384 {
            return Ok(reject(
                "invalid_request",
                "Answer missing or exceeds bounds",
            ));
        }
        let row:Option<(String,String)>=tx.query_row("SELECT q.status,a.body_json FROM conversation_questions q JOIN conversation_actions a ON a.turn_id=q.turn_id WHERE q.question_id=?1 AND q.project_key=?2",params![id,self.key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        let Some((status, raw)) = row else {
            return Ok(reject("question_unavailable", "No open question"));
        };
        if status != "open" {
            return Ok(reject("question_unavailable", "Question already answered"));
        }
        let link: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let target = link["target_task_id"].as_str();
        if body["payload"]["decision_revision"] != epoch(&tx, target)? {
            return Ok(reject("stale_proposal", "Question context changed"));
        }
        tx.execute("UPDATE conversation_questions SET status='answered',answer=?1,answer_command_id=?2 WHERE question_id=?3",params![answer,body["command_id"].as_str(),id]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE conversation_actions SET status='answered' WHERE turn_id=(SELECT turn_id FROM conversation_questions WHERE question_id=?1)",[id]).map_err(|e|e.to_string())?;
        let dependencies=linked_tasks(&tx,id)?;
        bump(&tx, target, "Relevant human answer changed")?;
        for task in dependencies{if Some(task.as_str())!=target{bump(&tx,Some(&task),"Linked human answer changed")?;}}

        event(
            &tx,
            "conversation.answer",
            target,
            &json!({"question_id":id,"answer":answer,"answer_sha256":sha256(answer.as_bytes()),"continuation_required":true}),
        )?;
        let outcome = json!({"status":"accepted","question_id":id,"continuation_required":true});
        commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }
    /// Called inside conversation_submit's transaction: once-only continuation, no charge yet.
    pub(crate) fn scope_continuation(
        &self,
        tx: &Transaction<'_>,
        body: &Value,
    ) -> Result<Option<Value>, String> {
        let Some(id) = body["payload"]["answer_question_id"].as_str() else {
            return Ok(None);
        };
        let row:Option<(String,String,Option<String>)>=tx.query_row("SELECT a.status,a.body_json,q.answer FROM conversation_actions a JOIN conversation_questions q ON q.turn_id=a.turn_id WHERE q.question_id=?1 AND q.project_key=?2",params![id,self.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
        let Some((status, raw, answer)) = row else {
            return Ok(Some(reject(
                "question_unavailable",
                "Saved answer unavailable",
            )));
        };
        let link: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if status != "answered"
            || (body["payload"]["task_id"] != link["target_task_id"] && !body["payload"]["task_id"].as_str().is_some_and(|task|linked_tasks(tx,id).is_ok_and(|ids|ids.iter().any(|v|v==task))))
            || body["payload"]["text"] != answer.unwrap_or_default()
        {
            return Ok(Some(reject(
                "continuation_unavailable",
                "Saved answer already submitted or changed",
            )));
        }
        tx.execute("UPDATE conversation_actions SET status='continued' WHERE turn_id=(SELECT turn_id FROM conversation_questions WHERE question_id=?1)",[id]).map_err(|e|e.to_string())?;
        Ok(None)
    }
    pub fn scope_packet(&self,next:&Value,profile:&Value,profile_sha:&Value)->Result<Value,String>{
        if self.continuous(){self.dialogue_packet(next,profile,profile_sha)}else{self.scope_packet_base(next,profile,profile_sha)}
    }
    pub(crate) fn scope_packet_base(&self,next:&Value,profile:&Value,profile_sha:&Value)->Result<Value,String>{
        let mut c=self.workflow.connection()?;let tx=c.transaction().map_err(|e|e.to_string())?;
        let turn=next["turn_id"].as_str().ok_or("Missing turn")?;
        let message=next["message_id"].as_str().ok_or("Missing message")?;
        let through:i64=tx.query_row("SELECT seq FROM conversation_messages WHERE message_id=?1 AND project_key=?2",params![message,self.key],|r|r.get(0)).map_err(|e|e.to_string())?;
        let mut scope=capture(&tx,&self.key,next["task_id"].as_str())?;scope["user_watermark"]=json!(through);
        let binding=tx.query_row("SELECT generation,provider,model,effort FROM conversation_bindings WHERE singleton=1",[],|r|Ok(json!({"generation":r.get::<_,i64>(0)?,"provider":r.get::<_,String>(1)?,"model":r.get::<_,String>(2)?,"effort":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?;
        let mut budgets=serde_json::Map::new();let mut stmt=tx.prepare("SELECT bucket,limit_count,charged FROM conversation_budgets").map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?))).map_err(|e|e.to_string())?{let(k,l,c)=row.map_err(|e|e.to_string())?;let n=c+if k=="conversation"{1}else{0};budgets.insert(k,json!({"limit":l,"charged":n,"remaining":l-n,"as_of":"packet reservation, including this held call"}));}drop(stmt);
        let mut stmt=tx.prepare("SELECT message_id,seq,role,text,content_sha256 FROM conversation_messages WHERE seq<?1 ORDER BY seq DESC LIMIT 6").map_err(|e|e.to_string())?;
        let mut messages=stmt.query_map([through],|r|Ok(json!({"message_id":r.get::<_,String>(0)?,"seq":r.get::<_,i64>(1)?,"role":r.get::<_,String>(2)?,"text":r.get::<_,String>(3)?,"text_sha256":r.get::<_,String>(4)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;messages.reverse();
        loop{
            let mut refs=vec![json!(format!("message:{message}"))];for m in &messages{refs.push(json!(format!("message:{}",m["message_id"].as_str().unwrap_or(""))));}
            for q in scope["questions"].as_array().ok_or("Missing selected questions")?{refs.push(json!(format!("question:{}",q["question_id"].as_str().unwrap_or(""))));}
            let context=json!({"scope":scope,"messages":messages,"project_key":self.key,"binding":binding,"budgets":budgets,"evidence_allowlist":refs});
            let packet=json!({"schema_version":2,"operation_kind":"conversation","operation_id":turn,"turn_id":turn,"run_id":turn,"stage":"conversation","role":"coordinator","provider":"claude","model":profile["model"],"project_key":self.key,"message":next["text"],"binding":binding,"execution":profile,"authorization_profile_sha256":profile_sha,"selected_context":context,"context_sha256":canonical_sha256(&context).map_err(|e|e.to_string())?});
            let limit=if profile["profile"]=="development-profiles-v1"{crate::development_context::MEMORY_LIMIT}else{crate::native_contract::PACKET_LIMIT};
            if serde_json::to_vec(&packet).map_err(|e|e.to_string())?.len()<=limit{return Ok(packet)}
            if messages.is_empty(){return Err(format!("Selected mandatory scope/decisions exceed the {limit}-byte preparation bound; no call reserved"))}messages.remove(0);
        }
    }
    pub(crate) fn scope_reservation_valid(&self,tx:&Transaction<'_>,packet:&Value)->Result<bool,String>{
        let frozen=&packet["selected_context"]["scope"];
        if self.continuous(){let task:Option<String>=tx.query_row("SELECT task_id FROM dialogue_subjects WHERE subject_id=?1",[frozen["subject_id"].as_str()],|r|r.get(0)).map_err(|e|e.to_string())?;if json!(task)!=frozen["target_task_id"]{return Ok(false)}for q in frozen["questions"].as_array().ok_or("Missing questions")?{let rev:i64=tx.query_row("SELECT revision FROM dialogue_questions WHERE question_id=?1",[q["question_id"].as_str()],|r|r.get(0)).map_err(|e|e.to_string())?;if json!(rev)!=q["revision"]{return Ok(false)}}}
        let now=capture(tx,&self.key,frozen["target_task_id"].as_str())?;
        let charged:i64=tx.query_row("SELECT charged FROM conversation_budgets WHERE bucket='conversation'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        Ok(now["base"]==frozen["base"]&&now["decision_revision"]==frozen["decision_revision"]&&now["proposal"]["authorization_command_id"]==frozen["proposal"]["authorization_command_id"]&&packet["selected_context"]["budgets"]["conversation"]["charged"]==charged+1)
    }
    pub fn scope_checkpoint(&self)->Result<Value,String>{
        let mut c=self.workflow.connection()?;let tx=c.transaction().map_err(|e|e.to_string())?;let mut stmt=tx.prepare("SELECT task_id FROM task_proposal_heads ORDER BY task_id").map_err(|e|e.to_string())?;
        let ids=stmt.query_map([],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;drop(stmt);
        let mut refs=vec![];for id in ids{if let Some(h)=head(&tx,&self.key,&id)?{refs.push(json!({"base":h["base"],"decision_revision":h["decision_revision"],"authorization_command_id":h["authorization_command_id"],"authorized":h["authorized"],"execution_available":false}));}}
        Ok(json!({"schema_version":1,"project_key":self.key,"scope_refs":refs,"execution_available":false}))
    }
    pub fn scope_apply(
        &self,
        turn: &str,
        packet_sha: &str,
        result: &Value,
    ) -> Result<Value, String> {
        if let Some(w) = &self.workflow.writer {
            let (t, p, r) = (turn.to_owned(), packet_sha.to_owned(), result.clone());
            return w.call(move |s| s.scope_apply(&t, &p, &r));
        }
        if self.continuous(){return self.dialogue_apply(turn,packet_sha,result)}
        let parsed = scope_contract::validate(result)?;
        if parsed.schema_version!=2 {return Err("Scope profile requires response version 2".into())}
        let digest = canonical_sha256(result).map_err(|e| e.to_string())?;
        let packet = crate::execution::read(
            &self
                .workflow
                .state
                .join("session/conversation")
                .join(turn)
                .join("packet.json"),
        )?;
        if canonical_sha256(&packet).map_err(|e| e.to_string())? != packet_sha {
            return Ok(reject("packet_mismatch", "Frozen packet changed"));
        }
        let context = &packet["selected_context"]["scope"];
        for r in &parsed.evidence_refs {
            if !packet["selected_context"]["evidence_allowlist"]
                .as_array()
                .is_some_and(|a| a.contains(&json!(r)))
            {
                return Err("Evidence reference outside frozen packet".into());
            }
        }
        if let Some(Action::UpsertProposal { base, .. }) = &parsed.action {
            if serde_json::to_value(base).map_err(|e| e.to_string())? != context["base"] {
                return Err("Model proposal base differs from frozen target".into());
            }
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let row:Option<(String,Option<String>,Option<String>,i64)>=tx.query_row("SELECT status,packet_sha256,result_sha256,binding_generation FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",params![turn,self.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?;
        let Some((status, expected, old, generation)) = row else {
            return Ok(reject("not_found", "Unknown turn"));
        };
        if status == "completed" && old.as_deref() == Some(&digest) {
            return Ok(json!({"status":"accepted","replayed":true}));
        }
        if status != "reserved" || expected.as_deref() != Some(packet_sha) {
            return Ok(reject("stale_turn", "Result is not for a reserved turn"));
        }
        let actual: i64 = tx
            .query_row(
                "SELECT generation FROM conversation_bindings WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let target = context["target_task_id"].as_str();
        let now = capture(&tx, &self.key, target)?;
        let stale = generation != actual
            || now["base"] != context["base"]
            || now["decision_revision"] != context["decision_revision"]
            || now["user_watermark"] != context["user_watermark"]
            || now["proposal"]["authorization_command_id"]
                != context["proposal"]["authorization_command_id"];
        let action_id = format!("s-{}", &sha256(turn.as_bytes())[..24]);
        if let Some(action) = parsed.action {
            let body = json!({"action":action,"target_task_id":target,"packet_sha256":packet_sha,"result_sha256":digest});
            let kind = if matches!(action, Action::AskQuestion { .. }) {
                "ask_question"
            } else {
                "upsert_proposal"
            };
            tx.execute(
                "INSERT INTO conversation_actions VALUES(?1,?2,?3,?4,?5)",
                params![
                    action_id,
                    turn,
                    kind,
                    encode(&body)?,
                    if stale { "stale" } else { "materialized" }
                ],
            )
            .map_err(|e| e.to_string())?;
            if stale {
                event(
                    &tx,
                    "scope.suggestion_stale",
                    target,
                    &json!({"turn_id":turn,"reason":"Based on an earlier scope; suggestion not applied"}),
                )?;
            } else {
                match action {
                    Action::AskQuestion {
                        prompt,
                        rationale,
                        options,
                    } => {
                        let count: i64 = tx
                            .query_row(
                                "SELECT count(*) FROM conversation_questions WHERE status='open'",
                                [],
                                |r| r.get(0),
                            )
                            .map_err(|e| e.to_string())?;
                        if count >= 8 {
                            return Err("Eight open questions already exist".into());
                        }
                        let id = format!("q-{}", &sha256(turn.as_bytes())[..24]);
                        tx.execute("INSERT INTO conversation_questions VALUES(?1,1,?2,?3,?4,?5,'open',NULL,NULL)",params![id,self.key,turn,prompt,encode(&json!(options))?]).map_err(|e|e.to_string())?;
                        bump(&tx, target, "Clarification requested")?;
                        tx.execute(
                            "UPDATE conversation_actions SET status='pending' WHERE action_id=?1",
                            [action_id],
                        )
                        .map_err(|e| e.to_string())?;
                        event(
                            &tx,
                            "conversation.question",
                            target,
                            &json!({"question_id":id,"question":prompt,"reason":rationale}),
                        )?;
                    }
                    Action::SuggestQuestionDisposition { .. } => return Err("Question suggestions require schema 7".into()),
                    Action::ProposeSteering { .. } => return Err("Conversational steering requires its negotiated source context".into()),
                    Action::RetrieveEvidence { .. } => return Err("Evidence retrieval requires its negotiated frozen grant".into()),
                    Action::UpsertProposal { proposal, .. } => {
                        let task = target
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("t-{}", &sha256(turn.as_bytes())[..24]));
                        if target.is_none() {
                            // Bind prior new-scope answers to this new task without rewriting their evidence.
                            tx.execute(
                                "INSERT INTO session_metadata(name,value) VALUES(?1,?2)",
                                params![
                                    format!("scope_epoch_v1:{task}"),
                                    encode(&context["decision_revision"])?
                                ],
                            )
                            .map_err(|e| e.to_string())?;
                        }
                        save(
                            &tx,
                            &self.key,
                            &task,
                            &proposal,
                            context,
                            json!({"kind":"coordinator","turn_id":turn,"packet_sha256":packet_sha,"result_sha256":digest}),
                            "Coordinator suggestion",
                        )?;
                    }
                }
            }
        }
        if !parsed.assistant_text.is_empty() {
            let id = format!("a-{}", &sha256(turn.as_bytes())[..24]);
            let (seq, _) = event(
                &tx,
                "conversation.message",
                target,
                &json!({"message_id":id,"role":"assistant","text":parsed.assistant_text,"turn_id":turn}),
            )?;
            tx.execute(
                "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'assistant',?4,?5,?6,?7)",
                params![
                    id,
                    self.key,
                    seq,
                    parsed.assistant_text,
                    sha256(parsed.assistant_text.as_bytes()),
                    target,
                    turn
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute("UPDATE conversation_turns SET status='completed',result_sha256=?1,result_json=?2 WHERE turn_id=?3",params![digest,encode(result)?,turn]).map_err(|e|e.to_string())?;
        tx.execute(
            "DELETE FROM session_metadata WHERE name='claude_chat_stream_v1'",
            [],
        )
        .map_err(|e| e.to_string())?;
        event(
            &tx,
            "conversation.turn_completed",
            target,
            &json!({"turn_id":turn,"action_disposition":if stale{"stale"}else{"applied"}}),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(json!({"status":"accepted","action_disposition":if stale{"stale"}else{"applied"}}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    fn fixture() -> (TempDir, Session) {
        let root = TempDir::new().unwrap();
        let project = root.path().join("project");
        let state = project.join(".waywr/state");
        std::fs::create_dir_all(&state).unwrap();
        crate::execution::write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":6,"execution_host":"systemd-user-v1","project_key":"scope-a"}),true).unwrap();
        let p = json!({"schema_version":1,"profile":"claude-proposal-v1","mode":"mock-cli","provider":"claude","model":"claude-sonnet-5-5","effort":"medium","call_limit":6,"binary":std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/claude-conversation/mock-cli.py"),"auth_home":project,"auth_config_dir":project});
        crate::execution::write(
            &project.join(".waywr/coordinator.json"),
            &p,
            true,
        )
        .unwrap();
        let s = Session::open(&project, &state, "scope-a", "scope-release").unwrap();
        s.conversation_schema().unwrap();
        s.workflow
            .connection()
            .unwrap()
            .execute(
                "UPDATE session_metadata SET value=?1 WHERE name='claude_chat_authorization_v1'",
                [encode(
                    &json!({"authorized":true,"profile_sha256":canonical_sha256(&p).unwrap()}),
                )
                .unwrap()],
            )
            .unwrap();
        (root, s)
    }
    fn proposal() -> Value {
        json!({"title":"Weather page","objective":"Fixed sample data","deliverables":["Static page"],"in_scope":["Local HTML"],"out_of_scope":["Network"],"constraints":[],"acceptance_criteria":["Sample temperatures visible"],"unresolved_decisions":["Color choice"]})
    }
    fn turn(s: &Session, id: &str, task: Option<&str>) -> (String, String, Value) {
        let rev = s.conversation_snapshot().unwrap()["state_revision"].clone();
        let r=s.conversation_submit(&json!({"command_id":id,"action":"submit_message","expected_state_revision":rev,"payload":{"text":"Natural unchanged message","task_id":task}})).unwrap();
        assert_eq!(r["status"], "accepted", "{r}");
        let t = r["turn_id"].as_str().unwrap().to_owned();
        let next=s.conversation_next_queued().unwrap().unwrap();let profile=crate::claude_chat::profile(&s.project).unwrap();
        let p=s.scope_packet(&next,&profile,&json!(canonical_sha256(&profile).unwrap())).unwrap();
        crate::execution::write(
            &s.workflow
                .state
                .join("session/conversation")
                .join(&t)
                .join("packet.json"),
            &p,
            true,
        )
        .unwrap();
        let r = s.conversation_reserve(&t, &p, "conversation").unwrap();
        assert_eq!(r["status"], "accepted", "{r}");
        (t, r["packet_sha256"].as_str().unwrap().to_owned(), p)
    }
    fn seed(s: &Session) -> String {
        let (t, h, p) = turn(s, "first", None);
        let r = json!({"schema_version":2,"assistant_text":"Scope ready","evidence_refs":[],"action":{"kind":"upsert_proposal","payload":{"base":p["selected_context"]["scope"]["base"],"proposal":proposal()}}});
        assert_eq!(s.scope_apply(&t, &h, &r).unwrap()["status"], "accepted");
        assert_eq!(s.scope_apply(&t, &h, &r).unwrap()["replayed"], true);
        s.snapshot().unwrap()["tasks"][0]["task_id"]
            .as_str()
            .unwrap()
            .into()
    }
    fn command(s: &Session, task: &str, id: &str, action: &str) -> Value {
        let d = s.scope_detail(task).unwrap().unwrap();
        json!({"command_id":id,"action":action,"payload":{"base":d["base"],"decision_revision":d["decision_revision"],"authorization_command_id":d["authorization_command_id"],"acknowledged_unresolved_sha256":canonical_sha256(&d["record"]["proposal"]["unresolved_decisions"]).unwrap(),"proposal":d["record"]["proposal"],"rationale":"Operator scope refinement"}})
    }
    #[test]
    fn immutable_scope_exact_authority_and_replay() {
        let (_r, s) = fixture();
        let task = seed(&s);
        let original = s.scope_detail(&task).unwrap().unwrap();
        let mut bad = command(&s, &task, "bad", "authorize_scope");
        bad["payload"]["acknowledged_unresolved_sha256"] = json!("wrong");
        assert_eq!(
            s.scope_command(&bad).unwrap()["code"],
            "acknowledgement_required"
        );
        let auth = command(&s, &task, "auth", "authorize_scope");
        let outcome = s.scope_command(&auth).unwrap();
        assert_eq!(outcome["status"], "accepted");
        assert!(
            s.scope_detail(&task).unwrap().unwrap()["authorized"]
                .as_bool()
                .unwrap()
        );
        let no_op = command(&s, &task, "no-op", "save_proposal_revision");
        assert_eq!(s.scope_command(&no_op).unwrap()["scope"]["unchanged"], true);
        assert_eq!(s.scope_detail(&task).unwrap().unwrap()["authorized"], true);
        let mut edit = command(&s, &task, "edit", "save_proposal_revision");
        edit["payload"]["proposal"]["objective"] = json!("Changed objective");
        assert_eq!(s.scope_command(&edit).unwrap()["status"], "accepted");
        let after = s.scope_detail(&task).unwrap().unwrap();
        assert_eq!(after["base"]["revision"], 2);
        assert_eq!(after["authorized"], false);
        assert_eq!(after["decision_history"].as_array().unwrap().len(), 1);
        assert_eq!(s.scope_command(&auth).unwrap(), outcome);
        let stored: String = s
            .workflow
            .connection()
            .unwrap()
            .query_row(
                "SELECT record_json FROM task_proposal_revisions WHERE task_id=?1 AND revision=1",
                [&task],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&stored).unwrap(),
            original["record"]
        );
        let mut conflict = auth.clone();
        conflict["payload"]["base"]["revision"] = json!(9);
        assert_eq!(
            s.scope_command(&conflict).unwrap()["code"],
            "idempotency_conflict"
        );
        assert!(s.snapshot().unwrap()["tasks"][0]["run_id"].is_null());
        assert_eq!(
            s.admit(
                &json!({"command_id":"forge","payload":{"task_id":task}}),
                &json!({})
            )
            .unwrap()["code"],
            "outside_increment"
        );
        assert!(s.workflow.create(&json!({})).is_err());
    }
    #[test]
    fn stale_result_is_completed_inert_and_adopted_once() {
        let (_r, s) = fixture();
        let task = seed(&s);
        let (t, h, p) = turn(&s, "revise", Some(&task));
        let mut edit = command(&s, &task, "local", "save_proposal_revision");
        edit["payload"]["proposal"]["constraints"] = json!(["Operator change"]);
        s.scope_command(&edit).unwrap();
        let r = json!({"schema_version":2,"assistant_text":"Older suggestion","evidence_refs":[],"action":{"kind":"upsert_proposal","payload":{"base":p["selected_context"]["scope"]["base"],"proposal":proposal()}}});
        assert_eq!(
            s.scope_apply(&t, &h, &r).unwrap()["action_disposition"],
            "stale"
        );
        assert_eq!(s.scope_apply(&t, &h, &r).unwrap()["replayed"], true);
        assert_eq!(
            s.scope_detail(&task).unwrap().unwrap()["record"]["proposal"]["constraints"],
            json!(["Operator change"])
        );
        assert_eq!(
            s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            2
        );
    }
    #[test]
    fn exhausted_answer_edit_and_authorize_do_not_spend_calls() {
        let (_r, s) = fixture();
        let task = seed(&s);
        let (t, h, _) = turn(&s, "ask", Some(&task));
        let r = json!({"schema_version":2,"assistant_text":"Choice?","evidence_refs":[],"action":{"kind":"ask_question","payload":{"prompt":"Unit?","rationale":"Scope choice","options":["C","F"]}}});
        s.scope_apply(&t, &h, &r).unwrap();
        s.workflow
            .connection()
            .unwrap()
            .execute(
                "UPDATE conversation_budgets SET charged=6 WHERE bucket='conversation'",
                [],
            )
            .unwrap();
        let q = s.conversation_snapshot().unwrap()["questions"][0]["question_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let ctx = s.scope_question_context(&q).unwrap();
        let answer = json!({"command_id":"answer","action":"answer_conversation","payload":{"question_id":q,"answer":"C","decision_revision":ctx["decision_revision"]}});
        let saved = s.conversation_answer(&answer).unwrap();
        assert_eq!(saved["status"], "accepted");
        assert_eq!(s.conversation_answer(&answer).unwrap(), saved);
        assert_eq!(
            s.scope_command(&command(&s, &task, "premature", "authorize_scope"))
                .unwrap()["code"],
            "scope_not_ready"
        );
        s.scope_command(&command(
            &s,
            &task,
            "attach-answer",
            "save_proposal_revision",
        ))
        .unwrap();
        assert_eq!(
            s.scope_command(&command(&s, &task, "auth", "authorize_scope"))
                .unwrap()["status"],
            "accepted"
        );
        assert_eq!(
            s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            6
        );
        assert_eq!(
            s.conversation_snapshot().unwrap()["budgets"]["task"]["limit"],
            0
        );
        assert_eq!(
            s.conversation_snapshot().unwrap()["budgets"]["consultation"]["limit"],
            0
        );
    }
    #[test]
    fn two_clients_same_base_one_wins_and_restart_preserves_records() {
        let (_r, s) = fixture();
        let task = seed(&s);
        let a = command(&s, &task, "a", "authorize_scope");
        let mut b = a.clone();
        b["command_id"] = json!("b");
        let p = s.project.clone();
        let state = s.workflow.state.clone();
        let writer = crate::db_writer::DbWriter::start(
            p.clone(),
            state.clone(),
            s.key.clone(),
            s.release.clone(),
        )
        .unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut jobs = vec![];
        for cmd in [a, b] {
            let w = writer.clone();
            let gate = barrier.clone();
            jobs.push(std::thread::spawn(move || {
                gate.wait();
                w.call(move |s| s.scope_command(&cmd)).unwrap()
            }));
        }
        let results = jobs
            .into_iter()
            .map(|j| j.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            results.iter().filter(|r| r["status"] == "accepted").count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| r["code"] == "stale_proposal")
                .count(),
            1
        );
        writer.shutdown().unwrap();
        let again = Session::open_existing(&p, &state, &s.key, &s.release).unwrap();
        again.conversation_schema().unwrap();
        assert_eq!(
            again.scope_detail(&task).unwrap().unwrap()["authorized"],
            true
        );
        assert_eq!(
            again.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            1
        );
    }
    #[test]
    fn forged_reference_base_and_partial_results_cannot_commit() {
        let (_r, s) = fixture();
        let (t, h, _) = turn(&s, "bad", None);
        let mut r = json!({"schema_version":2,"assistant_text":"Bad","evidence_refs":["foreign:secret"],"action":null});
        assert!(s.scope_apply(&t, &h, &r).is_err());
        r["evidence_refs"] = json!([]);
        r["action"] = json!({"kind":"upsert_proposal","payload":{"base":{"task_id":"foreign","revision":1,"sha256":"bad"},"proposal":proposal()}});
        assert!(s.scope_apply(&t, &h, &r).is_err());
        assert!(
            s.snapshot().unwrap()["tasks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn failed_application_rolls_back_then_same_receipt_adopts_once(){
        let (_r,s)=fixture();let(t,h,p)=turn(&s,"fault",None);
        let result=json!({"schema_version":2,"assistant_text":"Scope","evidence_refs":[],"action":{"kind":"upsert_proposal","payload":{"base":p["selected_context"]["scope"]["base"],"proposal":proposal()}}});
        s.workflow.connection().unwrap().execute_batch("CREATE TRIGGER injected_completion_failure BEFORE UPDATE OF status ON conversation_turns WHEN NEW.status='completed' BEGIN SELECT RAISE(ABORT,'fixture fault before commit'); END;").unwrap();
        assert!(s.scope_apply(&t,&h,&result).is_err());assert!(s.snapshot().unwrap()["tasks"].as_array().unwrap().is_empty());assert_eq!(s.conversation_snapshot().unwrap()["turns"][0]["status"],"reserved");
        s.workflow.connection().unwrap().execute_batch("DROP TRIGGER injected_completion_failure").unwrap();
        assert_eq!(s.scope_apply(&t,&h,&result).unwrap()["status"],"accepted");assert_eq!(s.scope_apply(&t,&h,&result).unwrap()["replayed"],true);assert_eq!(s.snapshot().unwrap()["tasks"].as_array().unwrap().len(),1);assert_eq!(s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],1);
    }
    #[test]
    fn packet_is_bounded_targeted_and_reservation_refuses_changed_dependencies(){
        let (_a,s)=fixture();let task=seed(&s);let (_b,other)=fixture();other.workflow.connection().unwrap().execute("UPDATE conversation_messages SET text='UNRELATED_PROJECT_SENTINEL'",[]).unwrap();
        let rev=s.conversation_snapshot().unwrap()["state_revision"].clone();let r=s.conversation_submit(&json!({"command_id":"bounded","action":"submit_message","expected_state_revision":rev,"payload":{"text":"Refine it","task_id":task}})).unwrap();
        // Large optional previous messages must be omitted rather than exceed the packet.
        s.workflow.connection().unwrap().execute("UPDATE conversation_messages SET text=?1 WHERE role='assistant'",["x".repeat(100000)]).unwrap();
        let next=s.conversation_next_queued().unwrap().unwrap();let profile=crate::claude_chat::profile(&s.project).unwrap();let packet=s.scope_packet(&next,&profile,&json!(canonical_sha256(&profile).unwrap())).unwrap();let bytes=crate::codec::canonical(&packet).unwrap();assert!(bytes.len()<=96*1024);assert!(!String::from_utf8_lossy(&bytes).contains("UNRELATED_PROJECT_SENTINEL"));assert_eq!(packet["selected_context"]["budgets"]["conversation"]["charged"],2);
        let mut edit=command(&s,&task,"change","save_proposal_revision");edit["payload"]["proposal"]["constraints"]=json!(["New constraint"]);s.scope_command(&edit).unwrap();
        assert_eq!(s.conversation_reserve(r["turn_id"].as_str().unwrap(),&packet,"conversation").unwrap()["code"],"stale_context");assert_eq!(s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],1);
    }

    #[test]
    fn scope_snapshot_event_pages_and_queued_messages_remain_bounded(){
        let (_r,s)=fixture();
        for i in 0..6{let rev=s.conversation_snapshot().unwrap()["state_revision"].clone();assert_eq!(s.conversation_submit(&json!({"command_id":format!("large-{i}"),"action":"submit_message","expected_state_revision":rev,"payload":{"text":"x".repeat(32768)}})).unwrap()["status"],"accepted");}
        let rev=s.conversation_snapshot().unwrap()["state_revision"].clone();assert_eq!(s.conversation_submit(&json!({"command_id":"overflow","action":"submit_message","expected_state_revision":rev,"payload":{"text":"Beyond pending allowance"}})).unwrap()["code"],"budget_exhausted");
        let snapshot=s.conversation_snapshot().unwrap();assert!(serde_json::to_vec(&snapshot).unwrap().len()<512*1024);let page=s.events(0,64).unwrap();assert!(serde_json::to_vec(&page).unwrap().len()<300*1024);assert!(page["next_seq"].as_u64().unwrap()>0);
        assert_eq!(s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],0);
    }

    #[test]
    fn inherited_open_question_blocks_authorization_and_answer_invalidates_linked_scope(){
        let (_r,s)=fixture();let(t,h,_)=turn(&s,"ask-first",None);s.scope_apply(&t,&h,&json!({"schema_version":2,"assistant_text":"Question","evidence_refs":[],"action":{"kind":"ask_question","payload":{"prompt":"Which unit?","rationale":"Scope depends on it","options":["C","F"]}}})).unwrap();
        let task=seed(&s);let scope=s.scope_detail(&task).unwrap().unwrap();assert_eq!(scope["questions"][0]["status"],"open");assert_eq!(s.scope_command(&command(&s,&task,"premature","authorize_scope")).unwrap()["code"],"scope_not_ready");
        let q=s.conversation_snapshot().unwrap()["questions"][0].clone();assert!(q["scope"]["linked_task_ids"].as_array().unwrap().contains(&json!(task)));
        s.scope_answer(&json!({"command_id":"answer-linked","action":"answer_conversation","payload":{"question_id":q["question_id"],"answer":"C","decision_revision":q["scope"]["decision_revision"]}})).unwrap();
        let scope=s.scope_detail(&task).unwrap().unwrap();assert_eq!(scope["needs_revision"],true);assert_eq!(scope["questions"][0]["status"],"answered");s.scope_command(&command(&s,&task,"refresh-decision","save_proposal_revision")).unwrap();assert_eq!(s.scope_command(&command(&s,&task,"authorize","authorize_scope")).unwrap()["status"],"accepted");
    }

    #[test]
    fn discussion_append_ignores_progress_revision_but_retains_dedupe_budget_and_target_checks(){
        let (_r,s)=fixture();let body=json!({"command_id":"append","action":"submit_message","expected_state_revision":99999,"payload":{"text":"Unchanged natural input"}});
        let r=s.conversation_submit(&body).unwrap();assert_eq!(r["status"],"accepted");assert_eq!(s.conversation_submit(&body).unwrap(),r);assert!(s.snapshot().unwrap()["tasks"].as_array().unwrap().is_empty());assert_eq!(s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],0);
        let mut foreign=body;foreign["command_id"]=json!("foreign");foreign["payload"]["task_id"]=json!("unrelated");assert_eq!(s.conversation_submit(&foreign).unwrap()["code"],"task_not_found");
    }

}
