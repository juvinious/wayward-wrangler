//! Schema-4 conversation authority. Provider output is inert until this broker
//! validates it; all mutations use the project writer and short SQLite commits.
use crate::{
    codec::{canonical, canonical_sha256, sha256},
    native_contract::{CoordinatorAction, CoordinatorTurn},
    session_store::Session,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};

pub(crate) fn encode(value: &Value) -> Result<String, String> {
    String::from_utf8(canonical(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub(crate) fn reject(code: &str, reason: &str) -> Value {
    json!({"status":"rejected","code":code,"message":reason})
}
pub(crate) fn metadata(tx: &rusqlite::Transaction<'_>, name: &str) -> Result<Value, String> {
    let raw: String = tx
        .query_row(
            "SELECT value FROM session_metadata WHERE name=?1",
            [name],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}
fn state_revision(tx: &rusqlite::Transaction<'_>) -> Result<u64, String> {
    metadata(tx, "state_revision")?
        .as_u64()
        .ok_or("Invalid state revision".into())
}
pub(crate) fn event(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    task: Option<&str>,
    body: &Value,
) -> Result<(i64, u64), String> {
    tx.execute(
        "INSERT INTO session_events(schema_version,type,task_id,body) VALUES(2,?1,?2,?3)",
        params![kind, task, encode(body)?],
    )
    .map_err(|e| e.to_string())?;
    let seq = tx.last_insert_rowid();
    let next = state_revision(tx)? + 1;
    tx.execute(
        "UPDATE session_metadata SET value=?1 WHERE name='state_revision'",
        [encode(&json!(next))?],
    )
    .map_err(|e| e.to_string())?;
    Ok((seq, next))
}
pub(crate) fn prior(tx: &rusqlite::Transaction<'_>, body: &Value) -> Result<Option<Value>, String> {
    let id = body["command_id"].as_str().ok_or("Missing command ID")?;
    let row: Option<(String, String)> = tx
        .query_row(
            "SELECT body,outcome FROM session_commands WHERE command_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match row {
        None => Ok(None),
        Some((old, outcome)) if old == encode(body)? => Ok(Some(
            serde_json::from_str(&outcome).map_err(|e| e.to_string())?,
        )),
        Some(_) => Ok(Some(reject(
            "idempotency_conflict",
            "Command ID already belongs to different input",
        ))),
    }
}
pub(crate) fn commit(tx: &rusqlite::Transaction<'_>, body: &Value, outcome: &Value) -> Result<(), String> {
    tx.execute(
        "INSERT INTO session_commands(command_id,body,outcome) VALUES(?1,?2,?3)",
        params![body["command_id"].as_str(), encode(body)?, encode(outcome)?],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub(crate) fn valid_command(body: &Value, action: &str) -> Result<(), Value> {
    let id = body["command_id"].as_str().unwrap_or("");
    if id.is_empty() || id.len() > 128 {
        return Err(reject("invalid_request", "Invalid command ID"));
    }
    if body["action"] != action || !body["payload"].is_object() {
        return Err(reject("invalid_request", "Invalid conversation command"));
    }
    Ok(())
}

impl Session {
    pub fn conversation_enabled(&self) -> Result<bool, String> {
        let contract = self.workflow.state.join("state-contract.json");
        if !contract.is_file() {
            return Ok(false);
        }
        let value = crate::execution::read(&contract)?;
        Ok(matches!(value["state_schema"].as_u64(),Some(4|5|6|7|8|9)) && value["project_key"] == self.key)
    }
    pub fn conversation_chat(&self)->bool{
        crate::execution::read(&self.workflow.state.join("state-contract.json")).ok()
            .is_some_and(|v|v["state_schema"]==5&&v["project_key"]==self.key)
    }
    pub fn chat_readiness(&self)->Result<Value,String>{
        if self.development(){return self.development_readiness()}
        let profile=match crate::claude_chat::profile(&self.project){Ok(v)=>v,Err(e)=>return Ok(json!({"status":"blocked","reason":e}))};
        if self.scope_enabled()!=matches!(profile["profile"].as_str(),Some("claude-proposal-v1"|"claude-proposal-v2")) || self.continuous()!=(profile["profile"]=="claude-proposal-v2"){return Ok(json!({"status":"blocked","reason":"Coordinator profile is incompatible with this project's bound conversation contract"}));}
        let hash=canonical_sha256(&profile).map_err(|e|e.to_string())?;
        let conn=self.workflow.connection()?;
        let raw:String=conn.query_row("SELECT value FROM session_metadata WHERE name='claude_chat_authorization_v1'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let auth:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let stopped:bool=conn.query_row("SELECT exists(SELECT 1 FROM conversation_turns WHERE status IN ('uncertain','failed','output-invalid','cancelled'))",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let mut stopped=stopped;let mut recoverable=false;let mut pause_reason=String::new();
        if self.continuous(){let raw:String=conn.query_row("SELECT value FROM session_metadata WHERE name='dialogue_pause_v1'",[],|r|r.get(0)).map_err(|e|e.to_string())?;let p:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;stopped=p["active"]==true&&p["hard"]==true;recoverable=p["active"]==true&&p["hard"]!=true;pause_reason=p["reason"].as_str().unwrap_or("").into();}
        let active:bool=conn.query_row("SELECT exists(SELECT 1 FROM conversation_turns WHERE status='reserved')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let remaining:i64=conn.query_row("SELECT limit_count-charged FROM conversation_budgets WHERE bucket='conversation'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        let authorized=auth["authorized"]==true&&auth["profile_sha256"]==hash;
        Ok(json!({"profile":profile,"profile_sha256":hash,"authorized":authorized,
          "status":if stopped{"blocked"}else if active{"running"}else if remaining==0{"exhausted"}else if authorized&&recoverable{"recoverable"}else if authorized{"available"}else{"awaiting-authorization"},
          "reason":if stopped{"A prior conversation stopped; inspect its receipt and partial output. No automatic replay or further trial calls."}else if recoverable{pause_reason.as_str()}else if !authorized{"Review settings and explicitly authorize the frozen conversation allowance."}else{""}}))
    }
    pub fn chat_progress(&self,turn:&str,text:&str)->Result<(),String>{
        if let Some(writer)=&self.workflow.writer{let turn=turn.to_owned();let text=text.to_owned();return writer.call(move|s|s.chat_progress(&turn,&text));}
        if text.len()>crate::claude_chat::TEXT_LIMIT{return Err("Conversation stream exceeds 32 KiB".into());}
        let mut conn=self.workflow.connection()?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let body=json!({"turn_id":turn,"text":text,"provisional":true});
        let old:Option<String>=tx.query_row("SELECT value FROM session_metadata WHERE name='claude_chat_stream_v1'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let raw=encode(&body)?;if old.as_deref()!=Some(raw.as_str()){
          tx.execute("INSERT INTO session_metadata(name,value) VALUES('claude_chat_stream_v1',?1) ON CONFLICT(name) DO UPDATE SET value=excluded.value",[raw]).map_err(|e|e.to_string())?;
          event(&tx,"conversation.delta",None,&json!({"turn_id":turn,"bytes":text.len(),"text_sha256":sha256(text.as_bytes()),"provisional":true}))?;
        }tx.commit().map_err(|e|e.to_string())
    }
    pub fn chat_reserved(&self)->Result<Option<Value>,String>{
        let conn=self.workflow.connection()?;
        conn.query_row("SELECT t.turn_id,t.packet_sha256 FROM conversation_turns t WHERE status='reserved' AND project_key=?1",[&self.key],|r|Ok(json!({"turn_id":r.get::<_,String>(0)?,"packet_sha256":r.get::<_,String>(1)?}))).optional().map_err(|e|e.to_string())
    }
    pub fn chat_cancel(&self,body:&Value)->Result<Value,String>{
        if self.continuous(){return self.dialogue_command(body)}
        if let Some(writer)=&self.workflow.writer{let body=body.clone();return writer.call(move|s|s.chat_cancel(&body));}
        if let Err(e)=valid_command(body,"cancel_conversation"){return Ok(e);}
        let mut conn=self.workflow.connection()?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if let Some(old)=prior(&tx,body)?{return Ok(old);}
        let turn:Option<String>=tx.query_row("SELECT turn_id FROM conversation_turns WHERE status='reserved' AND project_key=?1",[&self.key],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let Some(turn)=turn else{return Ok(reject("no_active_call","No coordinator call is active"));};
        if body["payload"]["turn_id"]!=turn{return Ok(reject("stale_turn","The selected coordinator call changed"));}
        let raw=encode(&json!({"turn_id":turn,"command_id":body["command_id"]}))?;
        tx.execute("INSERT INTO session_metadata(name,value) VALUES('claude_chat_cancel_v1',?1) ON CONFLICT(name) DO UPDATE SET value=excluded.value",[raw]).map_err(|e|e.to_string())?;
        let (_,rev)=event(&tx,"conversation.cancel_requested",None,&json!({"turn_id":turn}))?;
        let outcome=json!({"status":"accepted","turn_id":turn,"state_revision":rev});commit(&tx,body,&outcome)?;tx.commit().map_err(|e|e.to_string())?;Ok(outcome)
    }
    pub fn chat_cancel_request(&self,turn:&str)->Result<Option<Value>,String>{
        let conn=self.workflow.connection()?;
        let raw:Option<String>=conn.query_row("SELECT value FROM session_metadata WHERE name='claude_chat_cancel_v1'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        Ok(raw.and_then(|s|serde_json::from_str::<Value>(&s).ok()).filter(|v|v["turn_id"]==turn))
    }
    pub fn conversation_schema(&self) -> Result<(), String> {
        if !self.conversation_enabled()? {
            return Err("Project is not bound to schema 4".into());
        }
        self.workflow.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS conversation_messages(
                message_id TEXT PRIMARY KEY,schema_version INTEGER NOT NULL,project_key TEXT NOT NULL,
                seq INTEGER NOT NULL UNIQUE,role TEXT NOT NULL,text TEXT NOT NULL,content_sha256 TEXT NOT NULL,
                task_id TEXT,turn_id TEXT);
             CREATE TABLE IF NOT EXISTS conversation_turns(
                turn_id TEXT PRIMARY KEY,schema_version INTEGER NOT NULL,project_key TEXT NOT NULL,
                message_id TEXT NOT NULL UNIQUE,binding_generation INTEGER NOT NULL,
                context_sha256 TEXT,status TEXT NOT NULL,intent TEXT NOT NULL DEFAULT 'discussion',budget_bucket TEXT,
                packet_sha256 TEXT,result_sha256 TEXT,result_json TEXT,reason TEXT);
             CREATE TABLE IF NOT EXISTS conversation_questions(
                question_id TEXT PRIMARY KEY,schema_version INTEGER NOT NULL,project_key TEXT NOT NULL,
                turn_id TEXT NOT NULL,prompt TEXT NOT NULL,options_json TEXT NOT NULL,
                status TEXT NOT NULL,answer TEXT,answer_command_id TEXT);
             CREATE TABLE IF NOT EXISTS conversation_bindings(
                singleton INTEGER PRIMARY KEY CHECK(singleton=1),schema_version INTEGER NOT NULL,
                generation INTEGER NOT NULL,provider TEXT NOT NULL,model TEXT NOT NULL,effort TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS conversation_budgets(
                bucket TEXT PRIMARY KEY,limit_count INTEGER NOT NULL,charged INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE IF NOT EXISTS conversation_actions(
                action_id TEXT PRIMARY KEY,turn_id TEXT NOT NULL UNIQUE,kind TEXT NOT NULL,
                body_json TEXT NOT NULL,status TEXT NOT NULL);
             INSERT OR IGNORE INTO conversation_bindings VALUES(1,1,1,'fake','deterministic','none');
             INSERT OR IGNORE INTO conversation_budgets(bucket,limit_count,charged) VALUES('conversation',6,0);
             INSERT OR IGNORE INTO conversation_budgets(bucket,limit_count,charged) VALUES('consultation',1,0);
             INSERT OR IGNORE INTO conversation_budgets(bucket,limit_count,charged) VALUES('task',4,0);"
        ).map_err(|e|e.to_string())?;
        if self.conversation_native() && !self.development(){
          let p=crate::claude_chat::profile(&self.project).unwrap_or(json!({"model":"claude-sonnet-5-5","effort":"medium"}));
          let conn=self.workflow.connection()?;
          conn.execute("INSERT OR IGNORE INTO session_metadata(name,value) VALUES('claude_chat_authorization_v1',?1)",[encode(&json!({"schema_version":1,"authorized":false}))?]).map_err(|e|e.to_string())?;
          conn.execute("UPDATE conversation_bindings SET provider='claude',model=?1,effort=?2 WHERE generation=1",params![p["model"].as_str(),p["effort"].as_str()]).map_err(|e|e.to_string())?;
          conn.execute("UPDATE conversation_budgets SET limit_count=CASE WHEN bucket='conversation' THEN ?1 ELSE 0 END",[if self.scope_enabled(){6}else{3}]).map_err(|e|e.to_string())?;
        }
        if self.scope_enabled(){self.scope_schema()?;}
        if self.continuous(){self.dialogue_schema()?;}
        if self.development(){self.development_schema()?;}
        Ok(())
    }
    pub fn conversation_snapshot(&self) -> Result<Value, String> {
        if !self.conversation_enabled()? {
            return Err("Conversation unavailable on this project schema".into());
        }
        let chat=if self.conversation_native(){self.chat_readiness()?}else{Value::Null};
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|e| e.to_string())?;
        let watermark = state_revision(&tx)?;
        let mut stmt=tx.prepare("SELECT message_id,seq,role,text,task_id,turn_id FROM conversation_messages ORDER BY seq DESC LIMIT 64")
            .map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(json!({"message_id":r.get::<_,String>(0)?,
            "seq":r.get::<_,i64>(1)?,"role":r.get::<_,String>(2)?,"text":r.get::<_,String>(3)?,
            "task_id":r.get::<_,Option<String>>(4)?,"turn_id":r.get::<_,Option<String>>(5)?}))
            })
            .map_err(|e| e.to_string())?;
        let mut messages = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        crate::request_history::add_event_times(&tx, &mut messages)?;
        let mut history_truncated=false;
        if self.scope_enabled(){let mut bytes=0usize;let keep=messages.iter().take_while(|m|{bytes+=serde_json::to_vec(m).map_or(usize::MAX,|v|v.len());bytes<=256*1024}).count();history_truncated=keep<messages.len();messages.truncate(keep);}
        messages.reverse();
        let mut questions = Vec::new();
        let mut stmt=tx.prepare("SELECT question_id,turn_id,prompt,options_json,status,answer FROM conversation_questions WHERE status='open' OR (?1 AND status='answered' AND EXISTS(SELECT 1 FROM conversation_actions a WHERE a.turn_id=conversation_questions.turn_id AND a.status='answered')) OR (?2 AND status IN ('answered','deferred')) ORDER BY CASE WHEN ?1 THEN -rowid ELSE rowid END LIMIT 8")
            .map_err(|e|e.to_string())?;
        for row in stmt
            .query_map(params![self.scope_enabled(),self.continuous()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
        {
            let (id, turn, prompt, options, status, answer) = row.map_err(|e| e.to_string())?;
            questions.push(json!({"question_id":id,"turn_id":turn,"prompt":prompt,
                "options":serde_json::from_str::<Value>(&options).map_err(|e|e.to_string())?,
                "status":status,"answer":answer,"scope":if self.scope_enabled(){self.scope_question_context(&id)?}else{Value::Null}}));
        }
        let binding=tx.query_row("SELECT generation,provider,model,effort FROM conversation_bindings WHERE singleton=1",[],
            |r|Ok(json!({"generation":r.get::<_,i64>(0)?,"provider":r.get::<_,String>(1)?,
                "model":r.get::<_,String>(2)?,"effort":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?;
        let mut budget = serde_json::Map::new();
        let mut stmt = tx
            .prepare("SELECT bucket,limit_count,charged FROM conversation_budgets ORDER BY bucket")
            .map_err(|e| e.to_string())?;
        for row in stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
        {
            let (bucket, limit, charged) = row.map_err(|e| e.to_string())?;
            budget.insert(
                bucket,
                json!({"limit":limit,"charged":charged,"remaining":limit-charged}),
            );
        }
        let mut actions=Vec::new();
        let mut stmt=tx.prepare("SELECT action_id,turn_id,kind,body_json,status FROM conversation_actions WHERE status='pending' ORDER BY rowid LIMIT 8")
            .map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,
            r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?)))
            .map_err(|e|e.to_string())?{
            let (id,turn,kind,body,status)=row.map_err(|e|e.to_string())?;
            actions.push(json!({"action_id":id,"turn_id":turn,"kind":kind,"body":serde_json::from_str::<Value>(&body).map_err(|e|e.to_string())?,"status":status}));
        }
        if self.scope_enabled()&&!self.continuous(){actions.clear();} // Questions already expose the only pending scope action.
        let mut turns=Vec::new();
        let mut stmt=tx.prepare("SELECT turn_id,status,budget_bucket,reason FROM conversation_turns ORDER BY rowid DESC LIMIT 16")
            .map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,
            r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?)))
            .map_err(|e|e.to_string())?{
            let (id,status,bucket,reason)=row.map_err(|e|e.to_string())?;
            turns.push(json!({"turn_id":id,"status":status,"budget_bucket":bucket,"reason":reason,
                "execution_record":if self.conversation_native(){json!(format!("session/coordinator/{id}/call-v1/"))}else{Value::Null}}));
        }
        let stream:Option<String>=tx.query_row("SELECT value FROM session_metadata WHERE name='claude_chat_stream_v1'",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        drop(stmt);
        Ok(json!({"schema_version":1,"state_revision":watermark,"messages":messages,"questions":questions,
            "binding":binding,"budgets":budget,"actions":actions,"turns":turns,"scope_enabled":self.scope_enabled(),"continuous":if self.continuous(){self.dialogue_state()?}else{Value::Null},"history_truncated":history_truncated,"execution_available":self.development()||!self.conversation_native(),"development":if self.development(){self.development_snapshot()?}else{Value::Null},
            "coordination":if crate::coordination::enabled(self){crate::coordination::snapshot(self)?}else{Value::Null},"chat":chat,
            "provisional":stream.and_then(|s|serde_json::from_str::<Value>(&s).ok())}))
    }
    pub fn conversation_handoff(&self)->Result<Value,String>{
        if !self.conversation_enabled()?{return Err("Conversation unavailable on this project schema".into());}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Deferred).map_err(|e|e.to_string())?;
        let revision=state_revision(&tx)?;
        let decision_count:i64=tx.query_row("SELECT count(*) FROM conversation_questions WHERE status='answered'",[],|r|r.get(0))
            .map_err(|e|e.to_string())?;
        if decision_count>16{return Err("More than 16 accepted conversation decisions require an explicit context selection".into());}
        let mut decisions=Vec::new();
        let mut stmt=tx.prepare("SELECT q.question_id,q.turn_id,q.prompt,q.answer,a.body_json
            FROM conversation_questions q LEFT JOIN conversation_actions a ON a.turn_id=q.turn_id AND a.kind='ask_question'
            WHERE q.status='answered' ORDER BY q.rowid LIMIT 16").map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,
            r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?)))
            .map_err(|e|e.to_string())?{
            let (id,turn,prompt,answer,action)=row.map_err(|e|e.to_string())?;
            let rationale=action.and_then(|raw|serde_json::from_str::<Value>(&raw).ok())
                .and_then(|v|v["payload"]["rationale"].as_str().map(str::to_owned));
            decisions.push(json!({"question_id":id,"source_turn_id":turn,"prompt":prompt,
                "rationale":rationale,"answer":answer,"answer_sha256":sha256(answer.as_bytes())}));
        }
        let mut tasks=Vec::new();
        let mut stmt=tx.prepare("SELECT task_id,version,status,title,run_id FROM session_tasks ORDER BY rowid DESC LIMIT 16")
            .map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok(json!({"task_id":r.get::<_,String>(0)?,
            "version":r.get::<_,i64>(1)?,"status":r.get::<_,String>(2)?,
            "title":r.get::<_,String>(3)?,"run_id":r.get::<_,Option<String>>(4)?})))
            .map_err(|e|e.to_string())?{tasks.push(row.map_err(|e|e.to_string())?);}
        let mut turns=Vec::new();
        let mut stmt=tx.prepare("SELECT turn_id,status,budget_bucket,packet_sha256,result_sha256 FROM conversation_turns ORDER BY rowid DESC LIMIT 16")
            .map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok(json!({"turn_id":r.get::<_,String>(0)?,
            "status":r.get::<_,String>(1)?,"bucket":r.get::<_,Option<String>>(2)?,
            "packet_sha256":r.get::<_,Option<String>>(3)?,"result_sha256":r.get::<_,Option<String>>(4)?})))
            .map_err(|e|e.to_string())?{turns.push(row.map_err(|e|e.to_string())?);}
        let mut questions=Vec::new();
        let mut stmt=tx.prepare("SELECT question_id,turn_id,prompt FROM conversation_questions WHERE status='open' ORDER BY rowid LIMIT 8")
            .map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok(json!({"question_id":r.get::<_,String>(0)?,
            "source_turn_id":r.get::<_,String>(1)?,"prompt":r.get::<_,String>(2)?})))
            .map_err(|e|e.to_string())?{questions.push(row.map_err(|e|e.to_string())?);}
        let mut budgets=serde_json::Map::new();
        let mut stmt=tx.prepare("SELECT bucket,limit_count,charged FROM conversation_budgets ORDER BY bucket")
            .map_err(|e|e.to_string())?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?)))
            .map_err(|e|e.to_string())?{
            let (bucket,limit,charged)=row.map_err(|e|e.to_string())?;
            budgets.insert(bucket,json!({"limit":limit,"charged":charged,"remaining":limit-charged}));
        }
        Ok(json!({"schema_version":1,"project_key":self.key,"state_revision":revision,
            "scope_checkpoint":if self.scope_enabled(){self.scope_checkpoint()?}else{Value::Null},"accepted_decisions":decisions,"tasks":tasks,"open_questions":questions,
            "operation_refs":turns,"budgets":budgets}))
    }
    pub fn conversation_next_queued(&self)->Result<Option<Value>,String>{
        if self.continuous(){return self.dialogue_next()}
        if !self.conversation_enabled()?{return Ok(None);}
        let conn=self.workflow.connection()?;
        conn.query_row("SELECT t.turn_id,m.message_id,m.role,m.text,t.binding_generation,t.intent,m.task_id
            FROM conversation_turns t JOIN conversation_messages m ON m.message_id=t.message_id
            WHERE t.status='queued' AND t.project_key=?1
              AND NOT EXISTS(SELECT 1 FROM conversation_turns WHERE status='reserved'
                AND budget_bucket IN ('conversation','consultation'))
            ORDER BY m.seq LIMIT 1",[&self.key],|r|
            Ok(json!({"turn_id":r.get::<_,String>(0)?,"message_id":r.get::<_,String>(1)?,
                "role":r.get::<_,String>(2)?,"text":r.get::<_,String>(3)?,"binding_generation":r.get::<_,i64>(4)?,
                "intent":r.get::<_,String>(5)?,"task_id":r.get::<_,Option<String>>(6)?})))
            .optional().map_err(|e|e.to_string())
    }
    pub fn conversation_provisional(&self,turn_id:&str,text:&str)->Result<(),String>{
        if let Some(writer)=&self.workflow.writer{let turn=turn_id.to_owned();let text=text.to_owned();
            return writer.call(move|s|s.conversation_provisional(&turn,&text));}
        if text.len()>4096{return Err("Mock provisional text exceeds 4 KiB".into());}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let status:Option<String>=tx.query_row("SELECT status FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",
            params![turn_id,self.key],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        if status.as_deref()!=Some("reserved"){return Err("Provisional output has no reserved turn".into());}
        event(&tx,"conversation.mock_delta",None,&json!({"turn_id":turn_id,"text":text,
            "provisional":true,"authority":"none"}))?;
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn conversation_block_queued(&self,turn_id:&str,reason:&str)->Result<(),String>{
        if let Some(writer)=&self.workflow.writer{let turn=turn_id.to_owned();let reason=reason.to_owned();
            return writer.call(move|s|s.conversation_block_queued(&turn,&reason));}
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        let changed=tx.execute("UPDATE conversation_turns SET status='blocked',reason=?1 WHERE turn_id=?2 AND project_key=?3 AND status='queued'",
            params![reason,turn_id,self.key]).map_err(|e|e.to_string())?;
        if changed==1{event(&tx,"conversation.turn_blocked",None,&json!({"turn_id":turn_id,"reason":reason,
            "operator_action":"Inspect remaining call allowance and selected coordinator; no automatic retry"}))?;}
        tx.commit().map_err(|e|e.to_string())
    }
    pub fn conversation_has_answer(&self)->Result<bool,String>{
        let conn=self.workflow.connection()?;
        conn.query_row("SELECT exists(SELECT 1 FROM conversation_questions WHERE status='answered' AND project_key=?1)",
            [&self.key],|r|r.get(0)).map_err(|e|e.to_string())
    }
    pub fn conversation_retire_stranded_mock(&self)->Result<u64,String>{
        if let Some(writer)=&self.workflow.writer{return writer.call(|s|s.conversation_retire_stranded_mock());}
        let conn=self.workflow.connection()?;
        let mut stmt=conn.prepare("SELECT turn_id,packet_sha256 FROM conversation_turns WHERE status='reserved' AND project_key=?1")
            .map_err(|e|e.to_string())?;
        let rows=stmt.query_map([&self.key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))
            .map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        drop(stmt);drop(conn);
        for (turn,packet) in &rows{
            let directory=self.workflow.state.join("session/conversation").join(turn);
            let packet_file=directory.join("packet.json");
            if self.conversation_native(){continue;}
            let valid_packet=crate::execution::read(&packet_file).ok()
                .and_then(|record|canonical_sha256(&record).ok()).as_deref()==Some(packet.as_str());
            let saved=directory.join("result.json");
            let outcome=if !valid_packet{
                self.conversation_stop(turn,packet,"uncertain","packet-evidence-mismatch",
                    "Frozen mock packet evidence is missing or changed after restart; no result adoption or redispatch")?
            }else if saved.is_file(){
                let bytes=std::fs::read(&saved).map_err(|e|e.to_string())?;
                match serde_json::from_slice::<Value>(&bytes){
                    Ok(result)=>match self.conversation_apply(turn,packet,&result){
                        Ok(accepted) if accepted["status"]=="accepted"=>accepted,
                        Ok(refused)=>self.conversation_stop(turn,packet,"output-invalid",&sha256(&bytes),
                            &format!("Saved mock result refused after restart: {refused}"))?,
                        Err(error)=>self.conversation_stop(turn,packet,"output-invalid",&sha256(&bytes),
                            &format!("Saved mock result invalid after restart: {error}"))?,
                    },
                    Err(error)=>self.conversation_stop(turn,packet,"output-invalid",&sha256(&bytes),
                        &format!("Saved mock result is not JSON: {error}"))?,
                }
            }else{self.conversation_stop(turn,packet,"uncertain","no-completion-evidence",
                "Service restarted after mock turn reservation without a committed result; no redispatch")?};
            if outcome["status"]!="accepted"{return Err(format!("Stranded turn reconciliation could not be recorded: {outcome}"));}
        }
        Ok(rows.len() as u64)
    }
    pub fn conversation_submit(&self, body: &Value) -> Result<Value, String> {
        if self.continuous(){return self.dialogue_submit(body)}
        if let Some(writer) = &self.workflow.writer {
            let body = body.clone();
            return writer.call(move |s| s.conversation_submit(&body));
        }
        if let Err(rejected) = valid_command(body, "submit_message") {
            return Ok(rejected);
        }
        if !self.conversation_enabled()? {
            return Ok(reject("schema_mismatch", "Conversation requires schema 4"));
        }
        if let Some(old)=self.command_prior(body)?{return Ok(old);}
        if self.conversation_native(){
          let r=self.chat_readiness()?;
          if !matches!(r["status"].as_str(),Some("available"|"running")){return Ok(reject("coordinator_unavailable",r["reason"].as_str().unwrap_or("Conversation allowance exhausted")));}
          if body["payload"]["intent"].as_str().unwrap_or("discussion")!="discussion" || (!self.scope_enabled() && !body["payload"]["task_id"].is_null()){return Ok(reject("outside_increment","Only coordinator conversation is available; worker tasks, consultation and steering are deferred"));}
        }
        let text = body["payload"]["text"].as_str().unwrap_or("");
        if text.trim().is_empty() || text.len() > 32 * 1024 {
            return Ok(reject(
                "invalid_request",
                "Message missing or exceeds 32 KiB",
            ));
        }
        let intent=body["payload"]["intent"].as_str().unwrap_or("discussion");
        if !matches!(intent,"discussion"|"task_request"|"steer"){
            return Ok(reject("invalid_request","Unknown message intent"));
        }
        if intent=="steer" && body["payload"]["task_id"].as_str().is_none(){
            return Ok(reject("invalid_request","Steering requires a selected task"));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(outcome) = prior(&tx, body)? {
            return Ok(outcome);
        }
        let current = state_revision(&tx)?;
        if !self.scope_enabled() && body["expected_state_revision"] != current {
            return Ok(json!({"status":"rejected","code":"stale_state","state_revision":current}));
        }
        if metadata(&tx, "lifecycle_v1")?["phase"] != "running" {
            return Ok(reject(
                "session_paused",
                "Resume this project session before sending a message",
            ));
        }
        if let Some(task) = body["payload"]["task_id"].as_str() {
            let exists: bool = tx
                .query_row(
                    "SELECT exists(SELECT 1 FROM session_tasks WHERE task_id=?1)",
                    [task],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !exists {
                return Ok(reject(
                    "task_not_found",
                    "Selected task is not in this project",
                ));
            }
        }
        let queue: i64 = tx
            .query_row(
                "SELECT count(*) FROM conversation_turns WHERE status IN ('queued','reserved')",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if self.scope_enabled(){let pending:i64=tx.query_row("SELECT count(*) FROM conversation_turns WHERE status='queued'",[],|r|r.get(0)).map_err(|e|e.to_string())?;let remaining:i64=tx.query_row("SELECT limit_count-charged FROM conversation_budgets WHERE bucket='conversation'",[],|r|r.get(0)).map_err(|e|e.to_string())?;if pending>=remaining{return Ok(reject("budget_exhausted","Pending messages already use the remaining allowance; draft retained"));}}
        if queue >= 16 {
            return Ok(reject(
                "queue_full",
                "This project has 16 pending conversation turns",
            ));
        }
        let id = format!(
            "m-{}",
            &sha256(body["command_id"].as_str().unwrap().as_bytes())[..24]
        );
        let turn = format!("c-{id}");
        let (seq, next) = event(
            &tx,
            "conversation.message",
            None,
            &json!({"message_id":id,"role":"user","text":text,
            "target_task":body["payload"]["task_id"]}),
        )?;
        tx.execute(
            "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'user',?4,?5,?6,?7)",
            params![
                id,
                self.key,
                seq,
                text,
                sha256(text.as_bytes()),
                body["payload"]["task_id"].as_str(),
                turn
            ],
        )
        .map_err(|e| e.to_string())?;
        let generation: i64 = tx
            .query_row(
                "SELECT generation FROM conversation_bindings WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status,intent)
            VALUES(?1,1,?2,?3,?4,'queued',?5)",params![turn,self.key,id,generation,intent]).map_err(|e|e.to_string())?;
        if self.scope_enabled(){if let Some(refused)=self.scope_continuation(&tx,body)?{return Ok(refused)}}
        let outcome = json!({"status":"accepted","message_id":id,"turn_id":turn,"event_seq":seq,"state_revision":next});
        commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }
    pub fn conversation_reserve(
        &self,
        turn_id: &str,
        packet: &Value,
        bucket: &str,
    ) -> Result<Value, String> {
        if let Some(writer) = &self.workflow.writer {
            let turn = turn_id.to_owned();
            let packet = packet.clone();
            let bucket = bucket.to_owned();
            return writer.call(move |s| s.conversation_reserve(&turn, &packet, &bucket));
        }
        if !matches!(bucket, "conversation" | "consultation" | "task") {
            return Ok(reject("invalid_request", "Unknown budget bucket"));
        }
        if packet["project_key"] != self.key{return Ok(reject("invalid_packet","Coordinator packet belongs to a different project; no call reserved"));}
        if packet["turn_id"] != turn_id{return Ok(reject("invalid_packet","Coordinator packet belongs to a different turn; no call reserved"));}
        let modern=packet["execution"]["profile"]=="development-profiles-v1";
        let size=if modern{serde_json::to_vec(packet).map_err(|e|e.to_string())?.len()}else{canonical(packet).map_err(|e|e.to_string())?.len()};
        let limit=if modern{crate::development_context::MEMORY_LIMIT}else{crate::native_contract::PACKET_LIMIT};
        if size>limit{return Ok(reject("invalid_packet",&format!("Coordinator context is {size} bytes, exceeding the {limit}-byte application memory limit; no call reserved")));}
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let row:Option<(String,i64)>=tx.query_row("SELECT status,binding_generation FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",
            params![turn_id,self.key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        let Some((status, generation)) = row else {
            return Ok(reject("not_found", "Turn not found in this project"));
        };
        if status != "queued" {
            return Ok(reject(
                "already_reserved",
                "Turn has already left the queue",
            ));
        }
        if crate::provider_selection::enabled(self)&&crate::provider_selection::switch_pending(self){return Ok(reject("busy","Coordinator handover fences new reservations"));}
        if crate::provider_selection::enabled(self)&&packet["execution"]["selection"]["preferences_revision"]!=crate::provider_selection::preferences(self)?.1{return Ok(reject("busy","Preferences changed before reservation; prepare the same unreserved input again"));}
        let active:i64=tx.query_row("SELECT count(*) FROM conversation_turns WHERE status='reserved' AND budget_bucket IN ('conversation','consultation')",[],|r|r.get(0))
            .map_err(|e|e.to_string())?;
        if bucket != "task" && active > 0 {
            return Ok(reject(
                "busy",
                "Another coordinator or consultation turn is active",
            ));
        }
        let actual: i64 = tx
            .query_row(
                "SELECT generation FROM conversation_bindings WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if generation != actual {
            return Ok(reject(
                "stale_binding",
                "Coordinator binding changed before dispatch",
            ));
        }
        if self.scope_enabled() && !self.scope_reservation_valid(&tx,packet)?{return Ok(reject("stale_context","Frozen scope context changed before reservation; no launch"));}
        let (limit, charged): (i64, i64) = tx
            .query_row(
                "SELECT limit_count,charged FROM conversation_budgets WHERE bucket=?1",
                [bucket],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        if charged >= limit {
            return Ok(reject(
                "budget_exhausted",
                "No authorized calls remain in this bucket",
            ));
        }
        let digest = canonical_sha256(packet).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE conversation_budgets SET charged=charged+1 WHERE bucket=?1",
            [bucket],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("UPDATE conversation_turns SET status='reserved',budget_bucket=?1,packet_sha256=?2,context_sha256=?3 WHERE turn_id=?4",
            params![bucket,digest,packet["context_sha256"].as_str(),turn_id]).map_err(|e|e.to_string())?;
        let (seq, next) = event(
            &tx,
            "conversation.turn_reserved",
            None,
            &json!({"turn_id":turn_id,"bucket":bucket,
            "packet_sha256":digest,"binding_generation":generation,"selected_profile":{"provider":packet["provider"],"model":packet["model"],"effort":packet["execution"]["effort"],"selection":packet["execution"]["selection"]}}),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(
            json!({"status":"accepted","turn_id":turn_id,"packet_sha256":digest,
            "event_seq":seq,"state_revision":next,"charged":charged+1}),
        )
    }
    pub fn conversation_apply(
        &self,
        turn_id: &str,
        packet_sha: &str,
        result: &Value,
    ) -> Result<Value, String> {
        if let Some(writer) = &self.workflow.writer {
            let turn = turn_id.to_owned();
            let packet = packet_sha.to_owned();
            let result = result.clone();
            return writer.call(move |s| s.conversation_apply(&turn, &packet, &result));
        }
        if self.scope_enabled(){return self.scope_apply(turn_id,packet_sha,result);}
        let parsed = CoordinatorTurn::validate(result)
            .map_err(|e| format!("Invalid coordinator result: {e}"))?;
        if self.conversation_chat() && parsed.action.is_some(){return Ok(reject("outside_increment","Conversation result cannot request application actions"));}
        let result_sha = canonical_sha256(result).map_err(|e| e.to_string())?;
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let row:Option<(String,i64,Option<String>,Option<String>,String,Option<String>)>=tx.query_row(
            "SELECT status,binding_generation,packet_sha256,result_sha256,message_id,budget_bucket FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",
            params![turn_id,self.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))
            .optional().map_err(|e|e.to_string())?;
        let Some((status, generation, expected, old_result, message_id, bucket)) = row else {
            return Ok(reject("not_found", "Turn not found in this project"));
        };
        if status == "completed" && old_result.as_deref() == Some(&result_sha) {
            return Ok(json!({"status":"accepted","replayed":true,"turn_id":turn_id}));
        }
        if status != "reserved" {
            return Ok(reject("stale_turn", "Turn is not awaiting this result"));
        }
        if expected.as_deref() != Some(packet_sha) {
            return Ok(reject(
                "packet_mismatch",
                "Result does not match the frozen packet",
            ));
        }
        let actual: i64 = tx
            .query_row(
                "SELECT generation FROM conversation_bindings WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if generation != actual {
            return Ok(reject(
                "stale_binding",
                "Coordinator changed before result application",
            ));
        }
        if bucket.as_deref()==Some("consultation") && parsed.action.is_some(){
            return Ok(reject("invalid_result","Consultation may return advice, not an application action"));
        }
        if let Some(CoordinatorAction::AskQuestion { ref question, .. }) = parsed.action {
            let open: i64 = tx
                .query_row(
                    "SELECT count(*) FROM conversation_questions WHERE status='open'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if open >= 8 {
                return Ok(reject(
                    "question_limit",
                    "Eight project questions are already open",
                ));
            }
            if question.trim().is_empty() {
                return Ok(reject("invalid_result", "Question has no text"));
            }
        }
        if let Some(CoordinatorAction::ProposeSteering { ref task_id, .. }) = parsed.action {
            let exists: bool = tx
                .query_row(
                    "SELECT exists(SELECT 1 FROM session_tasks WHERE task_id=?1)",
                    [task_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !exists {
                return Ok(reject(
                    "task_not_found",
                    "Steering target is not a task in this project",
                ));
            }
        }
        tx.execute("DELETE FROM session_metadata WHERE name='claude_chat_stream_v1'",[]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE conversation_turns SET status='completed',result_sha256=?1,result_json=?2 WHERE turn_id=?3",
            params![result_sha,encode(result)?,turn_id]).map_err(|e|e.to_string())?;
        if !parsed.assistant_text.is_empty() {
            let id = format!("a-{}", &sha256(turn_id.as_bytes())[..24]);
            let (seq, _) = event(
                &tx,
                "conversation.message",
                None,
                &json!({"message_id":id,"role":if bucket.as_deref()==Some("consultation"){"consultant"}else{"assistant"},
                "text":parsed.assistant_text,"turn_id":turn_id,"evidence_refs":parsed.evidence_refs}),
            )?;
            tx.execute(
                "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,?4,?5,?6,NULL,?7)",
                params![
                    id,
                    self.key,
                    seq,
                    if bucket.as_deref()==Some("consultation"){"consultant"}else{"assistant"},
                    parsed.assistant_text,
                    sha256(parsed.assistant_text.as_bytes()),
                    turn_id
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(action) = parsed.action {
            let action_id = format!(
                "a-{}",
                &sha256(format!("{turn_id}:action").as_bytes())[..24]
            );
            let kind = match &action {
                CoordinatorAction::AskQuestion { .. } => "ask_question",
                CoordinatorAction::ProposeTask { .. } => "propose_task",
                CoordinatorAction::RequestConsultation { .. } => "request_consultation",
                CoordinatorAction::ProposeSteering { .. } => "propose_steering",
            };
            tx.execute(
                "INSERT INTO conversation_actions VALUES(?1,?2,?3,?4,'pending')",
                params![
                    action_id,
                    turn_id,
                    kind,
                    encode(&serde_json::to_value(&action).map_err(|e| e.to_string())?)?
                ],
            )
            .map_err(|e| e.to_string())?;
            match action {
                CoordinatorAction::AskQuestion {
                    question,
                    options,
                    rationale,
                } => {
                    let id = format!("q-{}", &sha256(turn_id.as_bytes())[..24]);
                    tx.execute("INSERT INTO conversation_questions VALUES(?1,1,?2,?3,?4,?5,'open',NULL,NULL)",
                        params![id,self.key,turn_id,question,encode(&json!(options))?]).map_err(|e|e.to_string())?;
                    event(
                        &tx,
                        "conversation.question",
                        None,
                        &json!({"question_id":id,"turn_id":turn_id,
                        "question":question,"options":options,"reason":rationale}),
                    )?;
                }
                CoordinatorAction::ProposeTask { brief } => {
                    let source:String=tx.query_row("SELECT text FROM conversation_messages WHERE message_id=?1 AND project_key=?2",
                        params![message_id,self.key],|r|r.get(0)).map_err(|e|e.to_string())?;
                    let task_id =
                        format!("t-{}", &sha256(format!("{turn_id}:task").as_bytes())[..24]);
                    let title = brief
                        .lines()
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(100)
                        .collect::<String>();
                    tx.execute(
                        "INSERT INTO session_tasks VALUES(?1,1,1,'proposed',?2,?3,NULL)",
                        params![task_id, title, source],
                    )
                    .map_err(|e| e.to_string())?;
                    tx.execute(
                        "INSERT INTO session_coordinator_calls VALUES(?1,?2,?3,'completed',?4)",
                        params![turn_id, task_id, packet_sha, result_sha],
                    )
                    .map_err(|e| e.to_string())?;
                    let proposal = json!({"task_version":1,"message_sha256":sha256(source.as_bytes()),"brief":brief,
                        "schema_version":1,"source_turn_id":turn_id,"project_key":self.key});
                    tx.execute(
                        "INSERT INTO session_proposals VALUES(?1,?2,?3,?4)",
                        params![task_id, turn_id, result_sha, encode(&proposal)?],
                    )
                    .map_err(|e| e.to_string())?;
                    tx.execute(
                        "UPDATE conversation_actions SET status='materialized' WHERE action_id=?1",
                        [&action_id],
                    )
                    .map_err(|e| e.to_string())?;
                    event(
                        &tx,
                        "task.proposed",
                        Some(&task_id),
                        &json!({"task_id":task_id,"brief":brief,
                        "source_turn_id":turn_id,"authorization":"required"}),
                    )?;
                }
                CoordinatorAction::RequestConsultation { .. }
                | CoordinatorAction::ProposeSteering { .. } => {
                    event(
                        &tx,
                        "conversation.action_proposed",
                        None,
                        &json!({"action_id":action_id,
                        "turn_id":turn_id,"kind":kind}),
                    )?;
                }
            }
        }
        if bucket.as_deref()==Some("consultation"){
            let original:String=tx.query_row("SELECT text FROM conversation_messages WHERE message_id=?1 AND project_key=?2",
                params![message_id,self.key],|r|r.get(0)).map_err(|e|e.to_string())?;
            let synthesis=format!("Synthesize the independent consultation for the proposed task. Brief: {original}\nAdvice: {}",parsed.assistant_text);
            let next_message=format!("m-{}",&sha256(format!("{turn_id}:synthesis").as_bytes())[..24]);
            let next_turn=format!("c-{next_message}");
            let (seq,_)=event(&tx,"conversation.synthesis_queued",None,&json!({"turn_id":next_turn,
                "source_consultation":turn_id,"text":synthesis}))?;
            tx.execute("INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'internal',?4,?5,NULL,?6)",
                params![next_message,self.key,seq,synthesis,sha256(synthesis.as_bytes()),next_turn]).map_err(|e|e.to_string())?;
            tx.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status)
                VALUES(?1,1,?2,?3,?4,'queued')",params![next_turn,self.key,next_message,generation])
                .map_err(|e|e.to_string())?;
        }
        event(
            &tx,
            "conversation.turn_completed",
            None,
            &json!({"turn_id":turn_id,"result_sha256":result_sha,
            "source_message_id":message_id}),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(
            json!({"status":"accepted","turn_id":turn_id,"result_sha256":result_sha,"replayed":false}),
        )
    }
    pub fn conversation_stop(
        &self,
        turn_id: &str,
        packet_sha: &str,
        kind: &str,
        output_sha: &str,
        reason: &str,
    ) -> Result<Value, String> {
        if let Some(writer) = &self.workflow.writer {
            let turn = turn_id.to_owned();
            let packet = packet_sha.to_owned();
            let kind = kind.to_owned();
            let output = output_sha.to_owned();
            let reason = reason.to_owned();
            return writer
                .call(move |s| s.conversation_stop(&turn, &packet, &kind, &output, &reason));
        }
        if self.continuous(){return self.dialogue_stop(turn_id,packet_sha,&json!({"schema_version":1,"category":"uncertain","known_completion":false,"settled":false,"capabilities_valid":false,"output_sha256":output_sha}),reason)}
        if !matches!(kind, "output-invalid" | "failed" | "uncertain" | "cancelled") || reason.trim().is_empty() {
            return Ok(reject(
                "invalid_request",
                "Invalid stopped-call classification",
            ));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let row:Option<(String,Option<String>,Option<String>,Option<String>)>=tx.query_row(
            "SELECT status,packet_sha256,result_sha256,reason FROM conversation_turns WHERE turn_id=?1 AND project_key=?2",
            params![turn_id,self.key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
            .optional().map_err(|e|e.to_string())?;
        let Some((status, expected, old_hash, old_reason)) = row else {
            return Ok(reject("not_found", "Turn not found in this project"));
        };
        if status == kind
            && old_hash.as_deref() == Some(output_sha)
            && old_reason.as_deref() == Some(reason)
        {
            return Ok(json!({"status":"accepted","replayed":true,"turn_id":turn_id}));
        }
        if status != "reserved" || expected.as_deref() != Some(packet_sha) {
            return Ok(reject(
                "stale_turn",
                "Stopped outcome does not match reserved turn and packet",
            ));
        }
        tx.execute(
            "UPDATE conversation_turns SET status=?1,result_sha256=?2,reason=?3 WHERE turn_id=?4",
            params![kind, output_sha, reason, turn_id],
        )
        .map_err(|e| e.to_string())?;
        let (seq, next) = event(
            &tx,
            "conversation.turn_stopped",
            None,
            &json!({"turn_id":turn_id,
            "packet_sha256":packet_sha,"output_sha256":output_sha,"status":kind,"reason":reason,
            "operator_action":"Inspect retained operation evidence; do not retry an uncertain call"}),
        )?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(
            json!({"status":"accepted","turn_id":turn_id,"event_seq":seq,"state_revision":next,"replayed":false}),
        )
    }
    pub fn conversation_authorize_consultation(&self,body:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let body=body.clone();
            return writer.call(move|s|s.conversation_authorize_consultation(&body));}
        if let Err(rejected)=valid_command(body,"authorize_consultation"){return Ok(rejected);}
        let action_id=body["payload"]["action_id"].as_str().unwrap_or("");
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if let Some(old)=prior(&tx,body)?{return Ok(old);}
        let current=state_revision(&tx)?;
        if body["expected_state_revision"]!=current{return Ok(json!({"status":"rejected","code":"stale_state","state_revision":current}));}
        let row:Option<(String,String,String)>=tx.query_row("SELECT kind,body_json,status FROM conversation_actions WHERE action_id=?1",
            [action_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
        let Some((kind,raw,status))=row else{return Ok(reject("not_found","Consultation action not found"));};
        if kind!="request_consultation"||status!="pending"{
            return Ok(reject("action_unavailable","This action is not an open consultation request"));
        }
        let (limit,charged):(i64,i64)=tx.query_row("SELECT limit_count,charged FROM conversation_budgets WHERE bucket='consultation'",[],
            |r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        if charged>=limit{return Ok(reject("budget_exhausted","No consultation call remains"));}
        let action:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let brief=action["payload"]["question"].as_str().ok_or("Invalid consultation brief")?;
        let message_id=format!("m-{}",&sha256(format!("{action_id}:consult").as_bytes())[..24]);
        let turn_id=format!("c-{message_id}");
        let generation:i64=tx.query_row("SELECT generation FROM conversation_bindings WHERE singleton=1",[],|r|r.get(0))
            .map_err(|e|e.to_string())?;
        tx.execute("UPDATE conversation_actions SET status='authorized' WHERE action_id=?1",[action_id])
            .map_err(|e|e.to_string())?;
        let (seq,next)=event(&tx,"conversation.consultation_authorized",None,&json!({"action_id":action_id,
            "turn_id":turn_id,"question":brief,"operator_command_id":body["command_id"]}))?;
        tx.execute("INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'consultation_request',?4,?5,NULL,?6)",
            params![message_id,self.key,seq,brief,sha256(brief.as_bytes()),turn_id]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status)
            VALUES(?1,1,?2,?3,?4,'queued')",params![turn_id,self.key,message_id,generation]).map_err(|e|e.to_string())?;
        let outcome=json!({"status":"accepted","turn_id":turn_id,"event_seq":seq,"state_revision":next});
        commit(&tx,body,&outcome)?;tx.commit().map_err(|e|e.to_string())?;
        Ok(outcome)
    }
    pub fn conversation_apply_steering(&self,body:&Value)->Result<Value,String>{
        if let Some(writer)=&self.workflow.writer{let body=body.clone();
            return writer.call(move|s|s.conversation_apply_steering(&body));}
        if let Err(rejected)=valid_command(body,"apply_conversation_steering"){return Ok(rejected);}
        let action_id=body["payload"]["action_id"].as_str().unwrap_or("");
        let mut conn=self.workflow.connection()?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if let Some(old)=prior(&tx,body)?{return Ok(old);}
        let current=state_revision(&tx)?;
        if body["expected_state_revision"]!=current{
            return Ok(json!({"status":"rejected","code":"stale_state","state_revision":current}));
        }
        let row:Option<(String,String,String,String)>=tx.query_row(
            "SELECT a.kind,a.body_json,a.status,a.turn_id FROM conversation_actions a WHERE a.action_id=?1",
            [action_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
            .optional().map_err(|e|e.to_string())?;
        let Some((kind,raw,status,turn))=row else{return Ok(reject("not_found","Steering action not found"));};
        if kind!="propose_steering"||status!="pending"{
            return Ok(reject("action_unavailable","No open steering suggestion matches this action"));
        }
        let action:Value=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let task=action["payload"]["task_id"].as_str().ok_or("Steering target missing")?;
        let suggestion=action["payload"]["text"].as_str().ok_or("Steering text missing")?;
        if suggestion.trim().is_empty()||suggestion.len()>8192{return Ok(reject("invalid_request","Steering text is empty or too large"));}
        let task_row:Option<(i64,i64,String,String,String,Option<String>)>=tx.query_row(
            "SELECT version,state_revision,status,title,body,run_id FROM session_tasks WHERE task_id=?1",
            [task],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))
            .optional().map_err(|e|e.to_string())?;
        let Some((version,task_rev,task_status,_title,old_body,run))=task_row else{
            return Ok(reject("task_not_found","Steering target is not in this project"));};
        if task_status!="proposed"||run.is_some(){
            return Ok(reject("frozen_work","This task has crossed authorization; create a new task instead"));
        }
        if body["expected_task_version"]!=version||body["expected_task_state_revision"]!=task_rev{
            return Ok(reject("stale_task","Task changed; review the suggestion and scope again"));
        }
        let original:Option<(String,String)>=tx.query_row(
            "SELECT result_sha256,body FROM session_proposals WHERE task_id=?1",[task],
            |r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        let Some((original_sha,raw_proposal))=original else{
            return Ok(reject("proposal_unavailable","Original proposal evidence missing"));};
        let original:Value=serde_json::from_str(&raw_proposal).map_err(|e|e.to_string())?;
        let updated=format!("{old_body}\n\nOperator-approved steering before authorization:\n{suggestion}");
        if updated.len()>32768{return Ok(reject("invalid_request","Combined task input exceeds 32 KiB"));}
        let new_id=format!("t-{}",&sha256(format!("{action_id}:steered").as_bytes())[..24]);
        let brief=format!("{}\nSteering: {suggestion}",original["brief"].as_str().unwrap_or("Task proposal"));
        let title=brief.lines().next().unwrap_or("").chars().take(100).collect::<String>();
        tx.execute("UPDATE session_tasks SET status='cancelled',state_revision=state_revision+1 WHERE task_id=?1",[task])
            .map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO session_tasks VALUES(?1,1,1,'proposed',?2,?3,NULL)",
            params![new_id,title,updated]).map_err(|e|e.to_string())?;
        let proposal=json!({"schema_version":2,"task_version":1,"message_sha256":sha256(updated.as_bytes()),
            "brief":brief,"project_key":self.key,"source_action_id":action_id,
            "prior_task_id":task,"original_proposal_result_sha256":original_sha,"source_turn_id":turn});
        tx.execute("INSERT INTO session_proposals VALUES(?1,?2,?3,?4)",
            params![new_id,turn,original_sha,encode(&proposal)?]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE conversation_actions SET status='applied' WHERE action_id=?1",[action_id])
            .map_err(|e|e.to_string())?;
        let (seq,next)=event(&tx,"conversation.steering_applied",Some(&new_id),
            &json!({"action_id":action_id,"prior_task_id":task,"new_task_id":new_id,
                "original_proposal_result_sha256":original_sha,"authorization":"still_required"}))?;
        event(&tx,"task.proposed",Some(&new_id),&json!({"task_id":new_id,"brief":brief,
            "source_action_id":action_id,"authorization":"required"}))?;
        let outcome=json!({"status":"accepted","prior_task_id":task,"task_id":new_id,
            "event_seq":seq,"state_revision":next+1});
        commit(&tx,body,&outcome)?;
        tx.commit().map_err(|e|e.to_string())?;
        Ok(outcome)
    }
    pub fn conversation_answer(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer) = &self.workflow.writer {
            let body = body.clone();
            return writer.call(move |s| s.conversation_answer(&body));
        }
        if self.scope_enabled(){return self.scope_answer(body);}
        if let Err(rejected) = valid_command(body, "answer_conversation") {
            return Ok(rejected);
        }
        let id = body["payload"]["question_id"].as_str().unwrap_or("");
        let answer = body["payload"]["answer"].as_str().unwrap_or("");
        if answer.trim().is_empty() || answer.len() > 16 * 1024 {
            return Ok(reject("invalid_request", "Answer missing or too large"));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(old) = prior(&tx, body)? {
            return Ok(old);
        }
        let current = state_revision(&tx)?;
        if body["expected_state_revision"] != current {
            return Ok(json!({"status":"rejected","code":"stale_state","state_revision":current}));
        }
        let status: Option<String> = tx
            .query_row(
                "SELECT status FROM conversation_questions WHERE question_id=?1 AND project_key=?2",
                params![id, self.key],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if status.as_deref() != Some("open") {
            return Ok(reject(
                "question_unavailable",
                "No open question has this ID",
            ));
        }
        let queue: i64 = tx
            .query_row(
                "SELECT count(*) FROM conversation_turns WHERE status IN ('queued','reserved')",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if queue >= 16 {
            return Ok(reject(
                "queue_full",
                "Answer remains pending: conversation queue is full",
            ));
        }
        tx.execute("UPDATE conversation_questions SET status='answered',answer=?1,answer_command_id=?2 WHERE question_id=?3",
            params![answer,body["command_id"].as_str(),id]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE conversation_actions SET status='answered' WHERE turn_id=(SELECT turn_id FROM conversation_questions WHERE question_id=?1) AND kind='ask_question' AND status='pending'",
            [id]).map_err(|e|e.to_string())?;
        let (seq, next) = event(
            &tx,
            "conversation.answer",
            None,
            &json!({"question_id":id,"answer":answer,
            "answer_sha256":sha256(answer.as_bytes())}),
        )?;
        let message_id = format!(
            "m-{}",
            &sha256(format!("answer:{}", body["command_id"].as_str().unwrap()).as_bytes())[..24]
        );
        let turn_id = format!("c-{message_id}");
        let rendered = format!("Answer to {id}: {answer}");
        tx.execute(
            "INSERT INTO conversation_messages VALUES(?1,1,?2,?3,'user',?4,?5,NULL,?6)",
            params![
                message_id,
                self.key,
                seq,
                rendered,
                sha256(rendered.as_bytes()),
                turn_id
            ],
        )
        .map_err(|e| e.to_string())?;
        let generation: i64 = tx
            .query_row(
                "SELECT generation FROM conversation_bindings WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO conversation_turns(turn_id,schema_version,project_key,message_id,binding_generation,status)
            VALUES(?1,1,?2,?3,?4,'queued')",params![turn_id,self.key,message_id,generation]).map_err(|e|e.to_string())?;
        let outcome = json!({"status":"accepted","question_id":id,"turn_id":turn_id,"event_seq":seq,"state_revision":next});
        commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }
    fn chat_authorize(&self,body:&Value)->Result<Value,String>{
        if let Err(e)=valid_command(body,"select_coordinator"){return Ok(e);}
        if let Some(old)=self.command_prior(body)?{return Ok(old);}
        let p=crate::claude_chat::profile(&self.project)?;
        let hash=canonical_sha256(&p).map_err(|e|e.to_string())?;
        if body["payload"]["authorize"]!=true || body["payload"]["profile_sha256"]!=hash{return Ok(reject("profile_mismatch","Review the exact coordinator settings before authorization"));}
        if let Err(e)=crate::claude_chat::preflight(&p){return Ok(reject("coordinator_preflight",&e));}
        let mut conn=self.workflow.connection()?;let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
        if let Some(old)=prior(&tx,body)?{return Ok(old);}
        let generation:i64=tx.query_row("SELECT generation FROM conversation_bindings WHERE singleton=1",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if body["expected_generation"]!=generation{return Ok(reject("stale_binding","Coordinator binding changed"));}
        let charged:i64=tx.query_row("SELECT charged FROM conversation_budgets WHERE bucket='conversation'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if charged>0{return Ok(reject("allowance_frozen","The frozen trial allowance cannot be reset or renewed"));}
        tx.execute("UPDATE session_metadata SET value=?1 WHERE name='claude_chat_authorization_v1'",[encode(&json!({"schema_version":1,"authorized":true,"profile_sha256":hash,"command_id":body["command_id"],"call_limit":p["call_limit"]}))?]).map_err(|e|e.to_string())?;
        let (_,rev)=event(&tx,"conversation.authorized",None,&json!({"profile_sha256":hash,"call_limit":p["call_limit"]}))?;
        let outcome=json!({"status":"accepted","state_revision":rev,"call_limit":p["call_limit"]});commit(&tx,body,&outcome)?;tx.commit().map_err(|e|e.to_string())?;Ok(outcome)
    }
    pub fn conversation_select_coordinator(&self, body: &Value) -> Result<Value, String> {
        if let Some(writer) = &self.workflow.writer {
            let body = body.clone();
            return writer.call(move |s| s.conversation_select_coordinator(&body));
        }
        if self.conversation_native(){return self.chat_authorize(body);}
        if let Err(rejected) = valid_command(body, "select_coordinator") {
            return Ok(rejected);
        }
        let provider = body["payload"]["provider"].as_str().unwrap_or("");
        let model = body["payload"]["model"].as_str().unwrap_or("");
        let effort = body["payload"]["effort"].as_str().unwrap_or("");
        if provider != "fake" || model != "deterministic" || effort != "none" {
            return Ok(reject(
                "native_admission_closed",
                "Only the isolated fake coordinator is admitted",
            ));
        }
        let mut conn = self.workflow.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        if let Some(old) = prior(&tx, body)? {
            return Ok(old);
        }
        let active: i64 = tx
            .query_row(
                "SELECT count(*) FROM conversation_turns WHERE status='reserved'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if active > 0 {
            return Ok(reject(
                "active_call",
                "Resolve the active call before replacing coordinator",
            ));
        }
        let generation: i64 = tx
            .query_row(
                "SELECT generation FROM conversation_bindings WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if body["expected_generation"] != generation {
            return Ok(reject("stale_binding", "Coordinator generation changed"));
        }
        tx.execute(
            "UPDATE conversation_bindings SET generation=generation+1 WHERE singleton=1",
            [],
        )
        .map_err(|e| e.to_string())?;
        let queued=tx.execute("UPDATE conversation_turns SET binding_generation=?1 WHERE status='queued' AND project_key=?2",
            params![generation+1,self.key]).map_err(|e|e.to_string())?;
        let (seq, next) = event(
            &tx,
            "conversation.binding",
            None,
            &json!({"generation":generation+1,"queued_turns_rebound":queued,
            "provider":provider,"model":model,"effort":effort}),
        )?;
        let outcome = json!({"status":"accepted","generation":generation+1,"event_seq":seq,"state_revision":next});
        commit(&tx, body, &outcome)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs,path::Path};
    use tempfile::TempDir;
    fn fixture(name: &str) -> (TempDir, Session) {
        let root = TempDir::new().unwrap();
        let project = root.path().join(name);
        fs::create_dir_all(&project).unwrap();
        let state = project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        crate::execution::write(
            &state.join("state-contract.json"),
            &json!({"schema_version":1,
            "state_schema":4,"execution_host":"systemd-user-v1","project_key":name}),
            true,
        )
        .unwrap();
        let session = Session::open(&project, &state, name, "fake-release").unwrap();
        session.conversation_schema().unwrap();
        (root, session)
    }
    fn submit(session: &Session, id: &str, rev: u64, text: &str) -> Value {
        session
            .conversation_submit(&json!({"command_id":id,"action":"submit_message",
            "expected_state_revision":rev,"payload":{"text":text}}))
            .unwrap()
    }
    #[test]
    fn discussion_is_not_a_task_and_replay_is_once() {
        let (_root, session) = fixture("project-a");
        let first = submit(&session, "first", 0, "What do you think?");
        assert_eq!(first["status"], "accepted");
        assert_eq!(first, submit(&session, "first", 0, "What do you think?"));
        assert_eq!(
            submit(&session, "first", 0, "Different")["code"],
            "idempotency_conflict"
        );
        assert_eq!(submit(&session, "second", 0, "More")["code"], "stale_state");
        assert!(
            session.snapshot().unwrap()["tasks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let reopened = Session::open_existing(
            &session.project,
            &session.workflow.state,
            "project-a",
            "fake-release",
        )
        .unwrap();
        assert_eq!(
            reopened.conversation_snapshot().unwrap()["messages"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn reservation_answer_binding_and_project_fences() {
        let (_a, session) = fixture("project-a");
        let (_b, other) = fixture("project-b");
        let first = submit(&session, "first", 0, "Need advice");
        let turn = first["turn_id"].as_str().unwrap();
        let packet = json!({"project_key":"project-a","turn_id":turn});
        let reserved = session
            .conversation_reserve(turn, &packet, "conversation")
            .unwrap();
        assert_eq!(reserved["status"], "accepted");
        assert_eq!(
            session
                .conversation_reserve(turn, &packet, "conversation")
                .unwrap()["code"],
            "already_reserved"
        );
        assert_eq!(
            other
                .conversation_reserve(turn, &packet, "conversation")
                .unwrap()["code"],
            "invalid_packet"
        );
        assert_eq!(session.conversation_apply(turn,"wrong",&json!({"schema_version":1,"assistant_text":"No","evidence_refs":[],"action":null})).unwrap()["code"],"packet_mismatch");
        let result = json!({"schema_version":1,"assistant_text":"A choice is needed","evidence_refs":[],"action":{"kind":"ask_question",
            "payload":{"question":"Which option?","rationale":"Changes behavior","options":["A","B"]}}});
        let applied = session
            .conversation_apply(turn, reserved["packet_sha256"].as_str().unwrap(), &result)
            .unwrap();
        assert_eq!(applied["status"], "accepted");
        assert_eq!(
            session
                .conversation_apply(turn, reserved["packet_sha256"].as_str().unwrap(), &result)
                .unwrap()["replayed"],
            true
        );
        let question = session.conversation_snapshot().unwrap()["questions"][0]["question_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let rev = session.snapshot().unwrap()["state_revision"]
            .as_u64()
            .unwrap();
        let answer = json!({"command_id":"answer-1","action":"answer_conversation","expected_state_revision":rev,
            "payload":{"question_id":question,"answer":"B"}});
        assert_eq!(
            session.conversation_answer(&answer).unwrap()["status"],
            "accepted"
        );
        assert_eq!(
            session.conversation_answer(&answer).unwrap()["status"],
            "accepted"
        );
        assert!(
            session.conversation_snapshot().unwrap()["questions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let handoff=session.conversation_handoff().unwrap();
        assert_eq!(handoff["schema_version"],1);
        assert_eq!(handoff["accepted_decisions"][0]["question_id"],question);
        assert_eq!(handoff["accepted_decisions"][0]["answer"],"B");
        let reopened=Session::open_existing(&session.project,&session.workflow.state,
            "project-a","fake-release").unwrap();
        assert_eq!(reopened.conversation_handoff().unwrap()["accepted_decisions"][0]["answer"],"B");
        assert_eq!(
            other.conversation_snapshot().unwrap()["messages"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
    #[test]
    fn overlapping_reservations_do_not_overspend() {
        let (_root, session) = fixture("project-a");
        let first = submit(&session, "one", 0, "first");
        let second = submit(
            &session,
            "two",
            first["state_revision"].as_u64().unwrap(),
            "second",
        );
        let a = first["turn_id"].as_str().unwrap();
        let b = second["turn_id"].as_str().unwrap();
        assert_eq!(
            session
                .conversation_reserve(
                    a,
                    &json!({"project_key":"project-a","turn_id":a}),
                    "conversation"
                )
                .unwrap()["status"],
            "accepted"
        );
        assert_eq!(
            session
                .conversation_reserve(
                    b,
                    &json!({"project_key":"project-a","turn_id":b}),
                    "conversation"
                )
                .unwrap()["code"],
            "busy"
        );
        let budget = &session.conversation_snapshot().unwrap()["budgets"]["conversation"];
        assert_eq!(budget["charged"], 1);
        assert_eq!(budget["remaining"], 5);
    }
    #[test]
    fn failed_turn_retains_debit_and_does_not_replay() {
        let (_root, session) = fixture("project-a");
        let submitted = submit(&session, "first", 0, "Discussion");
        let turn = submitted["turn_id"].as_str().unwrap();
        let packet = json!({"project_key":"project-a","turn_id":turn});
        let reserved = session
            .conversation_reserve(turn, &packet, "conversation")
            .unwrap();
        let digest = reserved["packet_sha256"].as_str().unwrap();
        let first = session
            .conversation_stop(
                turn,
                digest,
                "output-invalid",
                "output-hash",
                "Malformed result",
            )
            .unwrap();
        assert_eq!(first["status"], "accepted");
        assert_eq!(
            session
                .conversation_stop(
                    turn,
                    digest,
                    "output-invalid",
                    "output-hash",
                    "Malformed result"
                )
                .unwrap()["replayed"],
            true
        );
        assert_eq!(
            session
                .conversation_apply(
                    turn,
                    digest,
                    &json!({"schema_version":1,"assistant_text":"Late","action":null})
                )
                .unwrap()["code"],
            "stale_turn"
        );
        assert_eq!(
            session.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            1
        );
        assert!(
            session.snapshot().unwrap()["recent_events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["type"] == "conversation.turn_stopped")
        );
    }
    #[test]
    fn validated_proposal_creates_one_unadmitted_task() {
        let (_root, session) = fixture("project-a");
        let submitted = submit(&session, "first", 0, "Build a small page");
        let turn = submitted["turn_id"].as_str().unwrap();
        let reserved = session
            .conversation_reserve(
                turn,
                &json!({"project_key":"project-a","turn_id":turn}),
                "conversation",
            )
            .unwrap();
        let result = json!({"schema_version":1,"assistant_text":"Here is a scope to review.","action":{
            "kind":"propose_task","payload":{"brief":"Build a small page with fixed data"}}});
        assert_eq!(
            session
                .conversation_apply(turn, reserved["packet_sha256"].as_str().unwrap(), &result)
                .unwrap()["status"],
            "accepted"
        );
        let tasks = session.snapshot().unwrap()["tasks"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["status"], "proposed");
        assert!(tasks[0]["run_id"].is_null());
        assert_eq!(
            session
                .conversation_apply(turn, reserved["packet_sha256"].as_str().unwrap(), &result)
                .unwrap()["replayed"],
            true
        );
        assert_eq!(
            session.snapshot().unwrap()["tasks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn exhaustion_and_binding_change_refuse_before_new_call() {
        let (_root, session) = fixture("project-a");
        for index in 0..6 {
            let rev = session.snapshot().unwrap()["state_revision"]
                .as_u64()
                .unwrap();
            let submitted = submit(&session, &format!("msg-{index}"), rev, "Talk");
            let turn = submitted["turn_id"].as_str().unwrap();
            let reserve = session
                .conversation_reserve(
                    turn,
                    &json!({"project_key":"project-a","turn_id":turn}),
                    "conversation",
                )
                .unwrap();
            let result = json!({"schema_version":1,"assistant_text":"Reply","action":null});
            session
                .conversation_apply(turn, reserve["packet_sha256"].as_str().unwrap(), &result)
                .unwrap();
        }
        let rev = session.snapshot().unwrap()["state_revision"]
            .as_u64()
            .unwrap();
        let seventh = submit(&session, "seventh", rev, "One more");
        let turn = seventh["turn_id"].as_str().unwrap();
        let packet = json!({"project_key":"project-a","turn_id":turn});
        assert_eq!(
            session
                .conversation_reserve(turn, &packet, "conversation")
                .unwrap()["code"],
            "budget_exhausted"
        );
        assert_eq!(
            session.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
            6
        );
        let changed = session
            .conversation_select_coordinator(&json!({"command_id":"bind-2",
            "action":"select_coordinator","expected_generation":1,"payload":{"provider":"fake",
            "model":"deterministic","effort":"none"}}))
            .unwrap();
        assert_eq!(changed["generation"], 2);
        assert_eq!(
            session.conversation_reserve(turn, &packet, "conversation").unwrap()["code"],
            "budget_exhausted"
        );
    }

    #[test]
    fn stranded_mock_result_is_adopted_once_and_missing_result_stays_uncertain(){
        let (_root,session)=fixture("project-a");
        let first=submit(&session,"first",0,"Discuss");
        let turn=first["turn_id"].as_str().unwrap();
        let packet=json!({"project_key":"project-a","turn_id":turn,"mode":"mock"});
        let directory=session.workflow.state.join("session/conversation").join(turn);
        crate::execution::write(&directory.join("packet.json"),&packet,true).unwrap();
        let reserved=session.conversation_reserve(turn,&packet,"conversation").unwrap();
        assert_eq!(reserved["status"],"accepted");
        let result=json!({"schema_version":1,"assistant_text":"Mock answer","evidence_refs":[],"action":null});
        crate::execution::write(&directory.join("result.json"),&result,true).unwrap();
        assert_eq!(session.conversation_retire_stranded_mock().unwrap(),1);
        assert_eq!(session.conversation_retire_stranded_mock().unwrap(),0);
        assert_eq!(session.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],1);
        assert_eq!(session.conversation_snapshot().unwrap()["messages"].as_array().unwrap().len(),2);

        let rev=session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        let second=submit(&session,"second",rev,"Another");
        let turn=second["turn_id"].as_str().unwrap();
        let packet=json!({"project_key":"project-a","turn_id":turn,"mode":"mock"});
        let directory=session.workflow.state.join("session/conversation").join(turn);
        crate::execution::write(&directory.join("packet.json"),&packet,true).unwrap();
        assert_eq!(session.conversation_reserve(turn,&packet,"conversation").unwrap()["status"],"accepted");
        assert_eq!(session.conversation_retire_stranded_mock().unwrap(),1);
        assert_eq!(session.conversation_next_queued().unwrap(),None);
        let turns=session.conversation_snapshot().unwrap()["turns"].as_array().unwrap().clone();
        assert_eq!(turns[0]["status"],"uncertain");
        assert_eq!(session.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],2);
    }
    #[test]
    fn explicit_mock_replacement_preserves_queued_turn_and_budget(){
        let (_root,session)=fixture("project-a");
        let first=submit(&session,"first",0,"Discuss after replacement");
        let command=json!({"command_id":"replace","action":"select_coordinator",
            "expected_generation":1,"payload":{"provider":"fake","model":"deterministic","effort":"none"}});
        let changed=session.conversation_select_coordinator(&command).unwrap();
        assert_eq!(changed["status"],"accepted");
        assert_eq!(session.conversation_select_coordinator(&command).unwrap(),changed);
        assert_eq!(session.conversation_snapshot().unwrap()["binding"]["generation"],2);
        assert_eq!(session.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],0);
        let turn=first["turn_id"].as_str().unwrap();
        assert_eq!(session.conversation_reserve(turn,&json!({"project_key":"project-a","turn_id":turn}),
            "conversation").unwrap()["status"],"accepted");
    }
    #[test]
    fn steering_creates_linked_unadmitted_proposal_without_rewriting_original(){
        let (_root,session)=fixture("project-a");
        let first=submit(&session,"task",0,"Build a fixed weather page");
        let turn=first["turn_id"].as_str().unwrap();
        let reserve=session.conversation_reserve(turn,&json!({"project_key":"project-a","turn_id":turn}),"conversation").unwrap();
        let result=json!({"schema_version":1,"assistant_text":"Scope ready","action":{
            "kind":"propose_task","payload":{"brief":"Static weather page"}}});
        assert_eq!(session.conversation_apply(turn,reserve["packet_sha256"].as_str().unwrap(),&result)
            .unwrap()["status"],"accepted");
        let old=session.snapshot().unwrap()["tasks"][0]["task_id"].as_str().unwrap().to_owned();
        let original=session.proposal(&old).unwrap().unwrap();
        let rev=session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        let message=session.conversation_submit(&json!({"command_id":"steer-msg","action":"submit_message",
            "expected_state_revision":rev,"payload":{"intent":"steer","task_id":old,
                "text":"Use only local sample data"}})).unwrap();
        let turn=message["turn_id"].as_str().unwrap();
        let reserve=session.conversation_reserve(turn,&json!({"project_key":"project-a","turn_id":turn}),"conversation").unwrap();
        let result=json!({"schema_version":1,"assistant_text":"Steering suggestion ready","action":{
            "kind":"propose_steering","payload":{"task_id":old,"text":"Use only local sample data"}}});
        assert_eq!(session.conversation_apply(turn,reserve["packet_sha256"].as_str().unwrap(),&result)
            .unwrap()["status"],"accepted");
        let action=session.conversation_snapshot().unwrap()["actions"].as_array().unwrap()
            .iter().find(|a|a["kind"]=="propose_steering").unwrap()["action_id"].clone();
        let rev=session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        let command=json!({"command_id":"apply-steer","action":"apply_conversation_steering",
            "expected_state_revision":rev,"expected_task_version":1,"expected_task_state_revision":1,
            "payload":{"action_id":action}});
        assert_eq!(session.conversation_apply_steering(&json!({"expected_task_state_revision":9,
            "command_id":"wrong-steer","action":"apply_conversation_steering","expected_state_revision":rev,
            "expected_task_version":1,"payload":{"action_id":action}})).unwrap()["code"],"stale_task");
        let applied=session.conversation_apply_steering(&command).unwrap();
        assert_eq!(applied["status"],"accepted");
        assert_eq!(session.conversation_apply_steering(&command).unwrap(),applied);
        let next=applied["task_id"].as_str().unwrap();
        assert_eq!(session.task(&old).unwrap().unwrap()["status"],"cancelled");
        assert_eq!(session.task(next).unwrap().unwrap()["status"],"proposed");
        assert_eq!(session.proposal(&old).unwrap().unwrap(),original);
        assert_eq!(session.proposal(next).unwrap().unwrap()["body"]["prior_task_id"],old);
        assert_eq!(session.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],2);
    }

    #[test]
    fn foreign_steering_target_cannot_be_applied() {
        let (_root, session) = fixture("project-a");
        let submitted = submit(&session, "first", 0, "Steer work");
        let turn = submitted["turn_id"].as_str().unwrap();
        let reserved = session
            .conversation_reserve(
                turn,
                &json!({"project_key":"project-a","turn_id":turn}),
                "conversation",
            )
            .unwrap();
        let result = json!({"schema_version":1,"assistant_text":"I suggest a change",
            "action":{"kind":"propose_steering","payload":{"task_id":"foreign-task","text":"Edit it"}}});
        assert_eq!(
            session
                .conversation_apply(turn, reserved["packet_sha256"].as_str().unwrap(), &result)
                .unwrap()["code"],
            "task_not_found"
        );
        assert!(
            session.snapshot().unwrap()["tasks"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn native_chat_authority_is_closed_bounded_and_cannot_create_work(){
        let root=TempDir::new().unwrap();let project=root.path().join("p");let state=project.join(".waywr/state");
        fs::create_dir_all(&state).unwrap();
        crate::execution::write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":5,"execution_host":"systemd-user-v1","project_key":"project-a"}),true).unwrap();
        let binary=Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/claude-conversation/mock-cli.py").canonicalize().unwrap();
        let profile=json!({"schema_version":1,"profile":"claude-chat-v1","mode":"mock-cli","provider":"claude","model":"claude-sonnet-5-5","effort":"medium","call_limit":3,"binary":binary,"auth_home":root.path(),"auth_config_dir":root.path()});
        crate::execution::write(&project.join(".waywr/coordinator.json"),&profile,true).unwrap();
        let session=Session::open(&project,&state,"project-a","chat-release").unwrap();session.conversation_schema().unwrap();
        assert_eq!(session.chat_readiness().unwrap()["status"],"awaiting-authorization");
        assert_eq!(submit(&session,"closed",0,"Hello")["code"],"coordinator_unavailable");
        let auth=json!({"command_id":"allow","action":"select_coordinator","expected_generation":1,"payload":{"authorize":true,"profile_sha256":canonical_sha256(&profile).unwrap()}});
        assert_eq!(session.conversation_select_coordinator(&auth).unwrap()["status"],"accepted");
        assert_eq!(session.conversation_select_coordinator(&auth).unwrap()["status"],"accepted");
        let rev=session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
        let result=submit(&session,"one",rev,"Please build something");let turn=result["turn_id"].as_str().unwrap();
        let packet=json!({"project_key":"project-a","turn_id":turn});let reserve=session.conversation_reserve(turn,&packet,"conversation").unwrap();
        assert_eq!(session.conversation_apply(turn,reserve["packet_sha256"].as_str().unwrap(),&json!({"schema_version":1,"assistant_text":"Can discuss; execution unavailable","evidence_refs":[],"action":{"kind":"propose_task","payload":{"brief":"bad"}}})).unwrap()["code"],"outside_increment");
        let applied=json!({"schema_version":1,"assistant_text":"How should it look?","evidence_refs":[],"action":null});
        session.conversation_apply(turn,reserve["packet_sha256"].as_str().unwrap(),&applied).unwrap();
        assert!(session.snapshot().unwrap()["tasks"].as_array().unwrap().is_empty());
        let renewed=json!({"command_id":"renew","action":"select_coordinator","expected_generation":1,"payload":auth["payload"]});
        assert_eq!(session.conversation_select_coordinator(&renewed).unwrap()["code"],"allowance_frozen");
        for n in 2..=3{
            let next_rev=session.snapshot().unwrap()["state_revision"].as_u64().unwrap();
            let next=submit(&session,&format!("turn-{n}"),next_rev,"More discussion");let id=next["turn_id"].as_str().unwrap();
            let frozen=json!({"project_key":"project-a","turn_id":id});let hold=session.conversation_reserve(id,&frozen,"conversation").unwrap();
            session.conversation_apply(id,hold["packet_sha256"].as_str().unwrap(),&applied).unwrap();
        }
        assert_eq!(session.chat_readiness().unwrap()["status"],"exhausted");
        assert_eq!(submit(&session,"one",rev,"Please build something"),result,"Replay must precede readiness/exhaustion checks");
        assert_eq!(session.conversation_select_coordinator(&auth).unwrap()["status"],"accepted");
        let reopened=Session::open_existing(&project,&state,"project-a","chat-release").unwrap();
        assert_eq!(reopened.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],3);
        assert_eq!(reopened.conversation_snapshot().unwrap()["messages"].as_array().unwrap().len(),6);
    }

    #[test]
    fn cancelled_turn_is_durable_and_replayed_without_refunding_budget(){
        let (_root,s)=fixture("project-a");let a=submit(&s,"first",0,"Discuss");let id=a["turn_id"].as_str().unwrap();
        let r=s.conversation_reserve(id,&json!({"project_key":"project-a","turn_id":id}),"conversation").unwrap();
        let hash=r["packet_sha256"].as_str().unwrap();
        let stop=s.conversation_stop(id,hash,"cancelled","receipt-hash","Operator interrupted the call").unwrap();assert_eq!(stop["status"],"accepted");
        assert_eq!(s.conversation_stop(id,hash,"cancelled","receipt-hash","Operator interrupted the call").unwrap()["replayed"],true);
        let state=s.conversation_snapshot().unwrap();assert_eq!(state["turns"][0]["status"],"cancelled");assert_eq!(state["budgets"]["conversation"]["charged"],1);
    }

}
