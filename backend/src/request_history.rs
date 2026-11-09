//! Readable projection of committed public events. SQLite and immutable receipts remain authority.
use crate::{
    codec::{canonical_sha256, sha256},
    conversation_store::{encode, event},
    execution::{FileLock, now, write},
    session_store::Session,
};
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, Write},
    path::{Path, PathBuf},
};

pub fn timestamp(time: f64) -> String {
    let seconds = time as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::gmtime_r(&seconds, &mut tm) };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}
/// Presentation metadata from the committed timestamp sidecar. Missing historical
/// times stay absent; viewing a row must never fabricate its event time.
pub fn add_event_times(conn: &rusqlite::Connection, rows: &mut [Value]) -> Result<(), String> {
    let exists: bool = conn.query_row("SELECT exists(SELECT 1 FROM sqlite_master WHERE type='table' AND name='request_history_events')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if !exists { return Ok(()); }
    let mut query = conn.prepare("SELECT at FROM request_history_events WHERE seq=?1").map_err(|e| e.to_string())?;
    for row in rows {
        let Some(seq) = row["seq"].as_i64() else { continue; };
        let at: Option<f64> = query.query_row([seq], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        if let Some(at) = at { row["at"] = json!(at); }
    }
    Ok(())
}
pub fn schema(s: &Session) -> Result<(), String> {
    s.workflow.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS request_history_requests(request_id TEXT PRIMARY KEY,subject_id TEXT NOT NULL UNIQUE,task_id TEXT,record_json TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS request_history_events(seq INTEGER PRIMARY KEY,request_id TEXT NOT NULL,at REAL NOT NULL,actor TEXT NOT NULL);") .map_err(|e|e.to_string())?;
    // Versioned sidecar metadata leaves old journal rows untouched. New rows always have a timestamp.
    s.workflow.connection()?.execute_batch("CREATE TRIGGER IF NOT EXISTS request_history_timestamp AFTER INSERT ON session_events BEGIN
      INSERT INTO request_history_events(seq,request_id,at,actor) VALUES(NEW.seq,'',unixepoch('subsec'),CASE WHEN NEW.type='conversation.message' THEN coalesce(json_extract(NEW.body,'$.role'),'service') ELSE 'service' END); END;") .map_err(|e|e.to_string())
}
fn reserve_history_path(
    tx: &Transaction<'_>,
    root: &Path,
    at: f64,
    request: &str,
    prompt_hash: &str,
) -> Result<PathBuf, String> {
    let stamp = timestamp(at).replace([':', '-'], "");
    // Include request identity so repeated prompts remain separate requests.
    let digest = sha256(format!("{prompt_hash}:{request}").as_bytes());
    for width in [12, 16, 24, 64] {
        let path = root.join(format!("{stamp}-{}.log", &digest[..width]));
        let reserved:bool=tx.query_row("SELECT exists(SELECT 1 FROM request_history_requests WHERE json_extract(record_json,'$.log_path')=?1)",[path.to_str().ok_or("Non-UTF8 history path")?],|r|r.get(0)).map_err(|e|e.to_string())?;
        let occupied = [
            path.clone(),
            path.with_extension("summary.md"),
            path.with_extension("projection.json"),
            path.with_extension("append.json"),
        ]
        .iter()
        .any(|p| fs::symlink_metadata(p).is_ok());
        if !reserved && !occupied {
            return Ok(path);
        }
    }
    Err("History filename collision; existing records and files preserved".into())
}
pub fn begin(
    tx: &Transaction<'_>,
    s: &Session,
    subject: &str,
    text: &str,
    starting_commit: Option<String>,
) -> Result<String, String> {
    if let Some(id) = tx
        .query_row(
            "SELECT request_id FROM request_history_requests WHERE subject_id=?1",
            [subject],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
    {
        return Ok(id);
    }
    let id = format!(
        "req-{}",
        &sha256(format!("{}:{subject}", s.key).as_bytes())[..24]
    );
    let at = now();
    let readable = redact_text(text);
    let name = readable
        .lines()
        .next()
        .unwrap_or("Request")
        .chars()
        .take(80)
        .collect::<String>();
    let prompt_hash = sha256(text.as_bytes());
    let path = reserve_history_path(
        tx,
        &crate::project_layout::history_root(&s.project, &s.workflow.state),
        at,
        &id,
        &prompt_hash,
    )?;
    let start = starting_commit;
    let record = json!({"schema_version":1,"request_id":id,"initial_name":name,"initial_goal":text,"initial_prompt_sha256":prompt_hash,"history_filename_version":2,"project":s.project,"starting_commit":start,"starting_repository":if start.is_none(){"new repository, no starting commit"}else{"existing repository"},"started":timestamp(at),"log_path":path,"summary_path":path.with_extension("summary.md"),"narrative_pending":true});
    tx.execute(
        "INSERT INTO request_history_requests VALUES(?1,?2,NULL,?3)",
        params![id, subject, encode(&record)?],
    )
    .map_err(|e| e.to_string())?;
    let (seq, _) = event(tx, "request.started", None, &record)?;
    tx.execute(
        "UPDATE request_history_events SET request_id=?1 WHERE seq=?2",
        params![id, seq],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}
pub fn link(tx: &Transaction<'_>, subject: &str, task: &str) -> Result<(), String> {
    tx.execute(
        "UPDATE request_history_requests SET task_id=?1 WHERE subject_id=?2",
        params![task, subject],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn tag(tx: &Transaction<'_>, seq: i64, request: &str, actor: &str) -> Result<(), String> {
    tx.execute(
        "UPDATE request_history_events SET request_id=?1,actor=?2 WHERE seq=?3",
        params![request, actor, seq],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
fn redact_text(text: &str) -> String {
    let mut hide_next = false;
    let mut output = String::new();
    for word in text.split_inclusive(char::is_whitespace) {
        let trimmed = word.trim_end_matches(char::is_whitespace);
        let suffix = &word[trimmed.len()..];
        let credential = trimmed.len() > 12
            && ["sk-", "ghp_", "github_pat_", "xoxb-", "xapp-"]
                .iter()
                .any(|p| trimmed.trim_start_matches(['\"', '\'', '(']).starts_with(p));
        if hide_next || credential {
            output.push_str("[REDACTED CREDENTIAL]");
            output.push_str(suffix);
            hide_next = false;
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let key = name.to_ascii_uppercase();
            if key.ends_with("_API_KEY")
                || key.ends_with("_TOKEN")
                || key.ends_with("_SECRET")
                || key == "PASSWORD"
            {
                output.push_str(name);
                output.push_str("=[REDACTED CREDENTIAL]");
                output.push_str(suffix);
                continue;
            }
        }
        output.push_str(word);
        hide_next = trimmed.eq_ignore_ascii_case("Bearer");
    }
    output
}
// Public records contain no native tool transcripts, environment or hidden reasoning. Apply a
// final defensive key filter too; raw evidence stays linked, not copied into readable history.
pub(crate) fn public(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.to_ascii_lowercase().as_str(),
                        "api_key"
                            | "token"
                            | "access_token"
                            | "refresh_token"
                            | "password"
                            | "secret"
                            | "authorization"
                            | "environment"
                            | "reasoning"
                            | "hidden_reasoning"
                            | "chain_of_thought"
                            | "auth_home"
                            | "auth_config_dir"
                            | "review_diff"
                            | "excerpt"
                            | "last_coordinator_result"
                            | "native_review"
                    )
                })
                .map(|(k, v)| (k.clone(), public(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(public).collect()),
        Value::String(s) => Value::String(redact_text(s)),
        _ => v.clone(),
    }
}
fn frame(
    seq: i64,
    at: f64,
    actor: &str,
    request: &str,
    kind: &str,
    task: Option<&str>,
    body: &Value,
) -> String {
    let category = if kind.starts_with("conversation") || kind.starts_with("scope") {
        "CONVERSATION"
    } else if kind.contains("assign") || kind.contains("notification") || kind.contains("steer") {
        "TASKS"
    } else if kind.contains("summary") {
        "SUMMARY"
    } else if kind == "request.started" {
        "PROJECT TASK NAME"
    } else {
        "OUTCOMES"
    };
    let body = serde_json::to_string_pretty(&public(body)).unwrap_or_default();
    let payload = format!(
        "[{}] {category} · event #{seq} · {kind}\nActor: {actor} · request: {request} · task: {}\n{body}\n",
        timestamp(at),
        task.unwrap_or("discussion")
    );
    format!(
        "\n>>> WAYWR EVENT {seq} {} {}\n{payload}<<< WAYWR EVENT {seq}\n",
        payload.len(),
        sha256(payload.as_bytes())
    )
}
// A complete frame is the cursor. A crash during append truncates only the uncommitted tail;
// previously published complete entries are never rewritten. Validate hashes before appending.
fn recover(file: &mut fs::File, expected_tail: Option<&[u8]>) -> Result<i64, String> {
    let mut reader = BufReader::new(file.try_clone().map_err(|e| e.to_string())?);
    let mut position = 0u64;
    let mut last = 0;
    loop {
        let mut separator = String::new();
        let count = reader
            .read_line(&mut separator)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        if separator != "\n" {
            return Err("History contains unexpected bytes; preserve and inspect".into());
        }
        let mut header = String::new();
        let count = reader
            .by_ref()
            .take(1024)
            .read_line(&mut header)
            .map_err(|e| e.to_string())?;
        if count == 0 || !header.ends_with('\n') {
            if !">>> WAYWR EVENT ".starts_with(&header) && !header.starts_with(">>> WAYWR EVENT ") {
                return Err("History tail changed; preserve and inspect".into());
            }
            break;
        }
        let fields = header.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 6 || fields[..3] != [">>>", "WAYWR", "EVENT"] {
            return Err("History frame header changed; preserve and inspect".into());
        }
        let seq = fields[3].parse::<i64>().map_err(|e| e.to_string())?;
        let length = fields[4].parse::<usize>().map_err(|e| e.to_string())?;
        if length > 1024 * 1024 {
            return Err("History frame exceeds supported bound; preserve and inspect".into());
        }
        let mut payload = vec![0; length];
        match reader.read_exact(&mut payload) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.to_string()),
        }
        let mut footer = String::new();
        reader
            .by_ref()
            .take(128)
            .read_line(&mut footer)
            .map_err(|e| e.to_string())?;
        let expected = format!("<<< WAYWR EVENT {seq}\n");
        if footer != expected {
            if expected.starts_with(&footer) {
                break;
            }
            return Err("History footer changed; preserve and inspect".into());
        }
        if sha256(&payload) != fields[5] || seq <= last {
            return Err("History frame integrity changed; preserve and inspect".into());
        }
        last = seq;
        position = reader.stream_position().map_err(|e| e.to_string())?;
    }
    if position < file.metadata().map_err(|e| e.to_string())?.len() {
        let mut tail_file = file.try_clone().map_err(|e| e.to_string())?;
        tail_file
            .seek(std::io::SeekFrom::Start(position))
            .map_err(|e| e.to_string())?;
        let mut tail = Vec::new();
        tail_file
            .take(1024 * 1024 + 1024)
            .read_to_end(&mut tail)
            .map_err(|e| e.to_string())?;
        if !expected_tail.is_some_and(|expected| expected.starts_with(&tail)) {
            return Err(
                "Unexpected/manual history tail preserved; no matching durable append intent"
                    .into(),
            );
        }
        file.set_len(position).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    Ok(last)
}
pub fn requests(s: &Session) -> Result<Vec<Value>, String> {
    let c = s.workflow.connection()?;
    let mut st=c.prepare("SELECT task_id,record_json FROM request_history_requests ORDER BY rowid DESC LIMIT 100").map_err(|e|e.to_string())?;
    let rows = st
        .query_map([], |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|(task, raw)| {
            let mut v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let path = Path::new(v["log_path"].as_str().ok_or("Missing history path")?);
            v["projection"] = crate::execution::read(&path.with_extension("projection.json"))
                .unwrap_or(json!({"through_event":0,"attention":"projection pending"}));
            let work = task
                .as_deref()
                .map(|t| s.development_record("workflow", t))
                .transpose()?
                .flatten();
            v["narrative_pending"] = json!(work.as_ref().is_none_or(|w| w["summary"].is_null()));
            v["attention"] =
                crate::execution::read(&s.workflow.state.join("history/projection-attention.json"))
                    .unwrap_or(Value::Null);
            Ok(v)
        })
        .collect()
}
pub(crate) fn compact_workflow(w: &Value) -> Value {
    let all = w["obligations"].as_array().cloned().unwrap_or_default();
    let unresolved: Vec<_> = all.iter().filter(|o| o["status"] != "resolved").collect();
    let checks = w["checks"]["result"]["checks"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(
                    |c| json!({"argv":c["argv"],"exit_code":c["exit_code"],"success":c["success"]}),
                )
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let bounded = |v: &Value, limit| {
        v.as_str()
            .map(|s| s.chars().take(limit).collect::<String>())
    };
    json!({"work_id":w["work_id"],"status":w["status"],"requirement_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"commit":w["head"],
        "checks":{"commit":w["checks"]["commit"],"passed":w["checks"]["result"]["passed"],"evidence":w["checks"]["evidence"],"receipt_status":w["checks"]["receipt"]["status"],"result_sha256":w["checks"]["receipt"]["result_sha256"],"commands":checks},
        "obligations":{"total":all.len(),"resolved":all.len()-unresolved.len(),"unresolved_count":unresolved.len(),"unresolved":unresolved.iter().take(16).map(|o|json!({"name":bounded(&o["name"],512),"owner":o["owner"],"status":o["status"]})).collect::<Vec<_>>(),"additional_unresolved":unresolved.len().saturating_sub(16),"details":"Full names, resolution rationale and provenance remain in the append-only request log and Inspect"},
        "summary":if w["summary"].is_null(){Value::Null}else{json!({"label":w["summary"]["label"],"text":bounded(&w["summary"]["text"],4096),"rationale":bounded(&w["summary"]["rationale"],2048),"commit":w["summary"]["commit"],"source_operation":w["summary"]["source_operation"],"details":"Full coordinator narrative remains in the request log"})},
        "recovery":if w["recovery"].is_object(){crate::development_recovery::summary(&w["recovery"])}else{Value::Null},
        "progress":{"current_sha256":w["progress"]["current"]["sha256"],"wait_state":w["progress"]["wait_state"]},
        "candidate":w["candidate"],"checkout_handoff":w["checkout_handoff"],"delivery":w["delivery"],"attention":w["attention"],"next_action":match w["status"].as_str(){Some("delivered")=>"Delivered; inspect the result or start a new request",Some("cancelled")=>"Cancelled; retained work remains available for inspection",Some("attention")=>"Inspect the saved concrete blocker before recovery",_=>"Follow current task/check/dependency status in the TUI"}})
}
pub fn project(s: &Session) -> Result<bool, String> {
    if !s.development() {
        return Ok(false);
    }
    let root = crate::project_layout::history_root(&s.project, &s.workflow.state);
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let _lock = FileLock::exclusive(&root.join("projection.lock"))?;
    let mut catchup_pending = false;
    for request in requests(s)? {
        let id = request["request_id"].as_str().ok_or("Missing request ID")?;
        let path = Path::new(request["log_path"].as_str().ok_or("Missing history path")?);
        if !path.starts_with(&root)
            || fs::symlink_metadata(path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err("History path changed; preserve and inspect".into());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let intent_path = path.with_extension("append.json");
        let intent = crate::execution::read(&intent_path).ok();
        let pending = intent
            .as_ref()
            .and_then(|v| v["frame"].as_str())
            .filter(|frame| {
                intent
                    .as_ref()
                    .is_some_and(|i| i["frame_sha256"] == sha256(frame.as_bytes()))
            });
        let mut cursor = recover(&mut file, pending.map(str::as_bytes))?;
        let c = s.workflow.connection()?;
        let task: Option<String> = c
            .query_row(
                "SELECT task_id FROM request_history_requests WHERE request_id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut st=c.prepare("SELECT e.seq,h.at,h.actor,e.type,e.task_id,e.body FROM session_events e JOIN request_history_events h USING(seq) WHERE e.seq>?1 AND (h.request_id=?2 OR (h.request_id='' AND (e.task_id=?3 OR json_extract(e.body,'$.turn_id') IN (SELECT i.turn_id FROM dialogue_inputs i JOIN request_history_requests r USING(subject_id) WHERE r.request_id=?2) OR (e.type='session.state_changed' AND e.seq>=(SELECT min(seq) FROM request_history_events WHERE request_id=?2) AND e.seq<=coalesce((SELECT min(seq) FROM session_events WHERE type IN ('development.delivered','development.cancelled') AND task_id=?3),9223372036854775807))))) AND e.type!='conversation.delta' ORDER BY e.seq LIMIT 128").map_err(|e|e.to_string())?;
        let page = st
            .query_map(params![cursor, id, task], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(st);
        catchup_pending |= page.len() == 128;
        for row in page {
            let (seq, at, actor, kind, task, body) = row;
            let body: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
            let entry = frame(seq, at, &actor, id, &kind, task.as_deref(), &body);
            write(
                &intent_path,
                &json!({"schema_version":1,"source_event":seq,"frame_sha256":sha256(entry.as_bytes()),"frame":entry}),
                false,
            )?;
            file.write_all(entry.as_bytes())
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            cursor = seq;
        }
        let work = task
            .as_deref()
            .map(|t| s.development_record("workflow", t))
            .transpose()?
            .flatten();
        let summary = json!({"schema_version":1,"request_id":id,"through_event":cursor,"log_path":path,"workflow":work.as_ref().map(compact_workflow),"narrative_pending":work.as_ref().is_none_or(|w|w["summary"].is_null()),"freshness":"current through committed event watermark","provenance":"deterministic application facts; coordinator summary is labelled narrative"});
        let summary = public(&summary);
        let text = format!(
            "# {}\n\nApplication checkpoint through event #{cursor}.\n\nHistory: {}\n\n{}\n",
            request["initial_name"].as_str().unwrap_or("Request"),
            path.display(),
            serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
        );
        crate::execution::write_bytes(&path.with_extension("summary.md"), text.as_bytes(), false)?;
        write(
            &path.with_extension("projection.json"),
            &json!({"through_event":cursor,"summary_sha256":canonical_sha256(&summary).map_err(|e|e.to_string())?,"updated":timestamp(now()),"updated_unix":now(),"attention":null}),
            false,
        )?;
    }
    Ok(catchup_pending)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_history_names_separate_repeated_prompts_and_preserve_colliding_files_and_records() {
        let d = tempfile::tempdir().unwrap();
        let mut c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE request_history_requests(record_json TEXT)")
            .unwrap();
        let tx = c.transaction().unwrap();
        let text = "A long Unicode request 日本語 🦀 ".repeat(1000);
        let prompt_hash = sha256(text.as_bytes());
        let first = reserve_history_path(&tx, d.path(), 0.0, "req-first", &prompt_hash).unwrap();
        let other = reserve_history_path(&tx, d.path(), 0.0, "req-second", &prompt_hash).unwrap();
        assert_ne!(first, other);
        let name = first.file_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), 33);
        assert!(name.starts_with("19700101T000000Z-"));
        assert!(name.ends_with(".log"));
        assert!(!name.contains("Unicode"));
        fs::write(&first, "Existing history must stay intact").unwrap();
        let extended = reserve_history_path(&tx, d.path(), 0.0, "req-first", &prompt_hash).unwrap();
        assert_ne!(first, extended);
        assert_eq!(extended.file_name().unwrap().len(), 37);
        assert_eq!(
            fs::read_to_string(&first).unwrap(),
            "Existing history must stay intact"
        );
        tx.execute(
            "INSERT INTO request_history_requests VALUES(?1)",
            [json!({"log_path":extended}).to_string()],
        )
        .unwrap();
        let longer = reserve_history_path(&tx, d.path(), 0.0, "req-first", &prompt_hash).unwrap();
        assert_ne!(longer, extended);
        assert_eq!(longer.file_name().unwrap().len(), 45);
    }
    #[test]
    fn saved_history_paths_remain_stable_and_full_requests_remain_in_the_log_entry() {
        let d = tempfile::tempdir().unwrap();
        let s = Session::open(
            d.path(),
            &d.path().join("state"),
            &"a".repeat(32),
            "fixture",
        )
        .unwrap();
        schema(&s).unwrap();
        let mut c = s.workflow.connection().unwrap();
        let tx = c.transaction().unwrap();
        let text = "Keep this complete Unicode request 日本語 🦀. ".repeat(200);
        let id = begin(&tx, &s, "topic", &text, None).unwrap();
        let raw: String = tx
            .query_row(
                "SELECT record_json FROM request_history_requests WHERE request_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        let mut record: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(record["initial_goal"], text);
        assert_eq!(record["initial_prompt_sha256"], sha256(text.as_bytes()));
        assert!(frame(1, 0.0, "user", &id, "request.started", None, &record).contains(&text));
        record["log_path"] = json!("/preserved/legacy-readable-name.log");
        tx.execute(
            "UPDATE request_history_requests SET record_json=?1 WHERE request_id=?2",
            params![record.to_string(), id],
        )
        .unwrap();
        assert_eq!(begin(&tx, &s, "topic", "Later message", None).unwrap(), id);
        let saved: String = tx
            .query_row(
                "SELECT record_json FROM request_history_requests WHERE request_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&saved).unwrap()["log_path"],
            record["log_path"]
        );
    }
    #[test]
    fn compact_checkpoint_retains_status_and_evidence_without_repeating_resolved_obligations() {
        let obligations: Vec<_>=(0..32).map(|i|json!({"name":format!("Obligation {i}"),"status":"resolved","resolution":{"rationale":"repeated evidence ".repeat(1000)}})).collect();
        let w = json!({"work_id":"work","status":"delivered","head":"exact-sha","checks":{"commit":"exact-sha","evidence":"/checks/final","receipt":{"status":"completed","result_sha256":"result-hash"},"result":{"passed":true,"checks":[{"argv":["cargo","test","--locked"],"exit_code":0,"success":true,"tail":"large output ".repeat(1000)}]}},"obligations":obligations,"summary":{"label":"coordinator narrative / opinion","text":"Delivered 日本語","rationale":"Observed checks passed","commit":"exact-sha","source_operation":"coordinator-final"}});
        let compact = compact_workflow(&w);
        assert_eq!(compact["status"], "delivered");
        assert_eq!(compact["checks"]["passed"], true);
        assert_eq!(compact["checks"]["evidence"], "/checks/final");
        assert_eq!(
            compact["checks"]["commands"][0]["argv"],
            json!(["cargo", "test", "--locked"])
        );
        assert_eq!(compact["obligations"]["resolved"], 32);
        assert_eq!(compact["obligations"]["unresolved_count"], 0);
        assert!(compact.to_string().len() < 2500);
        assert_eq!(compact["summary"]["text"], "Delivered 日本語");
        let mut open = w;
        open["status"] = json!("attention");
        for o in open["obligations"].as_array_mut().unwrap() {
            o["status"] = json!("open");
        }
        let compact = compact_workflow(&open);
        assert_eq!(compact["obligations"]["unresolved_count"], 32);
        assert_eq!(
            compact["obligations"]["unresolved"]
                .as_array()
                .unwrap()
                .len(),
            16
        );
        assert_eq!(compact["obligations"]["additional_unresolved"], 16);
    }
    #[test]
    fn interrupted_append_recovers_without_duplicate_or_missing_entries() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("h.log");
        let a = frame(
            2,
            0.0,
            "user",
            "r",
            "conversation.message",
            None,
            &json!({"text":"Unicode 日本語"}),
        );
        let b = frame(
            7,
            1.0,
            "worker",
            "r",
            "unit.submitted",
            Some("t"),
            &json!({"commit":"abc"}),
        );
        fs::write(&p, format!("{a}{}", &b[..b.len() / 2])).unwrap();
        let mut f = OpenOptions::new().read(true).append(true).open(&p).unwrap();
        assert_eq!(recover(&mut f, Some(b.as_bytes())).unwrap(), 2);
        f.write_all(b.as_bytes()).unwrap();
        drop(f);
        let mut f = OpenOptions::new().read(true).append(true).open(&p).unwrap();
        assert_eq!(recover(&mut f, None).unwrap(), 7);
        assert_eq!(fs::read_to_string(&p).unwrap(), format!("{a}{b}"));
    }
    #[test]
    fn public_history_does_not_copy_secret_or_reasoning_fields() {
        assert_eq!(
            public(
                &json!({"secret":"sentinel","hidden_reasoning":"private","nested":{"access_token":"private","summary":"fact"}})
            ),
            json!({"nested":{"summary":"fact"}})
        );
    }
    #[test]
    fn readable_projection_redacts_recognized_credentials_in_public_text() {
        let text = "Keep Unicode 日本語. OPENAI_API_KEY=private-value Bearer private-token sk-abcdefghijklmnopqrstuvwxyz remain facts";
        let projected = public(&json!({"text":text})).to_string();
        assert!(!projected.contains("private-value"));
        assert!(!projected.contains("private-token"));
        assert!(!projected.contains("sk-abc"));
        assert!(projected.contains("日本語"));
        assert!(projected.contains("remain facts"));
    }
    #[test]
    fn unexpected_manual_tail_is_preserved_even_when_it_looks_like_a_frame() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("history.log");
        let complete = frame(
            1,
            0.0,
            "user",
            "r",
            "conversation.message",
            None,
            &json!({"text":"fact"}),
        );
        let bytes = format!("{complete}\n>>> WAYWR EVENT 99 manually appended");
        fs::write(&path, &bytes).unwrap();
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(&path)
            .unwrap();
        assert!(recover(&mut file, None).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), bytes);
    }
}
