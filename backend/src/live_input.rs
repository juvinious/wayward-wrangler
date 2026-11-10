//! Durable steering for one reserved conversation attempt. Human input changes
//! agent context, never the frozen model, tools, scope or execution authority.
use crate::{
    codec::sha256,
    conversation_store::{commit, encode, event, prior, reject, valid_command},
    execution::{read, write},
    host_attempt::Attempt,
    session_store::Session,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn schema(s: &Session) -> Result<(), String> {
    s.workflow
        .connection()?
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS conversation_live_inputs(
        input_id TEXT PRIMARY KEY,turn_id TEXT NOT NULL,subject_id TEXT NOT NULL,text TEXT NOT NULL,
        status TEXT NOT NULL,directory TEXT,scope_json TEXT,seq INTEGER NOT NULL);",
        )
        .map_err(|e| e.to_string())
}
pub fn rows(c: &rusqlite::Connection) -> Result<Vec<Value>, String> {
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='conversation_live_inputs')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !exists {
        return Ok(vec![]);
    }
    let mut q=c.prepare("SELECT input_id,turn_id,subject_id,text,status FROM conversation_live_inputs WHERE status IN ('pending','delivering') OR input_id IN (SELECT input_id FROM conversation_live_inputs WHERE status='held' ORDER BY seq DESC LIMIT 32) OR input_id IN (SELECT input_id FROM conversation_live_inputs WHERE status='applied' ORDER BY seq DESC LIMIT 16) ORDER BY seq DESC").map_err(|e|e.to_string())?;
    let mut rows=q.query_map([],|r|Ok(json!({"input_id":r.get::<_,String>(0)?,"turn_id":r.get::<_,String>(1)?,
        "subject_id":r.get::<_,String>(2)?,"text":r.get::<_,String>(3)?.chars().take(4096).collect::<String>(),"status":r.get::<_,String>(4)?,"delivery":"steer"}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    rows.reverse();
    Ok(rows)
}
fn withdraw_pending(directory: &str, id: &str) -> Result<bool, String> {
    let root = Path::new(directory).join("live-input");
    let unclaimed = || {
        !root.join("claimed").join(format!("{id}.json")).exists()
            && !root.join("applied").join(format!("{id}.json")).exists()
    };
    if !unclaimed() {
        return Ok(false);
    }
    fs::create_dir_all(root.join("withdrawn")).map_err(|e| e.to_string())?;
    match fs::rename(
        root.join("pending").join(format!("{id}.json")),
        root.join("withdrawn").join(format!("{id}.json")),
    ) {
        Ok(()) => Ok(unclaimed()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(!root.join("claimed").join(format!("{id}.json")).exists()
                && !root.join("applied").join(format!("{id}.json")).exists())
        }
        Err(e) => Err(e.to_string()),
    }
}
pub fn command(s: &Session, body: &Value) -> Result<Value, String> {
    if let Some(w) = &s.workflow.writer {
        let b = body.clone();
        return w.call(move |s| command(&s, &b));
    }
    let action = body["action"].as_str().unwrap_or("");
    if let Err(e) = valid_command(body, action) {
        return Ok(e);
    }
    if !s.continuous() {
        return Ok(reject(
            "unavailable",
            "Live input requires a current conversation session",
        ));
    }
    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    if let Some(old) = prior(&tx, body)? {
        return Ok(old);
    }
    let p = &body["payload"];
    let subject = p["subject_id"].as_str().unwrap_or("");
    let out = match action {
        "conversation.steer" => {
            let turn = p["turn_id"].as_str().unwrap_or("");
            let text = p["text"].as_str().unwrap_or("");
            if text.trim().is_empty() || text.len() > 32768 {
                return Ok(reject(
                    "invalid_request",
                    "Guidance must be between 1 and 32768 bytes",
                ));
            }
            let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM conversation_turns t JOIN dialogue_inputs d USING(turn_id) WHERE t.turn_id=?1 AND t.project_key=?2 AND t.status='reserved' AND d.subject_id=?3 AND d.interrupt_command_id IS NULL)",params![turn,s.key,subject],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !active {
                return Ok(reject(
                    "stale_turn",
                    "The selected coordinator has finished or is stopping; guidance remains unsent",
                ));
            }
            let count:i64=tx.query_row("SELECT count(*) FROM conversation_live_inputs WHERE turn_id=?1 AND status IN ('pending','delivering')",[turn],|r|r.get(0)).map_err(|e|e.to_string())?;
            if count >= 64 {
                return Ok(reject(
                    "queue_full",
                    "This call already has 64 pending guidance inputs",
                ));
            }
            let seq: i64 = tx
                .query_row(
                    "SELECT coalesce(max(seq),0)+1 FROM conversation_live_inputs",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let id = format!(
                "i-{seq:020}-{}",
                &sha256(body["command_id"].as_str().unwrap().as_bytes())[..16]
            );
            tx.execute(
                "INSERT INTO conversation_live_inputs VALUES(?1,?2,?3,?4,'pending',NULL,NULL,?5)",
                params![id, turn, subject, text, seq],
            )
            .map_err(|e| e.to_string())?;
            event(
                &tx,
                "conversation.input_pending",
                None,
                &json!({"input_id":id,"turn_id":turn,"subject_id":subject,"delivery":"steer","authority_changed":false}),
            )?;
            json!({"status":"accepted","input_id":id,"turn_id":turn,"delivery":"pending"})
        }
        "conversation.withdraw_input" => {
            let id = p["input_id"].as_str().unwrap_or("");
            let row:Option<(String,String,Option<String>)>=tx.query_row("SELECT status,text,directory FROM conversation_live_inputs WHERE input_id=?1 AND subject_id=?2",params![id,subject],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
            let Some((status, text, directory)) = row else {
                return Ok(reject("not_found", "Guidance is not in this topic"));
            };
            if !matches!(status.as_str(), "pending" | "held") {
                return Ok(reject(
                    "already_dispatched",
                    "Guidance is entering the agent context and can no longer be withdrawn",
                ));
            }
            if let Some(d) = directory.as_deref() {
                if !withdraw_pending(d, id)? {
                    return Ok(reject(
                        "already_dispatched",
                        "Guidance is entering the agent context and can no longer be withdrawn",
                    ));
                }
            }
            tx.execute(
                "UPDATE conversation_live_inputs SET status='withdrawn' WHERE input_id=?1",
                [id],
            )
            .map_err(|e| e.to_string())?;
            event(
                &tx,
                "conversation.input_withdrawn",
                None,
                &json!({"input_id":id,"subject_id":subject}),
            )?;
            json!({"status":"accepted","input_id":id,"text":text})
        }
        _ => return Ok(reject("invalid_request", "Unknown live input action")),
    };
    commit(&tx, body, &out)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(out)
}
pub fn reconcile(s: &Session) -> Result<(), String> {
    if let Some(w) = &s.workflow.writer {
        return w.call(|s| reconcile(&s));
    }
    let mut c = s.workflow.connection()?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let inputs = {
        let mut q=tx.prepare("SELECT i.input_id,i.turn_id,i.subject_id,i.text,i.status,i.directory,i.scope_json,t.status,d.interrupt_command_id FROM conversation_live_inputs i JOIN conversation_turns t USING(turn_id) JOIN dialogue_inputs d USING(turn_id) WHERE i.status IN ('pending','delivering') ORDER BY i.seq LIMIT 64").map_err(|e|e.to_string())?;
        q.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    for (id, turn, subject, text, old, recorded_directory, recorded_scope, status, cancel) in inputs
    {
        let (directory, scope) = if let (Some(d), Some(scope)) =
            (recorded_directory, recorded_scope)
        {
            (
                d,
                serde_json::from_str::<Value>(&scope).map_err(|e| e.to_string())?,
            )
        } else {
            let operation = if s.development() {
                s.development_chat_attempt(&turn)?
            } else {
                Some(
                    s.workflow
                        .state
                        .join("session/coordinator")
                        .join(&turn)
                        .join("call-v1"),
                )
            };
            let Some(operation) =
                operation.filter(|p| p.join("active-host-attempt.json").is_file())
            else {
                if status != "reserved" || cancel.is_some() {
                    tx.execute(
                        "UPDATE conversation_live_inputs SET status='held' WHERE input_id=?1",
                        [&id],
                    )
                    .map_err(|e| e.to_string())?;
                    event(
                        &tx,
                        "conversation.input_held",
                        None,
                        &json!({"input_id":id,"subject_id":subject}),
                    )?;
                }
                continue;
            };
            let attempt = Attempt::load(&s.workflow.state, &operation)?;
            let intent = attempt.intent()?;
            let scope = json!({"project_key":s.key,"turn_id":turn,"attempt_id":attempt.id,"packet_sha256":intent["packet_sha256"]});
            let directory = attempt.directory.to_string_lossy().into_owned();
            tx.execute(
                "UPDATE conversation_live_inputs SET directory=?1,scope_json=?2 WHERE input_id=?3",
                params![directory, encode(&scope)?, id],
            )
            .map_err(|e| e.to_string())?;
            (directory, scope)
        };
        let root = Path::new(&directory).join("live-input");
        let name = format!("{id}.json");
        let claimed = root.join("claimed").join(&name).exists();
        let applied = root.join("applied").join(&name);
        let new = if applied.is_file() {
            let proof = read(&applied)?;
            if proof["input_id"] != id || proof["scope"] != scope || proof["text"] != text {
                return Err(
                    "Live input acknowledgement differs from the published guidance".into(),
                );
            }
            "applied"
        } else if status != "reserved"
            || Path::new(&directory).join("receipt.json").exists()
            || Path::new(&directory).join("retired.json").exists()
        {
            let _ = withdraw_pending(&directory, &id)?;
            "held"
        } else if claimed {
            "delivering"
        } else if cancel.is_some() {
            if withdraw_pending(&directory, &id)? {
                "held"
            } else {
                "delivering"
            }
        } else {
            let pending = root.join("pending").join(&name);
            if !pending.exists() {
                write(
                    &pending,
                    &json!({"schema_version":1,"input_id":id,"scope":scope,"text":text,"kind":"steer"}),
                    true,
                )?;
            }
            "pending"
        };
        if new != old {
            if new == "applied" {
                let seq: i64 = tx
                    .query_row(
                        "SELECT coalesce(max(seq),0)+1 FROM conversation_messages",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                tx.execute("INSERT OR IGNORE INTO conversation_messages VALUES(?1,1,?2,?3,'user',?4,?5,NULL,?6)",params![id,s.key,seq,text,sha256(text.as_bytes()),turn]).map_err(|e|e.to_string())?;
            }
            tx.execute(
                "UPDATE conversation_live_inputs SET status=?1 WHERE input_id=?2",
                params![new, id],
            )
            .map_err(|e| e.to_string())?;
            event(
                &tx,
                &format!("conversation.input_{new}"),
                None,
                &json!({"input_id":id,"turn_id":turn,"subject_id":subject,"delivery":new,"authority_changed":false}),
            )?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
