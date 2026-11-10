//! Schema-7 interaction policy. Existing project writer, records and host remain authoritative.
use crate::{
    codec::{canonical_sha256, sha256},
    conversation_store::{commit, encode, event, metadata, prior, reject, valid_command},
    scope_contract::{self, Action},
    session_store::Session,
    task_scope,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
fn pause(tx: &Transaction<'_>) -> Result<Value, String> {
    metadata(tx, "dialogue_pause_v1")
}
fn set_pause(tx: &Transaction<'_>, p: &Value) -> Result<(), String> {
    tx.execute(
        "UPDATE session_metadata SET value=?1 WHERE name='dialogue_pause_v1'",
        [encode(p)?],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
fn question(tx: &Transaction<'_>, key: &str, id: &str) -> Result<Option<Value>, String> {
    tx.query_row("SELECT q.prompt,q.status,q.answer,d.revision,d.subject_id,q.options_json,(SELECT body_json FROM conversation_actions a WHERE a.turn_id=q.turn_id AND a.kind='ask_question') FROM conversation_questions q JOIN dialogue_questions d USING(question_id) WHERE q.question_id=?1 AND q.project_key=?2",params![id,key],|r|Ok(json!({"question_id":id,"prompt":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"answer":r.get::<_,Option<String>>(2)?,"revision":r.get::<_,i64>(3)?,"subject_id":r.get::<_,String>(4)?,"options":serde_json::from_str::<Value>(&r.get::<_,String>(5)?).unwrap_or(Value::Null),"rationale":serde_json::from_str::<Value>(&r.get::<_,String>(6)?).unwrap_or(Value::Null)["action"]["payload"]["rationale"]}))).optional().map_err(|e|e.to_string())
}
fn ack_valid(p: &Value, ack: &Value) -> bool {
    ack["generation"] == p["generation"]
        && ack["outcome_sha256"] == p["outcome_sha256"]
        && !ack["outcome_sha256"].is_null()
}
fn ack(tx: &Transaction<'_>, body: &Value, p: &Value) -> Result<(), String> {
    let record = json!({"schema_version":1,"command_id":body["command_id"],"generation":p["generation"],"outcome_sha256":p["outcome_sha256"]});
    tx.execute(
        "INSERT INTO dialogue_acknowledgements VALUES(?1,?2)",
        params![body["command_id"].as_str(), encode(&record)?],
    )
    .map_err(|e| e.to_string())?;
    let mut next = p.clone();
    next["active"] = json!(false);
    set_pause(tx, &next)?;
    event(tx, "conversation.recovery_acknowledged", None, &record)?;
    Ok(())
}
pub(crate) fn stop_tx(
    tx: &Transaction<'_>,
    turn: &str,
    kind: &str,
    proof: &Value,
    reason: &str,
) -> Result<Value, String> {
    let digest = hash(proof)?;
    let existing: Option<String> = tx
        .query_row(
            "SELECT record_json FROM dialogue_outcomes WHERE turn_id=?1",
            [turn],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(old) = existing {
        let old: Value = serde_json::from_str(&old).map_err(|e| e.to_string())?;
        return if old["proof_sha256"] == digest {
            Ok(json!({"status":"accepted","replayed":true}))
        } else {
            Ok(reject("outcome_conflict", "Outcome evidence changed"))
        };
    }
    let configuration=kind=="configuration"&&proof["provider_selection_v1"]==true&&(proof["no_provider_process_spawned"]==true||proof["no_model_request_sent"]==true);
    let recovery = (matches!(kind, "format" | "interrupted")||configuration)
        && proof["settled"] == true
        && proof["capabilities_valid"] == true
        && proof["known_completion"] == true;
    let old = pause(tx)?;
    let generation = old["generation"]
        .as_i64()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("Pause generation exhausted")?;
    let record = json!({"schema_version":1,"turn_id":turn,"category":kind,"proof":proof,"proof_sha256":digest,"recoverable":recovery,"generation":generation,"reason":reason});
    let sha = hash(&record)?;
    tx.execute(
        "INSERT INTO dialogue_outcomes VALUES(?1,?2,?3)",
        params![turn, sha, encode(&record)?],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("UPDATE conversation_turns SET status=?1,reason=?2,result_sha256=?3 WHERE turn_id=?4 AND status='reserved'",params![if kind=="interrupted"{"cancelled"}else if kind=="format"{"output-invalid"}else if configuration{"blocked"}else{"uncertain"},reason,sha,turn]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE dialogue_inputs SET held_generation=?1 WHERE turn_id IN (SELECT turn_id FROM conversation_turns WHERE status='queued')",[generation]).map_err(|e|e.to_string())?;
    tx.execute(
        "UPDATE conversation_turns SET status='held' WHERE status='queued'",
        [],
    )
    .map_err(|e| e.to_string())?;
    set_pause(
        tx,
        &json!({"schema_version":1,"active":true,"hard":!recovery,"generation":generation,"outcome_sha256":sha,"turn_id":turn,"reason":reason,"recoverable":recovery}),
    )?;
    event(
        tx,
        "conversation.turn_stopped",
        None,
        &json!({"turn_id":turn,"status":kind,"reason":reason,"recoverable":recovery,"scope_saved":false,"pause":pause(tx)?}),
    )?;
    Ok(
        json!({"status":"accepted","recoverable":recovery,"outcome_sha256":sha,"generation":generation}),
    )
}
impl Session {
    pub fn continuous(&self) -> bool {
        crate::execution::read(&self.workflow.state.join("state-contract.json"))
            .ok()
            .is_some_and(|v| matches!(v["state_schema"].as_u64(),Some(7|8|9)) && v["project_key"] == self.key)
    }
    pub(crate) fn dialogue_schema(&self) -> Result<(), String> {
        crate::live_input::schema(self)?;
        let c = self.workflow.connection()?;
        c.execute_batch("CREATE TABLE IF NOT EXISTS dialogue_subjects(subject_id TEXT PRIMARY KEY,task_id TEXT UNIQUE);
 CREATE TABLE IF NOT EXISTS dialogue_inputs(turn_id TEXT PRIMARY KEY,subject_id TEXT NOT NULL,request_revision INTEGER NOT NULL DEFAULT 0,interrupt_command_id TEXT,held_generation INTEGER);
 CREATE TABLE IF NOT EXISTS dialogue_questions(question_id TEXT PRIMARY KEY,subject_id TEXT NOT NULL,revision INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS dialogue_question_decisions(command_id TEXT PRIMARY KEY,question_id TEXT NOT NULL,revision INTEGER NOT NULL,record_json TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS dialogue_outcomes(turn_id TEXT PRIMARY KEY,sha256 TEXT NOT NULL,record_json TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS dialogue_acknowledgements(command_id TEXT PRIMARY KEY,record_json TEXT NOT NULL);") .map_err(|e|e.to_string())?;
        c.execute("INSERT OR IGNORE INTO session_metadata VALUES('dialogue_pause_v1',?1)",[encode(&json!({"schema_version":1,"generation":0,"active":false,"hard":false,"outcome_sha256":null}))?]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn dialogue_state(&self) -> Result<Value, String> {
        let mut c = self.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let p = pause(&tx)?;
        let mut q=tx.prepare("SELECT t.turn_id,t.status,m.text,m.message_id,d.subject_id,d.interrupt_command_id,d.request_revision,s.task_id FROM dialogue_inputs d JOIN conversation_turns t USING(turn_id) JOIN conversation_messages m ON m.message_id=t.message_id JOIN dialogue_subjects s USING(subject_id) WHERE t.status IN ('queued','held','reserved') ORDER BY m.seq LIMIT 16").map_err(|e|e.to_string())?;
        let mut rows=q.query_map([],|r|Ok(json!({"turn_id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"text":r.get::<_,String>(2)?.chars().take(4096).collect::<String>(),"message_id":r.get::<_,String>(3)?,"subject_id":r.get::<_,String>(4)?,"interrupt_requested":r.get::<_,Option<String>>(5)?.is_some(),"request_revision":r.get::<_,bool>(6)?,"task_id":r.get::<_,Option<String>>(7)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        if self.development() {
            for row in rows.iter_mut().filter(|row| row["status"] == "reserved") {
                let at: Option<f64> = tx.query_row("SELECT h.at FROM session_events e JOIN request_history_events h USING(seq) WHERE e.type='conversation.turn_reserved' AND json_extract(e.body,'$.turn_id')=?1 ORDER BY e.seq DESC LIMIT 1", [row["turn_id"].as_str()], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
                if let Some(at) = at { row["started_at"] = json!(at); }
            }
        }
        rows.extend(crate::live_input::rows(&tx)?);
        let mut st = tx
            .prepare(
                "SELECT subject_id,task_id FROM dialogue_subjects ORDER BY rowid DESC LIMIT 16",
            )
            .map_err(|e| e.to_string())?;
        let subjects=st.query_map([],|r|Ok(json!({"subject_id":r.get::<_,String>(0)?,"task_id":r.get::<_,Option<String>>(1)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        Ok(json!({"schema_version":1,"pause":p,"queue":rows,"subjects":subjects,"live_input_contract":if crate::provider_selection::enabled(self){1}else{0}}))
    }
    pub(crate) fn dialogue_submit(&self, body: &Value) -> Result<Value, String> {
        if let Some(w) = &self.workflow.writer {
            let b = body.clone();
            return w.call(move |s| s.dialogue_submit(&b));
        }
        if let Err(e) = valid_command(body, "submit_message") {
            return Ok(e);
        }
        if let Some(old) = self.command_prior(body)? {
            return Ok(old);
        }
        let readiness = self.chat_readiness()?;
        if !matches!(
            readiness["status"].as_str(),
            Some("available" | "running" | "recoverable")
        ) {
            return Ok(reject(
                "coordinator_unavailable",
                readiness["reason"]
                    .as_str()
                    .unwrap_or("Conversation unavailable"),
            ));
        }
        let p = &body["payload"];
        let text = p["text"].as_str().unwrap_or("");
        if text.trim().is_empty()
            || text.len() > 32768
            || p["intent"].as_str().is_some_and(|i| i != "discussion")
        {
            return Ok(reject(
                "invalid_request",
                "Send bounded coordinator discussion only",
            ));
        }
        let starting_commit=if self.development(){crate::development::git(&self.project,&["rev-parse","HEAD"]).ok()}else{None};
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(old) = prior(&tx, body)? {
            return Ok(old);
        }
        if metadata(&tx, "lifecycle_v1")?["phase"] != "running" {
            return Ok(reject(
                "session_paused",
                "Resume this project before sending",
            ));
        }
        let paused = pause(&tx)?;
        if paused["active"] == true {
            if paused["hard"] == true || !ack_valid(&paused, &p["acknowledged_outcome"]) {
                return Ok(reject(
                    "recovery_ack_required",
                    "Inspect the stopped call; explicitly send a new turn with its exact outcome acknowledgement",
                ));
            }
            ack(&tx, body, &paused)?;
        }
        let task = p["task_id"].as_str();
        if let Some(id) = task {
            if task_scope::head(&tx, &self.key, id)?.is_none() {
                return Ok(reject("task_not_found", "Scope not in this project"));
            }
        }
        let subject = p["subject_id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| {
                task.map(|t| format!("scope-{t}"))
                    .unwrap_or("discussion".into())
            });
        if subject.is_empty()
            || subject.len() > 128
            || !subject
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Ok(reject("invalid_subject", "Invalid discussion identity"));
        }
        tx.execute(
            "INSERT OR IGNORE INTO dialogue_subjects VALUES(?1,?2)",
            params![subject, task],
        )
        .map_err(|e| e.to_string())?;
        let bound: Option<String> = tx
            .query_row(
                "SELECT task_id FROM dialogue_subjects WHERE subject_id=?1",
                [&subject],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if task.is_some() && task != bound.as_deref() {
            return Ok(reject(
                "subject_conflict",
                "Discussion is bound to another scope",
            ));
        }
        let (eligible,total):(i64,i64)=tx.query_row("SELECT sum(status='queued'),count(*) FROM conversation_turns WHERE status IN ('queued','held','reserved')",[],|r|Ok((r.get::<_,Option<i64>>(0)?.unwrap_or(0),r.get(1)?))).map_err(|e|e.to_string())?;
        let remaining: i64 = tx
            .query_row(
                "SELECT limit_count-charged FROM conversation_budgets WHERE bucket='conversation'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if total >= 16 {
            return Ok(reject(
                "queue_full",
                "Sixteen pending/held messages; withdraw one or keep this draft",
            ));
        }
        if eligible >= remaining {
            return Ok(reject(
                "budget_exhausted",
                "Queued messages already use the remaining allowance; draft retained",
            ));
        }
        let id = format!(
            "m-{}",
            &sha256(body["command_id"].as_str().unwrap().as_bytes())[..24]
        );
        let turn = format!("c-{id}");
        let request=if self.development(){Some(crate::request_history::begin(&tx,self,&subject,text,starting_commit)?)}else{None};
        let (seq, rev) = event(
            &tx,
            "conversation.message",
            bound.as_deref(),
            &json!({"message_id":id,"role":"user","text":text,"subject_id":subject}),
        )?;
        if let Some(request)=request {crate::request_history::tag(&tx,seq,&request,"user")?;}
        tx.execute(
            "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'user',?4,?5,?6,?7)",
            params![
                id,
                self.key,
                seq,
                text,
                sha256(text.as_bytes()),
                bound,
                turn
            ],
        )
        .map_err(|e| e.to_string())?;
        let generation: i64 = tx
            .query_row("SELECT generation FROM conversation_bindings", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status,intent) VALUES(?1,1,?2,?3,?4,'queued','discussion')",params![turn,self.key,id,generation]).map_err(|e|e.to_string())?;
        tx.execute(
            "INSERT INTO dialogue_inputs(turn_id,subject_id,request_revision) VALUES(?1,?2,?3)",
            params![turn, subject, p["request_revision"] == true],
        )
        .map_err(|e| e.to_string())?;
        if crate::coordination::enabled(self)&&p["interaction"]["version"]==1{crate::coordination::save_turn_context(&tx,&turn,&p["interaction"])?;if p["interaction"]["provenance"]["imported"]!=true&&p["interaction"]["provenance"]["quoted"]!=true{crate::request_routing::save_source(&tx,self,&subject,&id,text)?;}}
        let out = json!({"status":"accepted","message_id":id,"turn_id":turn,"subject_id":subject,"queue_position":eligible+1,"state_revision":rev});
        commit(&tx, body, &out)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(out)
    }
    pub(crate) fn dialogue_next(&self) -> Result<Option<Value>, String> {
        if crate::provider_selection::enabled(self)&&crate::provider_selection::switch_pending(self){return Ok(None);}
        let c = self.workflow.connection()?;
        c.query_row("SELECT t.turn_id,m.message_id,m.text,t.binding_generation,d.subject_id,s.task_id FROM conversation_turns t JOIN conversation_messages m ON m.message_id=t.message_id JOIN dialogue_inputs d USING(turn_id) JOIN dialogue_subjects s USING(subject_id) WHERE t.status='queued' AND NOT EXISTS(SELECT 1 FROM conversation_turns WHERE status='reserved') ORDER BY m.seq LIMIT 1",[],|r|Ok(json!({"turn_id":r.get::<_,String>(0)?,"message_id":r.get::<_,String>(1)?,"text":r.get::<_,String>(2)?,"binding_generation":r.get::<_,i64>(3)?,"subject_id":r.get::<_,String>(4)?,"task_id":r.get::<_,Option<String>>(5)?}))).optional().map_err(|e|e.to_string())
    }
    pub(crate) fn dialogue_packet(
        &self,
        next: &Value,
        profile: &Value,
        profile_sha: &Value,
    ) -> Result<Value, String> {
        let mut packet = self.scope_packet_base(next, profile, profile_sha)?;
        if crate::coordination::enabled(self){packet["selected_context"]["interaction"]=crate::coordination::turn_context(self,next["turn_id"].as_str().unwrap_or(""))?;}
        let development_inputs=if self.development(){Some(self.development_discussion_inputs(next)?)}else{None};
        let mut c = self.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let mut scope = task_scope::capture(&tx, &self.key, next["task_id"].as_str())?;
        scope["subject_id"] = next["subject_id"].clone();
        let mut q=tx.prepare("SELECT q.question_id FROM conversation_questions q JOIN dialogue_questions d USING(question_id) WHERE d.subject_id=?1 ORDER BY q.rowid DESC LIMIT 16").map_err(|e|e.to_string())?;
        let ids = q
            .query_map([next["subject_id"].as_str()], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let questions = ids
            .iter()
            .map(|id| question(&tx, &self.key, id).map(|v| v.unwrap()))
            .collect::<Result<Vec<_>, _>>()?;
        scope["questions"] = json!(questions);
        packet["selected_context"]["scope"] = scope;
        let through: i64 = tx
            .query_row(
                "SELECT seq FROM conversation_messages WHERE message_id=?1",
                [next["message_id"].as_str()],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut stmt=tx.prepare("SELECT m.message_id,m.seq,m.role,m.text,m.content_sha256,t.status FROM conversation_messages m JOIN conversation_turns t ON t.turn_id=m.turn_id JOIN dialogue_inputs d ON d.turn_id=t.turn_id JOIN conversation_messages source ON source.message_id=t.message_id WHERE source.seq<?1 AND d.subject_id=?2 AND ((m.role='user' AND (t.status IN ('completed','output-invalid','cancelled') OR (?3 AND t.status='blocked'))) OR (m.role='assistant' AND t.status='completed')) ORDER BY source.seq DESC,m.role ASC LIMIT 6").map_err(|e|e.to_string())?;
        let mut messages=stmt.query_map(params![through,next["subject_id"].as_str(),self.development()],|r|{let mut v=json!({"message_id":r.get::<_,String>(0)?,"seq":r.get::<_,i64>(1)?,"role":r.get::<_,String>(2)?,"text":r.get::<_,String>(3)?,"text_sha256":r.get::<_,String>(4)?});if r.get::<_,String>(5)?=="blocked"{v["delivery_state"]=json!("blocked before reservation; saved user input, not an executed call");}Ok(v)}).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        messages.reverse();
        loop {
            packet["selected_context"]["last_outcome"] = pause(&tx)?;
            packet["selected_context"]["messages"] = json!(messages);
            packet["selected_context"]["current_message"] = json!({"message_id":next["message_id"],"text":next["text"],"text_sha256":sha256(next["text"].as_str().unwrap_or("").as_bytes())});
            let mut refs = vec![json!(format!(
                "message:{}",
                next["message_id"].as_str().unwrap()
            ))];
            for m in &messages {
                refs.push(json!(format!(
                    "message:{}",
                    m["message_id"].as_str().unwrap()
                )))
            }
            for q in &questions {
                refs.push(json!(format!(
                    "question:{}",
                    q["question_id"].as_str().unwrap()
                )))
            }
            packet["selected_context"]["evidence_allowlist"] = json!(refs);
            packet["context_sha256"] = json!(hash(&packet["selected_context"])?);
            if let Some(inputs)=&development_inputs{crate::development::fit_discussion_context(&mut packet,inputs)?;}
            if crate::codec::canonical(&packet)
                .map_err(|e| e.to_string())?
                .len()
                <= if profile["profile"]=="development-profiles-v1"{crate::development_context::MEMORY_LIMIT}else{profile["context_bytes"].as_u64().unwrap_or(96 * 1024) as usize}
            {
                return Ok(packet);
            }
            if messages.is_empty() {
                return Err(
                    "Mandatory discussion scope/decisions exceed the applicable preparation bound; no call reserved".into(),
                );
            }
            messages.remove(0);
        }
    }
    pub fn dialogue_command(&self, body: &Value) -> Result<Value, String> {
        if let Some(w) = &self.workflow.writer {
            let b = body.clone();
            return w.call(move |s| s.dialogue_command(&b));
        }
        let action = body["action"].as_str().unwrap_or("");
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
        let out = match action {
            "withdraw_queued" => {
                let id = p["turn_id"].as_str().unwrap_or("");
                let row:Option<(String,String)>=tx.query_row("SELECT t.status,m.text FROM conversation_turns t JOIN conversation_messages m ON m.message_id=t.message_id WHERE t.turn_id=?1 AND t.project_key=?2",params![id,self.key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
                let Some((state, text)) = row else {
                    return Ok(reject("not_found", "Message not in project"));
                };
                if !matches!(state.as_str(), "queued" | "held") {
                    return Ok(reject(
                        "already_dispatched",
                        "Message left queue; inspect/interrupt active call",
                    ));
                }
                tx.execute(
                    "UPDATE conversation_turns SET status='withdrawn' WHERE turn_id=?1",
                    [id],
                )
                .map_err(|e| e.to_string())?;
                event(&tx, "conversation.withdrawn", None, &json!({"turn_id":id}))?;
                json!({"status":"accepted","text":text,"call_charged":false})
            }
            "continue_held" => {
                let paused = pause(&tx)?;
                if paused["hard"] == true || !ack_valid(&paused, &p["acknowledged_outcome"]) {
                    return Ok(reject(
                        "stale_outcome",
                        "Acknowledge the exact recoverable outcome",
                    ));
                }
                let ids = p["turn_ids"]
                    .as_array()
                    .filter(|v| !v.is_empty() && v.len() <= 16)
                    .ok_or("Select bounded held messages")?;
                let available:i64=tx.query_row("SELECT limit_count-charged-(SELECT count(*) FROM conversation_turns WHERE status='queued') FROM conversation_budgets WHERE bucket='conversation'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
                if ids.len() as i64 > available {
                    return Ok(reject(
                        "budget_exhausted",
                        "Insufficient remaining allowance; held messages preserved",
                    ));
                }
                for id in ids {
                    let status:Option<String>=tx.query_row("SELECT status FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",params![id.as_str(),self.key],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
                    if status.as_deref() != Some("held") {
                        return Ok(reject("stale_queue", "Selected message is no longer held"));
                    }
                    tx.execute(
                        "UPDATE conversation_turns SET status='queued',binding_generation=(SELECT generation FROM conversation_bindings WHERE singleton=1) WHERE turn_id=?1",
                        [id.as_str()],
                    )
                    .map_err(|e| e.to_string())?;
                }
                ack(&tx, body, &paused)?;
                json!({"status":"accepted","released":ids.len()})
            }
            "cancel_conversation" => {
                let id = p["turn_id"].as_str().unwrap_or("");
                let changed=tx.execute("UPDATE dialogue_inputs SET interrupt_command_id=?1 WHERE turn_id=?2 AND interrupt_command_id IS NULL AND turn_id IN (SELECT turn_id FROM conversation_turns WHERE status='reserved' AND project_key=?3)",params![body["command_id"].as_str(),id,self.key]).map_err(|e|e.to_string())?;
                if changed == 0 {
                    return Ok(reject(
                        "no_active_call",
                        "Call completed or interruption already requested",
                    ));
                }
                tx.execute("INSERT INTO session_metadata VALUES('claude_chat_cancel_v1',?1) ON CONFLICT(name) DO UPDATE SET value=excluded.value",[encode(&json!({"turn_id":id,"command_id":body["command_id"]}))?]).map_err(|e|e.to_string())?;
                event(
                    &tx,
                    "conversation.cancel_requested",
                    None,
                    &json!({"turn_id":id,"actions_fenced":true}),
                )?;
                json!({"status":"accepted","turn_id":id,"actions_fenced":true})
            }
            "confirm_question_disposition" | "answer_conversation" => {
                let suggested = if action == "confirm_question_disposition" {
                    let raw:Option<String>=tx.query_row("SELECT body_json FROM conversation_actions WHERE action_id=?1 AND kind='suggest_question_disposition' AND status='pending'",[p["action_id"].as_str()],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
                    let Some(raw) = raw else {
                        return Ok(reject(
                            "suggestion_unavailable",
                            "Suggestion no longer available",
                        ));
                    };
                    let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                    if p["suggestion_sha256"] != hash(&v)? {
                        return Ok(reject("stale_suggestion", "Suggestion identity changed"));
                    }
                    v["action"]["payload"].clone()
                } else {
                    Value::Null
                };
                let id = if suggested.is_null() {
                    p["question_id"].as_str()
                } else {
                    suggested["question_id"].as_str()
                }
                .unwrap_or("");
                let Some(current) = question(&tx, &self.key, id)? else {
                    return Ok(reject("question_unavailable", "Question not in project"));
                };
                let expected = if suggested.is_null() {
                    &p["question_revision"]
                } else {
                    &suggested["question_revision"]
                };
                if *expected != current["revision"] {
                    return Ok(reject("stale_question", "Question decision changed"));
                }
                let disposition = if suggested.is_null() {
                    if crate::coordination::enabled(self)&&p["coordination_version"]==1&&p["disposition"]=="defer"{"defer"}else{"answer"}
                } else {
                    suggested["disposition"].as_str().unwrap_or("")
                };
                let answer = p["answer"]
                    .as_str()
                    .or_else(|| suggested["answer"].as_str())
                    .unwrap_or("");
                if answer.len() > 16384 || (disposition == "answer" && answer.trim().is_empty()) {
                    return Ok(reject("invalid_answer", "Supply a bounded answer"));
                }
                let next = current["revision"]
                    .as_i64()
                    .unwrap()
                    .checked_add(1)
                    .ok_or("Question revision exhausted")?;
                let record = json!({"schema_version":1,"command_id":body["command_id"],"project_key":self.key,"question_id":id,"revision":next,"disposition":disposition,"answer":answer,"suggestion":suggested});
                tx.execute(
                    "INSERT INTO dialogue_question_decisions VALUES(?1,?2,?3,?4)",
                    params![body["command_id"].as_str(), id, next, encode(&record)?],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "UPDATE dialogue_questions SET revision=?1 WHERE question_id=?2",
                    params![next, id],
                )
                .map_err(|e| e.to_string())?;
                tx.execute("UPDATE conversation_questions SET status=?1,answer=?2,answer_command_id=?3 WHERE question_id=?4",params![if disposition=="defer"{"deferred"}else{"answered"},answer,body["command_id"].as_str(),id]).map_err(|e|e.to_string())?;
                if !suggested.is_null() {
                    tx.execute(
                        "UPDATE conversation_actions SET status='confirmed' WHERE action_id=?1",
                        [p["action_id"].as_str()],
                    )
                    .map_err(|e| e.to_string())?;
                }
                for task in task_scope::linked_tasks(&tx, id)? {
                    task_scope::bump(
                        &tx,
                        Some(&task),
                        "Relevant formal question decision changed",
                    )?;
                }
                event(
                    &tx,
                    "conversation.answer",
                    None,
                    &json!({"question_id":id,"disposition":disposition,"answer":answer,"revision":next,"continuation_required":false}),
                )?;
                // A pre-proposal question still belongs to its discussion/request.
                if self.development(){
                    let seq:i64=tx.query_row("SELECT max(seq) FROM session_events",[],|r|r.get(0)).map_err(|e|e.to_string())?;
                    crate::coordination::tag_subject(&tx,seq,current["subject_id"].as_str().ok_or("Question subject missing")?,"user")?;
                }
                let automatic=crate::coordination::enabled(self)&&p["coordination_version"]==1&&disposition=="answer";
                if automatic{crate::coordination::answer_intent(&tx,self,&record,current["subject_id"].as_str().unwrap())?;}
                json!({"status":"accepted","question":record,"continuation_required":false,"continuation_saved":automatic,"subject_id":current["subject_id"]})
            }
            _ => {
                return Ok(reject(
                    "outside_increment",
                    "Unknown continuous interaction action",
                ));
            }
        };
        commit(&tx, body, &out)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(out)
    }
    pub fn dialogue_stop(
        &self,
        turn: &str,
        packet: &str,
        proof: &Value,
        reason: &str,
    ) -> Result<Value, String> {
        if let Some(w) = &self.workflow.writer {
            let (t, p, v, r) = (
                turn.to_owned(),
                packet.to_owned(),
                proof.clone(),
                reason.to_owned(),
            );
            return w.call(move |s| s.dialogue_stop(&t, &p, &v, &r));
        }
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let identity:Option<(String,Option<String>)>=tx.query_row("SELECT status,packet_sha256 FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",params![turn,self.key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        if identity
            .as_ref()
            .is_none_or(|(_, p)| p.as_deref() != Some(packet))
        {
            return Ok(reject("packet_mismatch", "Outcome packet changed"));
        }
        if identity
            .as_ref()
            .is_some_and(|(state, _)| state != "reserved")
            && !tx
                .query_row(
                    "SELECT exists(SELECT 1 FROM dialogue_outcomes WHERE turn_id=?1)",
                    [turn],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(|e| e.to_string())?
        {
            return Ok(reject(
                "stale_turn",
                "Completed turn cannot be stopped again",
            ));
        }
        let result = stop_tx(
            &tx,
            turn,
            proof["category"].as_str().unwrap_or("uncertain"),
            proof,
            reason,
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    }
    pub(crate) fn dialogue_apply(
        &self,
        turn: &str,
        packet_sha: &str,
        result: &Value,
    ) -> Result<Value, String> {
        if let Some(w) = &self.workflow.writer {
            let (t, p, r) = (turn.to_owned(), packet_sha.to_owned(), result.clone());
            return w.call(move |s| s.dialogue_apply(&t, &p, &r));
        }
        let parsed = scope_contract::validate(result)?;
        if parsed.schema_version != 3 {
            return Err("Continuous profile requires response v3".into());
        }
        let digest = hash(result)?;
        let root=self.workflow.state.join("session/conversation").join(turn);
        let packet=crate::execution::read(&if crate::provider_selection::enabled(self){root.join("packets").join(format!("{packet_sha}.json"))}else{root.join("packet.json")})?;
        if hash(&packet)? != packet_sha {
            return Err("Frozen packet integrity conflict".into());
        }
        crate::output::validate(result, &packet)?;
        let references=crate::output::conversation_references(&packet,&parsed.evidence_refs);
        // Read immutable evidence outside the writer transaction; a duplicate
        // completed result reuses the same immutable exchange, never a model call.
        let exchange=if let Some(Action::RetrieveEvidence{requests})=&parsed.action{
            let file=std::path::Path::new(packet["evidence_index"]["manifest_file"].as_str().ok_or("Missing frozen evidence manifest")?);
            let parent=file.parent().ok_or("Invalid evidence manifest")?;
            if !parent.canonicalize().map_err(|e|e.to_string())?.starts_with(self.workflow.state.canonicalize().map_err(|e|e.to_string())?){return Err("Evidence manifest leaves project state".into())}
            let prior=json!({"delivered":packet["evidence_receipts"],"unresolved_findings":packet["working_summary"]["unresolved_findings"],"scope_base":packet["scope_base"],"requirement_generation":packet["requirement_generation"]});
            Some(crate::development_context::retrieve(self,parent,&packet,requests,&prior,&parsed.assistant_text,&json!([]))?)
        }else{None};
        let context = &packet["selected_context"]["scope"];
        let subject = context["subject_id"].as_str().ok_or("Missing subject")?;
        let mut c = self.workflow.connection()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let history_start:i64=tx.query_row("SELECT coalesce(max(seq),0) FROM session_events",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let row:(String,Option<String>,Option<String>,i64)=tx.query_row("SELECT status,packet_sha256,result_sha256,binding_generation FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",params![turn,self.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
        if row.0 == "completed" && row.2.as_deref() == Some(&digest) {
            return Ok(json!({"status":"accepted","replayed":true}));
        }
        if row.0 != "reserved" || row.1.as_deref() != Some(packet_sha) {
            return Ok(reject("stale_turn", "Turn is not reserved for this result"));
        }
        let interrupt: Option<String> = tx
            .query_row(
                "SELECT interrupt_command_id FROM dialogue_inputs WHERE turn_id=?1",
                [turn],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if interrupt.is_some() {
            let proof = json!({"schema_version":1,"category":"interrupted","known_completion":true,"settled":true,"capabilities_valid":true,"result_sha256":digest,"packet_sha256":packet_sha,"interrupt_command_id":interrupt});
            let out = stop_tx(
                &tx,
                turn,
                "interrupted",
                &proof,
                "Interrupted; late action not applied",
            )?;
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(out);
        }
        let generation: i64 = tx
            .query_row("SELECT generation FROM conversation_bindings", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        let bound: Option<String> = tx
            .query_row(
                "SELECT task_id FROM dialogue_subjects WHERE subject_id=?1",
                [subject],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut stale = generation != row.3;
        let target = context["target_task_id"].as_str();
        let now = task_scope::capture(&tx, &self.key, target)?;
        if matches!(parsed.action, Some(Action::UpsertProposal { .. })) {
            stale |= bound.as_deref() != target
                || now["base"] != context["base"]
                || now["decision_revision"] != context["decision_revision"]
                || now["proposal"]["authorization_command_id"]
                    != context["proposal"]["authorization_command_id"];
        }
        let mut selected = vec![];
        for reference in parsed.evidence_refs.iter().filter(|_|parsed.action.is_some()) {
            if let Some(id) = reference.strip_prefix("question:") {
                let frozen = context["questions"]
                    .as_array()
                    .and_then(|a| a.iter().find(|q| q["question_id"] == id))
                    .ok_or("Question outside frozen context")?;
                let actual = question(&tx, &self.key, id)?.ok_or("Question missing")?;
                if actual["revision"] != frozen["revision"] {
                    stale = true
                }
                selected.push(actual)
            }
        }
        let action_id = format!("s-{}", &sha256(turn.as_bytes())[..24]);
        let mut disposition = "none";
        let mut saved = Value::Null;
        if let Some(action) = &parsed.action {
            let kind = match action {
                Action::AskQuestion { .. } => "ask_question",
                Action::UpsertProposal { .. } => "upsert_proposal",
                Action::SuggestQuestionDisposition { .. } => "suggest_question_disposition",
                Action::ProposeSteering { .. } => "propose_steering",
                Action::RetrieveEvidence { .. } => "retrieve_evidence",
            };
            let body = json!({"action":action,"target_task_id":target,"subject_id":subject,"packet_sha256":packet_sha,"result_sha256":digest});
            if let Action::SuggestQuestionDisposition {
                question_id,
                question_revision,
                source_message_id,
                source_sha256,
                quote_start,
                quote_end,
                answer,
                disposition: kind,
            } = action
            {
                let source = packet["selected_context"]["messages"]
                    .as_array()
                    .and_then(|a| a.iter().find(|m| m["message_id"] == *source_message_id))
                    .or_else(|| {
                        (packet["selected_context"]["current_message"]["message_id"]
                            == *source_message_id)
                            .then_some(&packet["selected_context"]["current_message"])
                    })
                    .ok_or("Suggestion source outside frozen messages")?;
                let text = source["text"].as_str().ok_or("Missing source")?;
                if source["text_sha256"] != *source_sha256
                    || !text.is_char_boundary(*quote_start)
                    || !text.is_char_boundary(*quote_end)
                    || *quote_end > text.len()
                    || (*kind == "answer" && &text[*quote_start..*quote_end] != answer)
                {
                    return Err("Suggestion quote provenance mismatch".into());
                }
                let q = question(&tx, &self.key, question_id)?.ok_or("Question not found")?;
                if q["subject_id"] != subject {
                    return Err("Question outside subject".into());
                }
                stale |= q["revision"] != *question_revision;
            }
            tx.execute(
                "INSERT INTO conversation_actions VALUES(?1,?2,?3,?4,?5)",
                params![
                    action_id,
                    turn,
                    kind,
                    encode(&body)?,
                    if stale {
                        "stale"
                    } else if kind == "upsert_proposal" {
                        "materialized"
                    } else {
                        "pending"
                    }
                ],
            )
            .map_err(|e| e.to_string())?;
            if stale {
                disposition = "stale";
                event(
                    &tx,
                    "scope.suggestion_stale",
                    target,
                    &json!({"turn_id":turn,"reason":"Saved dependency changed; no scope change saved"}),
                )?;
            } else {
                disposition = "applied";
                match action {
                    Action::RetrieveEvidence { .. } => {
                        let exchange=exchange.as_ref().ok_or("Missing evidence exchange")?;
                        let next=format!("ev-{}",&hash(&json!([turn,digest]))?[..24]);let message=format!("ev-message-{next}");
                        let text="Continue the same request with the requested immutable evidence. Service continuation; not a new human instruction or approval.";
                        let (seq,_)=event(&tx,"conversation.evidence_continuation",target,&json!({"source_turn":turn,"turn_id":next,"subject_id":subject,"evidence":crate::development_context::audit(exchange),"kind":"application read-only continuation"}))?;
                        tx.execute("INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'system',?4,?5,?6,?7)",params![message,self.key,seq,text,sha256(text.as_bytes()),target,next]).map_err(|e|e.to_string())?;
                        tx.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status,intent) VALUES(?1,1,?2,?3,?4,'queued','discussion')",params![next,self.key,message,generation]).map_err(|e|e.to_string())?;
                        tx.execute("INSERT INTO dialogue_inputs(turn_id,subject_id,request_revision) VALUES(?1,?2,0)",params![next,subject]).map_err(|e|e.to_string())?;
                        crate::coordination::save_turn_context(&tx,&next,&json!({"version":1,"subject_id":subject,"intent_kind":"evidence_continuation","source_turn":turn}))?;
                        let file=std::path::Path::new(packet["evidence_index"]["manifest_file"].as_str().unwrap()).parent().unwrap();
                        let record=json!({"work_id":target,"request_id":packet["request_id"],"subject_id":subject,"evidence_context":crate::development_context::reference(file,exchange)?});
                        tx.execute("INSERT INTO development_records(kind,id,work_id,record_json) VALUES('discussion_context',?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET record_json=excluded.record_json",params![subject,target,encode(&record)?]).map_err(|e|e.to_string())?;
                        tx.execute("UPDATE conversation_actions SET status='applied' WHERE action_id=?1",[&action_id]).map_err(|e|e.to_string())?;
                    }
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
                        tx.execute(
                            "INSERT INTO dialogue_questions VALUES(?1,?2,1)",
                            params![id, subject],
                        )
                        .map_err(|e| e.to_string())?;
                        event(
                            &tx,
                            "conversation.question",
                            target,
                            &json!({"question_id":id,"question":prompt,"reason":rationale}),
                        )?;
                    }
                    Action::UpsertProposal { proposal, .. } => {
                        let task = target
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("t-{}", &sha256(subject.as_bytes())[..24]));
                        let mut deps = context.clone();
                        deps["questions"] = json!(selected);
                        saved = task_scope::save(
                            &tx,
                            &self.key,
                            &task,
                            proposal,
                            &deps,
                            json!({"kind":"coordinator","turn_id":turn,"packet_sha256":packet_sha,"result_sha256":digest}),
                            "Coordinator validated suggestion",
                        )?;
                        tx.execute(
                            "UPDATE dialogue_subjects SET task_id=?1 WHERE subject_id=?2",
                            params![task, subject],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    Action::SuggestQuestionDisposition { .. } => {
                        disposition = "pending-human";
                        event(
                            &tx,
                            "conversation.question_suggestion",
                            target,
                            &json!({"action_id":action_id,"suggestion_sha256":hash(&body)?}),
                        )?;
                    }
                    Action::ProposeSteering { .. } => {
                        if let Err(reason)=crate::coordination::steering_intent(&tx,self,&packet,&serde_json::to_value(action).map_err(|e|e.to_string())?,&action_id){
                            disposition="stale";tx.execute("UPDATE conversation_actions SET status='stale' WHERE action_id=?1",[&action_id]).map_err(|e|e.to_string())?;
                            event(&tx,"scope.suggestion_stale",target,&json!({"turn_id":turn,"reason":reason,"steering_applied":false}))?;
                        }
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
                &json!({"message_id":id,"role":"assistant","text":parsed.assistant_text,"turn_id":turn,"evidence":references}),
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
            &json!({"turn_id":turn,"action_disposition":disposition,"scope_saved":!saved.is_null()&&saved["unchanged"]!=true,"base":saved["base"],"evidence":references}),
        )?;
        if self.development(){
            let bound:Option<String>=tx.query_row("SELECT task_id FROM dialogue_subjects WHERE subject_id=?1",[subject],|r|r.get(0)).map_err(|e|e.to_string())?;
            if let Some(t)=bound{crate::request_history::link(&tx,subject,&t)?;}
            tx.execute("UPDATE request_history_events SET request_id=(SELECT request_id FROM request_history_requests WHERE subject_id=?1),actor='coordinator' WHERE seq>?2",params![subject,history_start]).map_err(|e|e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(json!({"status":"accepted","action_disposition":disposition,"scope":saved}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{read, write};
    use tempfile::TempDir;
    fn fixture() -> (TempDir, Session) {
        let root = TempDir::new().unwrap();
        let project = root.path().join("project");
        let state = project.join(".waywr/state");
        std::fs::create_dir_all(&state).unwrap();
        write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":7,"execution_host":"systemd-user-v1","project_key":"continuous-a"}),true).unwrap();
        write(&project.join(".waywr/coordinator.json"),&json!({"schema_version":1,"profile":"claude-proposal-v2","mode":"mock-cli","provider":"claude","model":"claude-sonnet-5-5","effort":"medium","call_limit":6,"binary":std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/continuous-coordinator/mock-cli.py"),"auth_home":project,"auth_config_dir":project}),true).unwrap();
        let s = Session::open(&project, &state, "continuous-a", "test-c7").unwrap();
        let ready = s.chat_readiness().unwrap();
        assert_eq!(s.conversation_select_coordinator(&json!({"command_id":"authorize","action":"select_coordinator","expected_generation":1,"payload":{"authorize":true,"profile_sha256":ready["profile_sha256"]}})).unwrap()["status"],"accepted");
        (root, s)
    }
    fn send(s: &Session, id: &str, text: &str, subject: &str, ack: Value) -> Value {
        s.conversation_submit(&json!({"command_id":id,"action":"submit_message","payload":{"text":text,"subject_id":subject,"acknowledged_outcome":ack}})).unwrap()
    }
    fn reserve(s: &Session) -> (String, String, Value) {
        let next = s.conversation_next_queued().unwrap().unwrap();
        let p = crate::claude_chat::profile(&s.project).unwrap();
        let packet = s
            .scope_packet(&next, &p, &json!(hash(&p).unwrap()))
            .unwrap();
        let turn = next["turn_id"].as_str().unwrap();
        write(
            &s.workflow
                .state
                .join("session/conversation")
                .join(turn)
                .join("packet.json"),
            &packet,
            true,
        )
        .unwrap();
        assert_eq!(
            s.conversation_reserve(turn, &packet, "conversation")
                .unwrap()["status"],
            "accepted"
        );
        (turn.to_owned(), hash(&packet).unwrap(), packet)
    }
    fn proposal() -> Value {
        json!({"title":"Weather","objective":"Fixed data","deliverables":["index.html"],"in_scope":["Local page"],"out_of_scope":["API"],"constraints":["No network"],"acceptance_criteria":["Shows samples"],"unresolved_decisions":[]})
    }
    fn reply(text: &str, action: Value, refs: Value) -> Value {
        json!({"schema_version":3,"assistant_text":text,"evidence_refs":refs,"action":action})
    }
    fn create(s: &Session, id: &str) -> String {
        assert_eq!(
            send(s, id, "Plan it", "discussion", Value::Null)["status"],
            "accepted"
        );
        let (t, h, p) = reserve(s);
        assert_eq!(s.conversation_apply(&t,&h,&reply("First completed reply",json!({"kind":"upsert_proposal","payload":{"base":p["selected_context"]["scope"]["base"],"proposal":proposal()}}),json!([]))).unwrap()["status"],"accepted");
        s.snapshot().unwrap()["tasks"][0]["task_id"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn scope_command(s: &Session, task: &str, id: &str, action: &str) -> Value {
        let v = s.scope_detail(task).unwrap().unwrap();
        json!({"command_id":id,"action":action,"payload":{"base":v["base"],"decision_revision":v["decision_revision"],"authorization_command_id":v["authorization_command_id"],"proposal":v["record"]["proposal"],"rationale":"User edit","acknowledged_unresolved_sha256":hash(&v["record"]["proposal"]["unresolved_decisions"]).unwrap()}})
    }
    #[test]
    fn queued_followup_receives_preceding_reply_and_keeps_frozen_packet_and_authority() {
        let (_r, s) = fixture();
        send(&s, "one", "Plan it", "discussion", Value::Null);
        let (t, h, p) = reserve(&s);
        send(
            &s,
            "two",
            "An unrelated question",
            "discussion",
            Value::Null,
        );
        assert_eq!(
            hash(
                &read(
                    &s.workflow
                        .state
                        .join("session/conversation")
                        .join(&t)
                        .join("packet.json")
                )
                .unwrap()
            )
            .unwrap(),
            h
        );
        let action =
            json!({"kind":"upsert_proposal","payload":{"base":null,"proposal":proposal()}});
        assert_eq!(
            s.conversation_apply(&t, &h, &reply("Preceding response", action, json!([])))
                .unwrap()["status"],
            "accepted"
        );
        let task = s.snapshot().unwrap()["tasks"][0]["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (b, bh, bp) = reserve(&s);
        assert_eq!(bp["selected_context"]["scope"]["target_task_id"], task);
        assert!(
            bp["selected_context"]["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["text"] == "Preceding response")
        );
        assert_eq!(
            s.scope_command(&scope_command(&s, &task, "allow", "authorize_scope"))
                .unwrap()["status"],
            "accepted"
        );
        s.conversation_apply(&b, &bh, &reply("Discussing only", Value::Null, json!([])))
            .unwrap();
        assert_eq!(s.scope_detail(&task).unwrap().unwrap()["authorized"], true);
        assert_eq!(s.snapshot().unwrap()["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(p["message"], "Plan it");
    }
    #[test]
    fn local_edit_stales_old_scope_action_without_losing_text_or_decision_history() {
        let (_r, s) = fixture();
        let task = create(&s, "seed");
        s.scope_command(&scope_command(&s, &task, "allow", "authorize_scope"))
            .unwrap();
        send(&s, "revise", "Add constraint", "discussion", Value::Null);
        let (t, h, p) = reserve(&s);
        let mut edit = scope_command(&s, &task, "edit", "save_proposal_revision");
        edit["payload"]["proposal"]["constraints"] = json!(["User's saved change"]);
        s.scope_command(&edit).unwrap();
        let r = reply(
            "Earlier model suggestion",
            json!({"kind":"upsert_proposal","payload":{"base":p["selected_context"]["scope"]["base"],"proposal":proposal()}}),
            json!([]),
        );
        assert_eq!(
            s.conversation_apply(&t, &h, &r).unwrap()["action_disposition"],
            "stale"
        );
        assert_eq!(s.conversation_apply(&t, &h, &r).unwrap()["replayed"], true);
        let head = s.scope_detail(&task).unwrap().unwrap();
        assert_eq!(
            head["record"]["proposal"]["constraints"][0],
            "User's saved change"
        );
        assert_eq!(head["authorized"], false);
        assert_eq!(head["decision_history"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn recoverable_failure_holds_old_queue_new_turn_does_not_release_and_replay_is_once() {
        let (_r, s) = fixture();
        send(&s, "one", "Bad format", "discussion", Value::Null);
        let (t, h, _) = reserve(&s);
        let held = send(&s, "older", "Already queued", "discussion", Value::Null);
        let proof = json!({"schema_version":1,"category":"format","known_completion":true,"settled":true,"capabilities_valid":true});
        let out = s
            .dialogue_stop(&t, &h, &proof, "No scope change saved")
            .unwrap();
        assert_eq!(out["recoverable"], true);
        assert!(s.conversation_next_queued().unwrap().is_none());
        assert_eq!(
            s.dialogue_stop(&t, &h, &proof, "No scope change saved")
                .unwrap()["replayed"],
            true
        );
        let paused = s.dialogue_state().unwrap()["pause"].clone();
        assert_eq!(
            send(
                &s,
                "new",
                "Continue differently",
                "discussion",
                paused.clone()
            )["status"],
            "accepted"
        );
        let (n, nh, _) = reserve(&s);
        s.conversation_apply(&n, &nh, &reply("New response", Value::Null, json!([])))
            .unwrap();
        assert!(s.conversation_next_queued().unwrap().is_none());
        assert_eq!(s.dialogue_state().unwrap()["queue"][0]["status"], "held");
        let cmd = json!({"command_id":"continue-held","action":"continue_held","payload":{"turn_ids":[held["turn_id"]],"acknowledged_outcome":paused}});
        let accepted = s.dialogue_command(&cmd).unwrap();
        assert_eq!(accepted["released"], 1);
        assert_eq!(s.dialogue_command(&cmd).unwrap(), accepted);
        let (q, qh, _) = reserve(&s);
        assert_eq!(q, held["turn_id"].as_str().unwrap());
        s.conversation_apply(&q, &qh, &reply("Released by human", Value::Null, json!([])))
            .unwrap();
        assert_eq!(
            s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            3
        );
        let reopened = Session::open(&s.project, &s.workflow.state, &s.key, &s.release).unwrap();
        assert_eq!(
            reopened.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            3
        );
    }
    #[test]
    fn interrupt_fences_late_actions_and_hard_uncertainty_cannot_continue() {
        let (_r, s) = fixture();
        send(&s, "one", "May propose", "discussion", Value::Null);
        let (t, h, _) = reserve(&s);
        let cancel = json!({"command_id":"interrupt","action":"cancel_conversation","payload":{"turn_id":t}});
        assert_eq!(s.chat_cancel(&cancel).unwrap()["actions_fenced"], true);
        assert_eq!(s.chat_cancel(&cancel).unwrap()["actions_fenced"], true);
        let r = reply(
            "Saved?",
            json!({"kind":"upsert_proposal","payload":{"base":null,"proposal":proposal()}}),
            json!([]),
        );
        assert_eq!(
            s.conversation_apply(&t, &h, &r).unwrap()["recoverable"],
            true
        );
        assert!(
            s.snapshot().unwrap()["tasks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let pause = s.dialogue_state().unwrap()["pause"].clone();
        send(&s, "next", "A new message", "discussion", pause);
        let (t, h, _) = reserve(&s);
        s.dialogue_stop(&t,&h,&json!({"category":"uncertain","known_completion":false,"settled":false,"capabilities_valid":true}),"Missing receipt").unwrap();
        let p = s.dialogue_state().unwrap()["pause"].clone();
        assert_eq!(
            send(&s, "forbidden", "Try again", "discussion", p)["code"],
            "coordinator_unavailable"
        );
    }
    #[test]
    fn pending_question_does_not_block_discussion_or_revoke_scope_and_confirmation_is_auditable() {
        let (_r, s) = fixture();
        let task = create(&s, "seed");
        s.scope_command(&scope_command(&s, &task, "allow", "authorize_scope"))
            .unwrap();
        send(&s, "ask", "Question please", "discussion", Value::Null);
        let (t, h, _) = reserve(&s);
        s.conversation_apply(&t,&h,&reply("A question",json!({"kind":"ask_question","payload":{"prompt":"Units?","rationale":"Optional clarification","options":["C"]}}),json!([]))).unwrap();
        assert_eq!(s.scope_detail(&task).unwrap().unwrap()["authorized"], true);
        let q = s.conversation_snapshot().unwrap()["questions"][0].clone();
        send(&s, "answer-text", "C", "discussion", Value::Null);
        let (t, h, p) = reserve(&s);
        let source = &p["selected_context"]["current_message"];
        let action = json!({"kind":"suggest_question_disposition","payload":{"question_id":q["question_id"],"question_revision":q["scope"]["question_revision"],"disposition":"answer","answer":"C","source_message_id":source["message_id"],"source_sha256":source["text_sha256"],"quote_start":0,"quote_end":1}});
        s.conversation_apply(&t, &h, &reply("Suggested answer", action, json!([])))
            .unwrap();
        let snapshot = s.conversation_snapshot().unwrap();
        assert_eq!(snapshot["questions"][0]["status"], "open");
        let suggestion = snapshot["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["kind"] == "suggest_question_disposition")
            .unwrap();
        let cmd = json!({"command_id":"confirm","action":"confirm_question_disposition","payload":{"action_id":suggestion["action_id"],"suggestion_sha256":hash(&suggestion["body"]).unwrap()}});
        let accepted = s.dialogue_command(&cmd).unwrap();
        assert_eq!(accepted["status"], "accepted");
        assert_eq!(s.dialogue_command(&cmd).unwrap(), accepted);
        assert_eq!(
            s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            3
        );
        let c = s.workflow.connection().unwrap();
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM dialogue_question_decisions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
    #[test]
    fn withdrawn_queue_never_dispatches_and_writer_busy_is_bounded() {
        let (_r, s) = fixture();
        let a = send(&s, "queued", "Draft", "discussion", Value::Null);
        let cmd = json!({"command_id":"withdraw","action":"withdraw_queued","payload":{"turn_id":a["turn_id"]}});
        assert_eq!(s.dialogue_command(&cmd).unwrap()["call_charged"], false);
        assert!(s.conversation_next_queued().unwrap().is_none());
        let lock = s.workflow.connection().unwrap();
        lock.execute_batch("BEGIN IMMEDIATE").unwrap();
        let start = std::time::Instant::now();
        assert!(s.conversation_submit(&json!({"command_id":"busy","action":"submit_message","payload":{"text":"Kept draft"}})).is_err());
        assert!(start.elapsed() < std::time::Duration::from_millis(500));
        lock.execute_batch("ROLLBACK").unwrap();
        assert!(s.conversation_next_queued().unwrap().is_none());
    }
    #[test]
    fn linked_answer_revisions_invalidate_only_their_saved_scope_and_are_append_only() {
        let (_r, s) = fixture();
        send(&s, "ask", "Question", "discussion", Value::Null);
        let (t, h, _) = reserve(&s);
        s.conversation_apply(&t,&h,&reply("Question",json!({"kind":"ask_question","payload":{"prompt":"Units?","rationale":"Scope","options":["C"]}}),json!([]))).unwrap();
        let q = s.conversation_snapshot().unwrap()["questions"][0].clone();
        send(&s, "proposal", "Propose", "discussion", Value::Null);
        let (t, h, _) = reserve(&s);
        s.conversation_apply(
            &t,
            &h,
            &reply(
                "Proposal",
                json!({"kind":"upsert_proposal","payload":{"base":null,"proposal":proposal()}}),
                json!([format!("question:{}", q["question_id"].as_str().unwrap())]),
            ),
        )
        .unwrap();
        let task = s.snapshot().unwrap()["tasks"][0]["task_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            s.scope_command(&scope_command(&s, &task, "premature", "authorize_scope"))
                .unwrap()["code"],
            "scope_not_ready"
        );
        let answer = json!({"command_id":"answer","action":"answer_conversation","payload":{"question_id":q["question_id"],"question_revision":1,"answer":"C"}});
        assert_eq!(
            s.conversation_answer(&answer).unwrap()["status"],
            "accepted"
        );
        assert_eq!(
            s.scope_detail(&task).unwrap().unwrap()["needs_revision"],
            true
        );
        let edit = scope_command(&s, &task, "capture-answer", "save_proposal_revision");
        s.scope_command(&edit).unwrap();
        assert_eq!(
            s.scope_command(&scope_command(&s, &task, "allow", "authorize_scope"))
                .unwrap()["status"],
            "accepted"
        );
        let mut changed = answer;
        changed["command_id"] = json!("answer-revision");
        changed["payload"]["question_revision"] = json!(2);
        changed["payload"]["answer"] = json!("F");
        assert_eq!(
            s.conversation_answer(&changed).unwrap()["status"],
            "accepted"
        );
        assert_eq!(s.scope_detail(&task).unwrap().unwrap()["authorized"], false);
        assert_eq!(
            s.workflow
                .connection()
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM dialogue_question_decisions",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
    }
    #[test]
    fn concurrent_duplicate_send_commits_once() {
        let (root, mut s) = fixture();
        let writer = crate::db_writer::DbWriter::start(
            s.project.clone(),
            s.workflow.state.clone(),
            s.key.clone(),
            s.release.clone(),
        )
        .unwrap();
        s.attach_writer(writer);
        let s = std::sync::Arc::new(s);
        let body = json!({"command_id":"same-input","action":"submit_message","payload":{"text":"Natural unchanged message","subject_id":"discussion"}});
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers = (0..2)
            .map(|_| {
                let s = s.clone();
                let b = body.clone();
                let gate = barrier.clone();
                std::thread::spawn(move || {
                    gate.wait();
                    s.conversation_submit(&b).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let result = workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(result[0], result[1]);
        assert_eq!(
            s.dialogue_state().unwrap()["queue"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        drop(s);
        drop(root);
    }

    #[test]
    fn deferred_formal_decision_stays_unresolved_and_requires_matching_scope_acknowledgement(){
        let (_r,s)=fixture();send(&s,"ask","Ask","discussion",Value::Null);let(t,h,_)=reserve(&s);s.conversation_apply(&t,&h,&reply("Question",json!({"kind":"ask_question","payload":{"prompt":"Which units?","rationale":"Scope","options":["C","F"]}}),json!([]))).unwrap();
        send(&s,"defer","Decide later","discussion",Value::Null);let(t,h,p)=reserve(&s);let q=&p["selected_context"]["scope"]["questions"][0];let m=&p["selected_context"]["current_message"];assert_eq!(q["options"],json!(["C","F"]));assert_eq!(q["rationale"],"Scope");
        s.conversation_apply(&t,&h,&reply("Suggest deferral",json!({"kind":"suggest_question_disposition","payload":{"question_id":q["question_id"],"question_revision":q["revision"],"disposition":"defer","answer":"","source_message_id":m["message_id"],"source_sha256":m["text_sha256"],"quote_start":0,"quote_end":12}}),json!([]))).unwrap();let snap=s.conversation_snapshot().unwrap();let a=snap["actions"].as_array().unwrap().iter().find(|a|a["kind"]=="suggest_question_disposition").unwrap();assert_eq!(snap["questions"][0]["status"],"open");
        assert_eq!(s.dialogue_command(&json!({"command_id":"record-defer","action":"confirm_question_disposition","payload":{"action_id":a["action_id"],"suggestion_sha256":hash(&a["body"]).unwrap()}})).unwrap()["status"],"accepted");assert_eq!(s.conversation_snapshot().unwrap()["questions"][0]["status"],"deferred");
        send(&s,"proposal","Proposal","discussion",Value::Null);let(t,h,p)=reserve(&s);let mut body=proposal();body["unresolved_decisions"]=json!(["Which units?"]);s.conversation_apply(&t,&h,&reply("Scope",json!({"kind":"upsert_proposal","payload":{"base":p["selected_context"]["scope"]["base"],"proposal":body}}),json!([format!("question:{}",q["question_id"].as_str().unwrap())]))).unwrap();let task=s.snapshot().unwrap()["tasks"][0]["task_id"].as_str().unwrap().to_owned();let mut cmd=scope_command(&s,&task,"bad-ack","authorize_scope");cmd["payload"]["acknowledged_unresolved_sha256"]=json!(hash(&json!([])).unwrap());assert_eq!(s.scope_command(&cmd).unwrap()["status"],"rejected");assert_eq!(s.scope_command(&scope_command(&s,&task,"exact-ack","authorize_scope")).unwrap()["status"],"accepted");assert_eq!(s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],3);
    }


    #[test]fn scope_acknowledgement_supplies_revision_for_immediate_lifecycle_control(){let(_r,s)=fixture();let task=create(&s,"proposal");let outcome=s.scope_command(&scope_command(&s,&task,"auth","authorize_scope")).unwrap();assert_eq!(outcome["state_revision"],s.snapshot().unwrap()["state_revision"]);assert_eq!(s.lifecycle_command(&json!({"command_id":"exit-now","action":"stop_session","expected_state_revision":outcome["state_revision"],"payload":{}})).unwrap()["status"],"accepted");}


    #[test]fn native_mode_requires_its_own_grant_and_pinned_preflight(){
        let (_r,s)=fixture();let path=s.project.join(".waywr/coordinator.json");let mut p=read(&path).unwrap();p["mode"]=json!("native");write(&path,&p,false).unwrap();let r=s.chat_readiness().unwrap();assert_eq!(r["status"],"awaiting-authorization");assert_eq!(r["authorized"],false);assert_eq!(s.conversation_select_coordinator(&json!({"command_id":"native-grant","action":"select_coordinator","expected_generation":1,"payload":{"authorize":true,"profile_sha256":r["profile_sha256"]}})).unwrap()["code"],"coordinator_preflight");assert_eq!(send(&s,"native-blocked","No launch","discussion",Value::Null)["status"],"rejected");assert_eq!(s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],0);assert!(s.conversation_next_queued().unwrap().is_none());
        p["profile"]=json!("claude-proposal-v1");write(&path,&p,false).unwrap();assert!(crate::claude_chat::profile(&s.project).unwrap_err().contains("State 7"));
    }

}
