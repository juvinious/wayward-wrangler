use crossterm::{event::{self, EnableBracketedPaste, DisableBracketedPaste, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode}};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::{Value, json};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::{env, fs, io::{self, stdout, Write}, os::{unix::{fs::{OpenOptionsExt, PermissionsExt}, net::UnixStream}}, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc::{self}}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use waywr::{protocol::{self, Frame}, prototype::{editor::{Composer, Draft}, view::{App, LiveIntent, Modal}}};
mod rust_runtime;
mod rust_bundle;
mod rust_cli;
mod rust_views;
mod rust_review;
mod rust_maintenance;
mod rust_migrate;
mod cli_schema;

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)) {
            let _=execute!(stdout(),PopKeyboardEnhancementFlags,DisableBracketedPaste,LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) { let _ = execute!(stdout(), PopKeyboardEnhancementFlags, DisableBracketedPaste, LeaveAlternateScreen); let _ = disable_raw_mode(); }
}
fn page_bytes(hex:&str)->io::Result<Vec<u8>>{
    if hex.len()%2!=0{return Err(io::Error::other("Malformed project output page"));}
    hex.as_bytes().chunks_exact(2).map(|pair|{
        let text=std::str::from_utf8(pair).map_err(io::Error::other)?;
        u8::from_str_radix(text,16).map_err(io::Error::other)
    }).collect()
}
fn terminal_text(bytes:&[u8])->String{
    String::from_utf8_lossy(bytes).chars().map(|character|{
        if character=='\n' || character=='\t' || !character.is_control(){character.to_string()}
        else{format!("\\u{{{:x}}}",character as u32)}
    }).collect()
}

fn peer_is_current_uid(stream: &UnixStream) -> io::Result<bool> {
    use std::os::fd::AsRawFd;
    let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe { libc::getsockopt(stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, &mut cred as *mut _ as *mut _, &mut size) };
    if result != 0 { return Err(io::Error::last_os_error()); }
    Ok(cred.uid == unsafe { libc::geteuid() })
}

fn session(socket: &str, key: &str, release: &str, channel: &str) -> io::Result<UnixStream> {
    let stream = protocol::connect(socket, key, release, channel)?;
    if !peer_is_current_uid(&stream)? { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "service UID mismatch")); }
    Ok(stream)
}

fn reconnect(socket: &str, key: &str, release: &str, channel: &str) -> io::Result<UnixStream> {
    let mut last = None;
    for delay in [1, 2, 4] {
        thread::sleep(Duration::from_secs(delay));
        match session(socket,key,release,channel) {
            Ok(stream) => return Ok(stream),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("reconnect exhausted")))
}

fn reconnect_snapshot(socket:&str,key:&str,release:&str,cursor:u64)->io::Result<(UnixStream,Frame)>{
    let deadline=Instant::now()+Duration::from_secs(6);
    let mut last=io::Error::other("Reconnect exhausted before a verified snapshot");
    for attempt in 0..6{
        if Instant::now()>=deadline{break;}
        if attempt>0{thread::sleep(Duration::from_millis(100*(attempt as u64)));}
        if Instant::now()>=deadline{break;}
        let mut stream=match session(socket,key,release,"events"){Ok(s)=>s,Err(e)=>{last=e;continue;}};
        let remaining=deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero(){break;}
        stream.set_read_timeout(Some(remaining.min(Duration::from_secs(2))))?;
        stream.set_write_timeout(Some(remaining.min(Duration::from_secs(2))))?;
        match request(&mut stream,"snapshot","reconnected-snapshot",json!({})){
            Ok(snapshot) if snapshot.kind=="snapshot"=>{
                if snapshot.body["cursor"].as_u64().is_none_or(|latest|latest<cursor){return Err(io::Error::new(io::ErrorKind::InvalidData,"Reconnected state regressed; mutation refused"));}
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                return Ok((stream,snapshot));
            },
            Ok(error)=>last=io::Error::other(format!("Snapshot unavailable during reconnect: {}",error.body)),
            Err(error)=>last=error,
        }
    }
    Err(last)
}


fn request(stream: &mut UnixStream, kind: &str, id: &str, body: Value) -> io::Result<Frame> {
    protocol::write_frame(stream, &Frame::new(kind, id, body))?;
    let reply = protocol::read_frame(stream)?;
    if reply.request_id != id { return Err(io::Error::new(io::ErrorKind::InvalidData, "mismatched reply ID")); }
    Ok(reply)
}

fn next_id(counter: &mut u64) -> String { *counter += 1; format!("{}-{}", std::process::id(), *counter) }

fn command_id(counter: &mut u64) -> String {
    *counter += 1;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{}-{}-{}", stamp, std::process::id(), counter)
}

fn persist_outbox(directory: &Path, id: &str, payload: &Value) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    if fs::read_dir(directory)?.count() >= 128 { return Err(io::Error::other("outbox full")); }
    let temporary = directory.join(format!(".{id}.tmp"));
    let final_path = directory.join(format!("{id}.json"));
    let bytes = serde_json::to_vec(payload)?;
    if bytes.len() > 32 * 1024 + 1024 { return Err(io::Error::other("composer too large")); }
    let mut file = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(temporary, final_path)?;
    fs::File::open(directory)?.sync_all()?;
    Ok(())
}

fn pending_outbox(directory: &Path) -> io::Result<Vec<(String, Value)>> {
    if !directory.exists() { return Ok(Vec::new()); }
    let mut result = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|value| value == "json") {
            let id = entry.file_name().to_string_lossy().trim_end_matches(".json").to_owned();
            let payload = serde_json::from_slice(&fs::read(entry.path())?)?;
            result.push((id, payload));
        }
    }
    result.sort_by(|a,b| a.0.cmp(&b.0));
    Ok(result)
}


fn save_editor_draft(path:&Path,draft:&Draft)->io::Result<()> {
    let parent=path.parent().ok_or_else(||io::Error::other("draft path lacks parent"))?;
    fs::create_dir_all(parent)?;
    let temp=path.with_extension("tmp");
    let mut file=fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temp)?;
    file.write_all(&serde_json::to_vec(&json!({"schema_version":2,"draft":draft}))?)?;
    file.sync_all()?;fs::rename(temp,path)?;fs::File::open(parent)?.sync_all()?;
    Ok(())
}

fn load_editor_draft(path:&Path)->io::Result<Draft> {
    if !path.exists(){return Ok(Draft::default());}
    let value:Value=serde_json::from_slice(&fs::read(path)?)?;
    match value["schema_version"].as_u64() {
        Some(1)=>Ok(Draft{text:value["text"].as_str().unwrap_or("").into(),..Draft::default()}),
        Some(2)=>serde_json::from_value(value["draft"].clone()).map_err(io::Error::other),
        _=>Err(io::Error::other("unsupported UI draft version")),
    }
}

fn enqueue_mutation(outbox:&Path,tx:&mpsc::SyncSender<(String,String,Value)>,id:String,payload:Value)->io::Result<()> {
    persist_outbox(outbox,&id,&payload)?;
    tx.try_send(("command".into(),id,payload)).map_err(|e|io::Error::other(format!("command saved for reconciliation: {e}")))
}

fn backend_info() -> io::Result<()> {
    let (root,manifest)=paired_root()?;
    rust_runtime::backend_info(&root,&manifest)
}

fn protocol_probe(socket: &str, key: &str, release: &str) -> io::Result<()> {
    let mut connection = session(socket, key, release, "command")?;
    let snapshot = request(&mut connection, "snapshot", "p-snapshot", json!({}))?;
    let revision = snapshot.body.get("state_revision").and_then(Value::as_u64).ok_or_else(|| io::Error::other("invalid snapshot"))?;
    let command = json!({"command_id":"probe-1", "action":"submit_message", "expected_state_revision":revision, "payload":{"text":"proof"}});
    let first = request(&mut connection, "command", "p-command", command.clone())?;
    let replay = request(&mut connection, "command", "p-replay", command.clone())?;
    let conflict = request(&mut connection, "command", "p-conflict", json!({"command_id":"probe-1", "action":"submit_message", "expected_state_revision":revision, "payload":{"text":"different"}}))?;
    let stale = request(&mut connection, "command", "p-stale", json!({"command_id":"probe-2", "action":"submit_message", "expected_state_revision":revision, "payload":{"text":"stale"}}))?;
    let events = request(&mut connection, "events_read", "p-events", json!({"after_seq":0,"limit":64}))?;
    println!("{}", json!({"first":first.body,"replay":replay.body,"conflict":conflict.body,"stale":stale.body,"events":events.body}));
    Ok(())
}

fn lost_reply_probe(socket: &str, key: &str, release: &str, outbox: &Path) -> io::Result<()> {
    let id = command_id(&mut 0);
    let payload = json!({"command_id":id,"action":"submit_message","expected_state_revision":0,"payload":{"text":"lost-reply-proof"}});
    persist_outbox(outbox, &id, &payload)?;
    let mut original = session(socket,key,release,"command")?;
    let first = request(&mut original,"command",&id,payload.clone());
    if first.is_ok() { return Err(io::Error::other("fixture did not drop the first reply")); }
    let mut replacement = session(socket,key,release,"command")?;
    let status = request(&mut replacement,"command_get","lookup",json!({"command_id":id}))?;
    let replayed = pending_outbox(outbox)?;
    if replayed != vec![(id.clone(),payload.clone())] { return Err(io::Error::other("outbox payload changed")); }
    let result = request(&mut replacement,"command",&id,payload)?;
    if result.body.get("status") != Some(&Value::String("accepted".into())) { return Err(io::Error::other("same-ID replay refused")); }
    fs::remove_file(outbox.join(format!("{id}.json")))?;
    println!("{}",json!({"lookup":status.body,"replay":result.body,"pending":pending_outbox(outbox)?.len()}));
    Ok(())
}

fn run_ui(socket: String, key: String, release: String, title: String, outbox_dir: PathBuf,
    ui_token: Option<String>, project:Option<PathBuf>) -> io::Result<()> {
    let draft_path=outbox_dir.parent().ok_or_else(||io::Error::other("outbox lacks session parent"))?.join("draft.json");
    let mut command_stream = session(&socket, &key, &release, "command")?;
    let mut event_stream = session(&socket, &key, &release, "events")?;
    if let Some(token) = ui_token {
        if env::var("HERDR_ENV").ok().as_deref() != Some("1") {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "UI pane is not Herdr-managed"));
        }
        let pane = env::var("HERDR_PANE_ID").map_err(|_| io::Error::other("missing Herdr pane identity"))?;
        let registered = request(&mut command_stream,"ui.register","ui-register",json!({"ui_token":token,"pane_id":pane}))?;
        if registered.kind != "ui.register_result" { return Err(io::Error::other(format!("UI registration refused: {}",registered.body))); }
    }
    let snapshot = request(&mut command_stream, "snapshot", "initial", json!({}))?;
    if snapshot.kind != "snapshot" { return Err(io::Error::other("snapshot refused")); }
    let (commands_tx, commands_rx) = mpsc::sync_channel::<(String, String, Value)>(16);
    let (updates_tx, updates_rx) = mpsc::sync_channel::<Frame>(16);
    let command_updates = updates_tx.clone();
    let revision = Arc::new(AtomicU64::new(snapshot.body.get("state_revision").and_then(Value::as_u64).unwrap_or(0)));
    let mutations_allowed = Arc::new(AtomicBool::new(true));
    let command_revision = revision.clone();
    let command_outbox = outbox_dir.clone();
    let command_socket = socket.clone();
    let command_key = key.clone();
    let command_release = release.clone();
    thread::spawn(move || {
        while let Ok((kind, id, payload)) = commands_rx.recv() {
            let mut reconnects = 0;
            let reply = loop {
                match request(&mut command_stream, &kind, &id, payload.clone()) {
                    Ok(frame) => break Ok(frame),
                    Err(error) => {
                        if reconnects >= 2 { break Err(error); }
                        reconnects += 1;
                        match reconnect(&command_socket,&command_key,&command_release,"command") {
                            Ok(replacement) => command_stream = replacement,
                            Err(error) => break Err(error),
                        }
                    }
                }
            };
            match reply {
                Ok(mut frame) => {
                    if kind=="command"&&matches!(payload["action"].as_str(),Some("submit_message"|"coordination.submit"))&&(frame.body["status"]=="rejected"||frame.kind=="error"){
                        let evidence=command_outbox.parent().unwrap().join("refused-deliveries").join(format!("{id}.json"));
                        let _=waywr_backend::execution::write(&evidence,&json!({"command":payload,"response":frame.body}),true);
                        frame.body["local_refused_submission"]=payload["payload"].clone();
                    }
                    if let Some(value) = frame.body.get("state_revision").and_then(Value::as_u64) {
                        command_revision.fetch_max(value, Ordering::SeqCst);
                    }
                    if kind == "command" && (frame.kind == "command_result" || frame.kind == "error") {
                        let _ = fs::remove_file(command_outbox.join(format!("{id}.json")));
                    }
                    if command_updates.send(frame).is_err() { break; }
                },
                Err(error) => {
                    let _ = command_updates.send(Frame::new("error", &id, json!({"code":"service_unavailable","message":error.to_string(),"operator_action":"Reconnect after checking the project unit; pending command remains in the outbox"})));
                    break;
                }
            }
        }
    });
    for (id, payload) in pending_outbox(&outbox_dir)? {
        if commands_tx.try_send(("command".into(), id, payload)).is_err() { break; }
    }
    let event_revision = revision.clone();
    let event_mutations = mutations_allowed.clone();
    let event_socket = socket.clone();
    let event_key = key.clone();
    let event_release = release.clone();
    let initial_cursor = snapshot.body.get("cursor").and_then(Value::as_u64).unwrap_or(0);
    thread::spawn(move || {
        let mut cursor = initial_cursor;
        let mut counter = 0;
        loop {
            let id = next_id(&mut counter);
            let reply = request(&mut event_stream, "events_read", &id, json!({"after_seq":cursor,"limit":64,"wait_ms":1000}));
            let frame = match reply {
                Ok(frame) => frame,
                Err(_) => {
                    event_mutations.store(false,Ordering::SeqCst);
                    let _=updates_tx.send(Frame::new("error",&id,json!({"code":"service_unavailable","message":"Reconnecting to this project's service; draft preserved"})));
                    let (replacement,snapshot)=match reconnect_snapshot(&event_socket,&event_key,&event_release,cursor){
                        Ok(value)=>value,
                        Err(error)=>{let _=updates_tx.send(Frame::new("error",&id,json!({"code":"resync_required","message":format!("Reconnect stopped: {error}. Reopen Waywr to reconcile; no call was resubmitted.")})));break;}
                    };
                    cursor=snapshot.body["cursor"].as_u64().unwrap();
                    if let Some(value)=snapshot.body.get("state_revision").and_then(Value::as_u64) { event_revision.fetch_max(value,Ordering::SeqCst); }
                    if updates_tx.send(snapshot).is_err() { break; }
                    event_stream=replacement;
                    event_mutations.store(true,Ordering::SeqCst);
                    continue;
                }
            };
            let mut expected = cursor + 1;
            let mut gap = false;
            if let Some(events) = frame.body.get("events").and_then(Value::as_array) {
                for item in events {
                    if item.get("seq").and_then(Value::as_u64) != Some(expected) { gap = true; break; }
                    expected += 1;
                }
            } else { gap = true; }
            let next = frame.body.get("next_seq").and_then(Value::as_u64).unwrap_or(cursor);
            if gap || next != expected - 1 {
                event_mutations.store(false, Ordering::SeqCst);
                let _ = updates_tx.send(Frame::new("error", &id, json!({"code":"resync_required","message":"Event sequence gap; reconnect and reload snapshot before mutation"})));
                break;
            }
            cursor = next;
            if let Some(value) = frame.body.get("state_revision").and_then(Value::as_u64) {
                event_revision.fetch_max(value, Ordering::SeqCst);
            }
            let changed = frame.body.get("events").and_then(Value::as_array).is_some_and(|items| !items.is_empty());
            if updates_tx.send(frame).is_err() { break; }
            if changed {
                let id = next_id(&mut counter);
                match request(&mut event_stream,"snapshot",&id,json!({})) {
                    Ok(snapshot) if snapshot.kind == "snapshot" => { if updates_tx.send(snapshot).is_err() { break; } },
                    _ => {
                        event_mutations.store(false,Ordering::SeqCst);
                        let _=updates_tx.send(Frame::new("error",&id,json!({"code":"service_unavailable","message":"Snapshot delivery interrupted; reconnecting with draft preserved"})));
                        match reconnect_snapshot(&event_socket,&event_key,&event_release,cursor){
                            Ok((replacement,snapshot))=>{
                                cursor=snapshot.body["cursor"].as_u64().unwrap();
                                if let Some(value)=snapshot.body["state_revision"].as_u64(){event_revision.fetch_max(value,Ordering::SeqCst);}
                                if updates_tx.send(snapshot).is_err(){break;}
                                event_stream=replacement;event_mutations.store(true,Ordering::SeqCst);
                            },
                            Err(error)=>{let _=updates_tx.send(Frame::new("error",&id,json!({"code":"resync_required","message":format!("Snapshot reconnect stopped: {error}. Reopen Waywr; no call was resubmitted.")})));break;}
                        }
                    }
                }
            }
        }
    });
    let mut _guard=TerminalGuard::enter()?;
    let mut terminal=Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app=App::new_live(title,load_editor_draft(&draft_path)?);
    let input_history_path = draft_path.with_file_name("command-history.json");
    app.input_history = waywr::prototype::input_history::InputHistory::load(&input_history_path)?;
    app.apply_snapshot(&snapshot.body);
    let scope_draft_path=draft_path.with_file_name("scope-edit.json");
    if app.scope_mode()&&scope_draft_path.exists(){let bytes=fs::read(&scope_draft_path)?;if bytes.len()>128*1024{return Err(io::Error::other("Saved scope edit exceeds bounds; inspect it before attaching"));}app.restore_scope_edit(serde_json::from_slice(&bytes)?);}
    let subject_path=draft_path.with_file_name("conversation-subject.json");
    if app.continuous_mode()&&subject_path.exists(){let bytes=fs::read(&subject_path)?;if bytes.len()>1024*1024{return Err(io::Error::other("Saved conversation subjects/drafts exceed 1 MiB"));}app.restore_conversation_ui(&serde_json::from_slice::<Value>(&bytes)?).map_err(io::Error::other)?;}
    let mut last_subject=app.conversation_ui_state();
    let mut last_scope_draft=app.scope_edit_snapshot();
    let mut counter=0u64;
    let mut last_draft=app.composer.snapshot();
    let mut last_draft_save=Instant::now();
    let mut last_draw=Instant::now()-Duration::from_secs(1);
    let mut exit_preparation:Option<String>=None;
    let mut exit_command_id:Option<String>=None;
    let mut exit_result:Option<Value>=None;
    let mut output_request_id:Option<String>=None;
    let mut last_output_fetch=Instant::now()-Duration::from_secs(1);
    let mut operation_request_id:Option<String>=None;
    let mut last_operation_fetch=Instant::now()-Duration::from_secs(2);
    let mut artifact_request_id:Option<String>=None;
    let mut detail_request_id:Option<String>=None;
    let mut last_detail_fetch=Instant::now()-Duration::from_secs(1);
    let (view_tx,view_rx)=mpsc::sync_channel::<String>(2);
    let mut view_pending=false;
    loop {
        app.tick();
        if app.dirty || app.effects_running() && last_draw.elapsed()>=Duration::from_millis(34) {
            let elapsed=last_draw.elapsed();
            terminal.draw(|frame|app.render(frame,elapsed))?;
            app.dirty=false;
            last_draw=Instant::now();
        }
        if event::poll(Duration::from_millis(34))? { app.input(event::read()?); }
        while let Some(intent)=app.take_intent() {
            if !mutations_allowed.load(Ordering::SeqCst) && !matches!(intent,LiveIntent::Detach) {
                if matches!(intent,LiveIntent::SubmitTask(_)){if let Some(draft)=app.submission_draft.take(){app.composer=Composer::from_draft(draft);}}
                if matches!(&intent,LiveIntent::Dialogue(v) if matches!(v["action"].as_str(),Some("coordination.present"|"candidate.present"))){app.presentation_delivery_failed();}
                app.notice="Disconnected: draft retained; mutations wait for a verified service snapshot".into();
                continue;
            }
            let row=app.live.as_ref().and_then(|v|v.tasks.get(app.journal.selected_task)).cloned();
            let task_id=row.as_ref().and_then(|v|v["task_id"].as_str()).map(str::to_owned);
            match intent {
                LiveIntent::Dialogue(value)=>{let id=command_id(&mut counter);let mut payload=json!({"command_id":id,"action":value["action"],"payload":value["payload"]});if value["action"]=="resume_session"{payload["expected_state_revision"]=json!(revision.load(Ordering::SeqCst));}match enqueue_mutation(&outbox_dir,&commands_tx,id.clone(),payload){Ok(())=>{app.provider_command_started(value["action"].as_str().unwrap_or(""),&id);if !matches!(value["action"].as_str(),Some("coordination.present"|"candidate.present")){app.notice="Sending · delivery pending".into();}},Err(e)=>app.notice=format!("Delivery needs attention: {e}")}},
                scope_intent @ (LiveIntent::SaveScope(_)|LiveIntent::AuthorizeScope(_))=>{
                    let saving=matches!(scope_intent,LiveIntent::SaveScope(_));let payload=match scope_intent{LiveIntent::SaveScope(p)|LiveIntent::AuthorizeScope(p)=>p,_=>unreachable!()};let id=command_id(&mut counter);
                    let command=json!({"command_id":id,"action":if saving{"save_proposal_revision"}else{"authorize_scope"},"payload":payload});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id.clone(),command){
                        Ok(())=>{if saving{if let Some(edit)=&mut app.scope_edit{edit["pending_command_id"]=json!(id);}}app.notice=if saving{"Scope revision sent; waiting for durable acceptance"}else if app.development_mode(){"Scope and policy approval sent; waiting for durable acceptance"}else{"Scope authorization sent; execution unavailable"}.into();},
                        Err(e)=>app.notice=format!("Scope delivery needs attention: {e}; edit retained"),
                    }
                },
                LiveIntent::ContinueAnswer=>{
                    let Some(question)=app.saved_scope_answer().cloned()else{app.notice="No saved answer needs continuation".into();continue};
                    let id=command_id(&mut counter);let payload=json!({"command_id":id,"action":"submit_message","expected_state_revision":revision.load(Ordering::SeqCst),"payload":{"text":question["answer"],"intent":"discussion","task_id":if task_id.as_ref().is_some_and(|id|question["scope"]["linked_task_ids"].as_array().is_some_and(|a|a.contains(&json!(id)))){json!(task_id)}else{question["scope"]["target_task_id"].clone()},"answer_question_id":question["question_id"]}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){Ok(())=>app.notice="Saved answer submitted once; coordinator continuation uses one call".into(),Err(e)=>app.notice=format!("Continuation delivery needs attention: {e}")}
                },
                LiveIntent::Detach=>{app.detached=true;},
                LiveIntent::SubmitTask(text)=>{
                    let original_draft=app.submission_draft.take();
                    let id=command_id(&mut counter);
                    let conversation=app.live.as_ref().is_some_and(|v|v.conversation.is_some());
                    let intent=match app.journal.target{
                        waywr::prototype::scenario::Target::Coordinator=>"discussion",
                        waywr::prototype::scenario::Target::NewTask=>"task_request",
                        _=>"steer",
                    };
                    let mut message_payload=if app.continuous_mode(){app.message_payload(&text)}else if app.scope_mode(){json!({"text":text,"intent":"discussion","task_id":if app.journal.target==waywr::prototype::scenario::Target::Coordinator{task_id.clone()}else{None}})}else if conversation{json!({"text":text,"intent":intent,
                        "task_id":if intent=="steer"{task_id.clone()}else{None}})}
                        else{json!({"text":text})};
                    if app.coordination_mode(){message_payload["interaction"]["provenance"]["literal_input"]=json!(original_draft.as_ref().is_some_and(|d|d.literal_paste));}
                    let payload=if app.development_mode()&&intent=="steer"{json!({"command_id":id,"action":"development.steer","payload":app.development_steering_payload(&text).ok_or_else(||io::Error::other("Select an approved workflow before steering"))?})}else{json!({"command_id":id,"action":if app.coordination_mode(){"coordination.submit"}else{"submit_message"},
                        "expected_state_revision":revision.load(Ordering::SeqCst),"payload":message_payload})};
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload) {
                        Ok(())=>{app.message_submitted();app.conversation_revision_request=false;app.notice=if conversation{"Sending · delivery pending"}else{"Task request sent; waiting for durable acceptance"}.into();},
                        Err(error)=>{
                            if !error.to_string().contains("saved for reconciliation") {
                                app.composer=Composer::from_draft(original_draft.unwrap_or(Draft{text,..Draft::default()}));
                            }
                            app.notice=format!("Delivery needs attention: {error}");
                        }
                    }
                },
                LiveIntent::Plan | LiveIntent::Inspect=>{
                    if let Some(task_id)=task_id {
                        let kind=if matches!(intent,LiveIntent::Plan)&&!app.scope_mode(){"task.plan"}else{"task.get"};
                        if kind!="task.get" || detail_request_id.is_none(){
                            let id=command_id(&mut counter);
                            if commands_tx.try_send((kind.into(),id.clone(),json!({"task_id":task_id}))).is_err(){
                                app.notice="Request queue full; try again after responses arrive".into();
                            }else if kind=="task.get"{detail_request_id=Some(id);last_detail_fetch=Instant::now();}
                        }
                    } else if app.live.as_ref().is_some_and(|v|v.conversation.is_some()){
                        app.notice="Conversation, questions and budgets are shown in this project's Inspect panel".into();
                    } else {app.notice="Select a task first".into();}
                },
                LiveIntent::Consult=>{
                    let action=app.live.as_ref().and_then(|v|v.conversation.as_ref())
                        .and_then(|v|v["actions"].as_array())
                        .and_then(|items|items.iter().find(|a|a["kind"]=="request_consultation"));
                    let Some(action_id)=action.and_then(|v|v["action_id"].as_str()) else{
                        app.notice="No consultation is awaiting authorization".into();continue;
                    };
                    let id=command_id(&mut counter);
                    let payload=json!({"command_id":id,"action":"authorize_consultation",
                        "expected_state_revision":revision.load(Ordering::SeqCst),
                        "payload":{"action_id":action_id}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                        Ok(())=>app.notice="One mock consultation authorization sent; awaiting durable receipt".into(),
                        Err(e)=>app.notice=format!("Consultation delivery needs attention: {e}"),
                    }
                },
                LiveIntent::ApplySteering=>{
                    let action=app.live.as_ref().and_then(|v|v.conversation.as_ref())
                        .and_then(|v|v["actions"].as_array())
                        .and_then(|items|items.iter().find(|a|a["kind"]=="propose_steering" &&
                            a["body"]["payload"]["task_id"].as_str()==task_id.as_deref()));
                    let Some(action_id)=action.and_then(|v|v["action_id"].as_str()) else{
                        app.notice="No steering suggestion matches this selected task".into();continue;
                    };
                    let Some(row)=row.as_ref() else{app.notice="Select the proposed task first".into();continue;};
                    let id=command_id(&mut counter);
                    let payload=json!({"command_id":id,"action":"apply_conversation_steering",
                        "expected_state_revision":revision.load(Ordering::SeqCst),
                        "expected_task_version":row["version"],
                        "expected_task_state_revision":row["state_revision"],
                        "payload":{"action_id":action_id}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                        Ok(())=>app.notice="Steering request sent; the new scope still needs review".into(),
                        Err(e)=>app.notice=format!("Steering delivery needs attention: {e}"),
                    }
                },
                LiveIntent::ReplaceCoordinator=>{
                    let generation=app.live.as_ref().and_then(|v|v.conversation.as_ref())
                        .and_then(|v|v["binding"]["generation"].as_i64());
                    let Some(generation)=generation else{
                        app.notice="This project has no replaceable mock coordinator binding".into();continue;
                    };
                    let id=command_id(&mut counter);
                    let payload=json!({"command_id":id,"action":"select_coordinator",
                        "expected_generation":generation,
                        "payload":if let Some(chat)=app.chat(){json!({"authorize":true,"profile_sha256":chat["profile_sha256"]})}else{json!({"provider":"fake","model":"deterministic","effort":"none"})}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                        Ok(())=>app.notice=if app.chat().is_some(){"Coordinator allowance sent; awaiting durable receipt"}else{"Mock binding replacement sent; awaiting durable receipt"}.into(),
                        Err(e)=>app.notice=format!("Binding delivery needs attention: {e}"),
                    }
                },
                LiveIntent::Output=>{
                    if operation_request_id.is_some(){
                        // The open inspector will request its full list after the
                        // current background metadata read settles.
                        last_operation_fetch=Instant::now()-Duration::from_secs(1);
                        continue;
                    }
                    if let Some(task_id)=task_id{
                        let id=command_id(&mut counter);
                        if commands_tx.try_send(("operation.list".into(),id.clone(),json!({"task_id":task_id}))).is_err(){
                            app.notice="Request queue full; output remains available through CLI logs".into();
                        }else{operation_request_id=Some(id);last_operation_fetch=Instant::now();}
                    }else{app.notice="Select a task before opening output".into();}
                },
                LiveIntent::HerdrView=>{
                    if view_pending{app.notice="Herdr view request is still resolving".into();continue;}
                    let endpoint=env::var("WAYWR_HERDR_VIEW_SOCKET").ok().or_else(||
                        if env::var("HERDR_ENV").ok().as_deref()==Some("1"){
                            env::var("HERDR_SOCKET_PATH").ok()
                        }else{None});
                    let Some(endpoint)=endpoint else{
                        app.notice="No Herdr view endpoint selected. Use WAYWR_HERDR_VIEW_SOCKET or CLI observe --viewer herdr --socket PATH".into();continue;
                    };
                    let Some(project)=project.clone() else{
                        app.notice="Installed project identity unavailable for optional view".into();continue;
                    };
                    let Some(operation)=app.worker_read_request() else{
                        app.notice="Open /output and select an operation first".into();continue;
                    };
                    let task=operation["task_id"].as_str().unwrap_or("").to_owned();
                    let stage=operation["stage"].as_str().unwrap_or("").to_owned();
                    let Some(ipc_base)=Path::new(&socket).parent().and_then(Path::parent) else{
                        app.notice="Project IPC location is invalid".into();continue;
                    };
                    let args=vec!["--project".into(),project.to_string_lossy().into_owned(),
                        "--ipc-dir".into(),ipc_base.to_string_lossy().into_owned()];
                    let sender=view_tx.clone();view_pending=true;
                    thread::spawn(move||{
                        let result=paired_root().and_then(|(root,manifest)|
                            rust_views::open(&root,&manifest,&args,&task,&stage,&endpoint));
                        let _=sender.send(match result{Ok(message)=>message,
                            Err(error)=>format!("Herdr view unavailable: {error}. Output remains in Waywr.")});
                    });
                },
                LiveIntent::Authorize=>{
                    let result=app.live.as_ref().and_then(|v|v.plan.as_ref());
                    let Some(plan)=result.map(|v|&v["plan"]) else {app.notice="Open the current scope first".into();continue;};
                    if plan["task_id"].as_str()!=task_id.as_deref() {app.notice="Selection changed; load its scope again".into();continue;}
                    let id=command_id(&mut counter);
                    let payload=json!({"command_id":id,"action":"admit_task",
                        "expected_task_version":plan["task_version"],"expected_state_revision":plan["task_state_revision"],
                        "payload":{"task_id":plan["task_id"],"run_id":plan["run_id"],"workspace":plan["workspace"],
                        "profile":plan["profile"],"context":plan["context"],
                        "plan_digest":result.unwrap()["plan_digest"]}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                        Ok(())=>app.notice="Authorization sent; waiting for durable acceptance".into(),
                        Err(e)=>app.notice=format!("Authorization delivery needs attention: {e}"),
                    }
                },
                decision @ (LiveIntent::Answer(_)|LiveIntent::Approve|LiveIntent::Reject(_))=>{
                    if let LiveIntent::Answer(ref answer)=decision{
                        let pending=app.live.as_ref().and_then(|v|v.conversation.as_ref())
                            .and_then(|v|v["questions"].as_array()).and_then(|items|items.iter().find(|q|q["status"]=="open"&&(!app.continuous_mode()||q["scope"]["subject_id"]==app.conversation_subject)));
                        if let Some(question_id)=pending.and_then(|v|v["question_id"].as_str()){
                            let id=command_id(&mut counter);
                            let payload=json!({"command_id":id,"action":"answer_conversation",
                                "expected_state_revision":revision.load(Ordering::SeqCst),
                                "payload":{"question_id":question_id,"answer":answer,"coordination_version":if app.coordination_mode(){json!(1)}else{Value::Null},"decision_revision":pending.map(|v|v["scope"]["decision_revision"].clone()),"question_revision":pending.map(|v|v["scope"]["question_revision"].clone())}});
                            match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                                Ok(())=>app.notice="Answer sent; awaiting durable receipt".into(),
                                Err(e)=>app.notice=format!("Answer delivery needs attention: {e}"),
                            }
                            continue;
                        }
                    }
                    let Some(detail)=app.live.as_ref().and_then(|v|v.detail.as_ref()) else {app.notice="Load the selected task's pending gate first".into();continue;};
                    let pending=&detail["pending"];
                    let kind=pending["kind"].as_str().unwrap_or("");
                    let (outcome,content)=match decision {
                        LiveIntent::Answer(answer)=>{if kind!="choice" {app.notice="No choice question is pending".into();continue;} ("answered",answer)},
                        LiveIntent::Approve=>{if kind!="final-review" {app.notice="No final review is pending".into();continue;}
                            let sha=pending["artifact_sha256"].as_str();
                            if app.live.as_ref().and_then(|v|v.reviewed_sha.as_deref())!=sha {app.notice="Open and verify this exact candidate before approval".into();continue;}
                            ("approved","Approved".into())},
                        LiveIntent::Reject(reason)=>{if kind!="final-review" {app.notice="No final review is pending".into();continue;} ("rejected",reason)},
                        _=>unreachable!(),
                    };
                    if detail["task"]["task_id"].as_str()!=task_id.as_deref(){app.notice="Selected task changed; reload its gate".into();continue;}
                    let id=command_id(&mut counter);
                    let payload=json!({"command_id":id,"action":"answer_task",
                        "expected_task_version":detail["task"]["version"],
                        "expected_state_revision":detail["task"]["state_revision"],
                        "payload":{"task_id":task_id,"request_id":pending["request_id"],
                            "outcome":outcome,"answer":content,"artifact_sha256":pending["artifact_sha256"]}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                        Ok(())=>app.notice="Decision sent; awaiting durable receipt".into(),
                        Err(e)=>app.notice=format!("Decision delivery needs attention: {e}"),
                    }
                },
                LiveIntent::OpenArtifact | LiveIntent::OpenArtifactNext=>{
                    let next=matches!(intent,LiveIntent::OpenArtifactNext);
                    if artifact_request_id.is_some(){app.notice="Candidate page is still loading".into();continue;}
                    let detail=app.live.as_ref().and_then(|v|v.detail.as_ref());
                    let requested=detail.and_then(|v|v["pending"]["artifact_sha256"].as_str());
                    let selected=detail.and_then(|v|v["task"]["task_id"].as_str());
                    if selected!=task_id.as_deref() || requested.is_none(){
                        app.notice="Load this task's final review before opening its artifact".into();continue;
                    }
                    let offset=if next{app.live.as_ref().map_or(0,|v|v.artifact_next_offset)}else{0};
                    if !next {if let Some(live)=app.live.as_mut(){live.artifact=None;live.reviewed_sha=None;
                        live.artifact_next_offset=0;live.artifact_complete=false;}}
                    let id=command_id(&mut counter);
                    if commands_tx.try_send(("artifact.read".into(),id.clone(),
                        json!({"task_id":task_id,"artifact":"latest","offset":offset,"limit":16*1024}))).is_ok(){
                        artifact_request_id=Some(id);app.modal=Modal::Artifact;
                        app.notice="Loading an exact-hash verified candidate page".into();
                    }else{app.notice="Request queue full; candidate review remains unapproved".into();}
                },
                LiveIntent::OpenFile(name)=>{
                    if artifact_request_id.is_some(){app.notice="Candidate page is still loading".into();continue;}
                    let detail=app.live.as_ref().and_then(|v|v.detail.as_ref());
                    let selected=detail.and_then(|v|v["task"]["task_id"].as_str());
                    if selected!=task_id.as_deref() || detail.and_then(|v|v["pending"]["artifact_sha256"].as_str()).is_none(){
                        app.notice="Load this task's final review before opening a candidate file".into();continue;
                    }
                    let id=command_id(&mut counter);
                    if commands_tx.try_send(("artifact.read".into(),id.clone(),
                        json!({"task_id":task_id,"artifact":name,"offset":0,"limit":32*1024}))).is_ok(){
                        artifact_request_id=Some(id);app.modal=Modal::Artifact;
                        app.notice="Loading a verified candidate file".into();
                    }else{app.notice="Request queue full; file preview was not sent".into();}
                },
                LiveIntent::Cancel=>{
                    let active_chat=app.live.as_ref().and_then(|v|v.conversation.as_ref()).and_then(|v|v["turns"].as_array()).is_some_and(|a|a.iter().any(|v|v["status"]=="reserved"));
                    if app.development_mode()&&!active_chat&&let Some(work)=app.development_workflow(){let id=command_id(&mut counter);let payload=json!({"command_id":id,"action":"development.cancel","payload":{"work_id":work["work_id"],"revision":work["revision"]}});match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){Ok(())=>app.notice="Development cancellation requested; retained work is preserved".into(),Err(e)=>app.notice=format!("Cancellation delivery needs attention: {e}")};continue;}
                    if app.chat().is_some(){
                        let turn=app.live.as_ref().and_then(|v|v.conversation.as_ref()).and_then(|v|v["turns"].as_array())
                            .and_then(|a|a.iter().find(|v|v["status"]=="reserved")).and_then(|v|v["turn_id"].as_str());
                        let Some(turn)=turn else{app.notice="No coordinator call is active".into();continue;};
                        let id=command_id(&mut counter);let payload=json!({"command_id":id,"action":"cancel_conversation","payload":{"turn_id":turn}});
                        match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){Ok(())=>app.notice="Coordinator interruption requested".into(),Err(e)=>app.notice=format!("Interruption delivery needs attention: {e}")};continue;
                    }
                    let Some(row)=row else {app.notice="Select a task first".into();continue;};
                    let id=command_id(&mut counter);
                    let payload=json!({"command_id":id,"action":"cancel_task",
                        "expected_task_version":row["version"],"expected_state_revision":row["state_revision"],
                        "payload":{"task_id":row["task_id"]}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id,payload){
                        Ok(())=>app.notice="Cancellation requested; awaiting service result".into(),
                        Err(e)=>app.notice=format!("Cancellation delivery needs attention: {e}"),
                    }
                },
                LiveIntent::Exit=>{
                    if exit_preparation.is_some()||exit_command_id.is_some(){app.notice="Exit delivery pending; no duplicate request".into();continue;}
                    let id=command_id(&mut counter);
                    if commands_tx.try_send(("snapshot".into(),id.clone(),json!({}))).is_ok(){exit_preparation=Some(id);app.notice="Preparing exit from current project state…".into();}else{app.notice="Exit preparation busy; choose Exit again".into();}
                },
            }
            app.dirty=true;
        }
        if (app.modal==Modal::Worker || app.has_active_workers()) && operation_request_id.is_none() &&
            last_operation_fetch.elapsed()>=Duration::from_secs(1){
            if let Some(task_id)=app.live.as_ref().and_then(|live|live.tasks.get(app.journal.selected_task))
                .and_then(|task|task["task_id"].as_str()){
                let id=command_id(&mut counter);
                if commands_tx.try_send(("operation.list".into(),id.clone(),json!({"task_id":task_id,"active_only":app.modal!=Modal::Worker}))).is_ok(){
                    operation_request_id=Some(id);last_operation_fetch=Instant::now();
                }
            }
        }
        if app.modal==Modal::Worker && output_request_id.is_none() &&
            last_output_fetch.elapsed()>=Duration::from_millis(250) {
            if let Some(body)=app.worker_read_request(){
                let id=command_id(&mut counter);
                if commands_tx.try_send(("operation.output.read".into(),id.clone(),body)).is_ok(){
                    output_request_id=Some(id);last_output_fetch=Instant::now();
                }
            }
        }
        while let Ok(update)=updates_rx.try_recv() {
            let detail_reply=detail_request_id.as_deref()==Some(update.request_id.as_str());
            if detail_reply{detail_request_id=None;}
            if operation_request_id.as_deref()==Some(update.request_id.as_str()){
                operation_request_id=None;
            }
            if output_request_id.as_deref()==Some(update.request_id.as_str()){
                output_request_id=None;
            }
            if artifact_request_id.as_deref()==Some(update.request_id.as_str()){
                artifact_request_id=None;
            }
            if exit_preparation.as_deref()==Some(update.request_id.as_str()){
                exit_preparation=None;
                if update.kind=="snapshot"&&update.body["state_revision"].is_u64(){
                    let id=command_id(&mut counter);let payload=json!({"command_id":id,"action":"stop_session","expected_state_revision":update.body["state_revision"],"payload":{}});
                    match enqueue_mutation(&outbox_dir,&commands_tx,id.clone(),payload){Ok(())=>{exit_command_id=Some(id);app.notice="Exit requested; awaiting durable acknowledgement".into();},Err(e)=>app.notice=format!("Exit delivery needs attention: {e}")}
                }else{app.notice="Exit preparation failed; reconnect, then choose Exit again".into();}
            }
            if exit_command_id.as_deref()==Some(update.request_id.as_str()) {
                if update.kind=="command_result" && update.body["status"]=="accepted" {
                    exit_result=Some(update.body.clone());app.detached=true;
                } else {app.notice=format!("Exit not committed: {}",update.body);exit_command_id=None;}
            }
            if let Some(submitted)=update.body.get("local_refused_submission"){
                if app.composer.text().is_empty()&&(!app.continuous_mode()||submitted["subject_id"]==app.conversation_subject){
                    app.composer=Composer::from_draft(Draft{text:submitted["text"].as_str().unwrap_or("").into(),literal_paste:true,..Draft::default()});
                    app.conversation_revision_request=submitted["request_revision"]==true;
                    app.retain_submission_context(submitted);
                }
            }
            match update.kind.as_str() {
                "snapshot"=>{
                    if let Some(value)=update.body["state_revision"].as_u64(){
                        revision.fetch_max(value,Ordering::SeqCst);
                    }
                    let was_disconnected=app.journal.connection!="connected";
                    app.apply_snapshot(&update.body);
                    app.journal.connection="connected".into();
                    if was_disconnected {app.notice="Reconnected to this project's service".into();}
                },
                "events_result"=>{
                    if let Some(events)=update.body["events"].as_array(){for event in events{app.apply_event(event);}}
                },
                "task.plan_result"=>app.apply_plan(update.body),
                "operation.list_result"=>app.apply_operation_list(&update.body),
                "operation.output.page"=>{
                    let text=update.body["page"]["segments"].as_array()
                        .ok_or_else(||io::Error::other("Output page lacks segments"))
                        .and_then(|segments|segments.iter().map(|segment|{
                            let bytes=page_bytes(segment["data"].as_str()
                                .ok_or_else(||io::Error::other("Output segment lacks bytes"))?)?;
                            Ok(format!("[{}] {}",segment["stream"].as_str().unwrap_or("output"),
                                terminal_text(&bytes)))
                        }).collect::<io::Result<String>>());
                    match text{Ok(text)=>app.apply_worker_output(&update.body,&text),
                        Err(error)=>app.notice=format!("Output page invalid: {error}")}
                },
                "artifact.page"=>{
                    match update.body["data"].as_str().ok_or_else(||io::Error::other("Artifact page lacks bytes"))
                        .and_then(page_bytes){
                        Ok(bytes)=>{
                            if update.body["artifact"]=="latest"{
                                app.apply_artifact_page(&update.body,&terminal_text(&bytes));
                                if update.body["complete"]==true{app.notice="Exact candidate fully loaded and verified".into();}
                            }else{app.apply_file_page(&update.body,&terminal_text(&bytes));}
                        },Err(error)=>app.notice=format!("Candidate page invalid: {error}; approval blocked"),
                    }
                },
                "task.result"=>app.apply_detail(update.body),
                "error"=>{
                    if update.body["code"]!="service_unavailable"{app.provider_command_result(&update.request_id,&update.body);}
                    if detail_reply {app.pending_action_failed();}
                    if matches!(update.body["code"].as_str(),Some("service_unavailable"|"resync_required")) {
                        app.journal.connection="reconnecting".into();
                    }
                    app.notice=format!("{} · {}",update.body["code"].as_str().unwrap_or("Service error"),
                        update.body["message"].as_str().unwrap_or("Inspect this project's service"));
                },
                "command_result"=>{
                    app.provider_command_result(&update.request_id,&update.body);
                    if app.scope_edit.as_ref().is_some_and(|e|e["pending_command_id"].as_str()==Some(update.request_id.as_str())){
                        if update.body["status"]=="accepted"{app.scope_edit=None;}else if let Some(e)=app.scope_edit.as_mut(){e.as_object_mut().unwrap().remove("pending_command_id");}
                    }
                    if update.body["status"]=="rejected" {
                        app.presentation_delivery_failed();
                        app.notice=if update.body["code"]=="stale_state"{"Project state changed; action was not accepted. Review refreshed state and choose the action again.".into()}else{format!("Action refused: {} · {}",update.body["code"],update.body["message"])};
                        if update.body.get("local_refused_submission").is_some(){app.notice.push_str(" · Input retained; no automatic resend");}
                    } else if update.body["status"]=="accepted" {
                        if !app.message_acknowledgement(&update.request_id,&update.body)&&!update.body["card_id"].is_string(){app.notice="Action accepted".into();}
                        if let Some(live)=app.live.as_mut(){live.plan=None;}
                    }
                },
                _=>app.notice=format!("Service response: {}",update.kind),
            }
            app.dirty=true;
        }
        if detail_request_id.is_none() && last_detail_fetch.elapsed()>=Duration::from_millis(500){
            let selected=app.live.as_ref().and_then(|live|live.tasks.get(app.journal.selected_task));
            // Scope detail includes live pending-turn/answer dependencies. A text-only
            // completion need not change the immutable proposal or task version.
            let stale=selected.is_some_and(|row|app.scope_mode() || (row["status"]=="awaiting-input" &&
                app.live.as_ref().and_then(|live|live.detail.as_ref()).is_none_or(|detail|
                    detail["task"]["task_id"]!=row["task_id"] || detail["task"]["version"]!=row["version"] ||
                    detail["task"]["state_revision"]!=row["state_revision"])));
            if stale {
                let id=command_id(&mut counter);
                if commands_tx.try_send(("task.get".into(),id.clone(),json!({"task_id":selected.unwrap()["task_id"]}))).is_ok(){
                    detail_request_id=Some(id);last_detail_fetch=Instant::now();
                }
            }
        }
        if let Ok(message)=view_rx.try_recv(){view_pending=false;app.notice=message;app.dirty=true;}
            let draft=app.input_history.draft_to_save(app.composer.snapshot());
        if draft.text!=last_draft.text || draft.row!=last_draft.row || draft.col!=last_draft.col || draft.multiline!=last_draft.multiline {
            if last_draft_save.elapsed()>=Duration::from_millis(250) {
                save_editor_draft(&draft_path,&draft)?;
                last_draft=draft;
                last_draft_save=Instant::now();
            }
        }
        if app.continuous_mode(){let value=app.conversation_ui_state();if value!=last_subject{waywr_backend::execution::write(&subject_path,&value,false).map_err(io::Error::other)?;last_subject=value;}}
        if app.scope_mode(){let value=app.scope_edit_snapshot();if value!=last_scope_draft{waywr_backend::execution::write(&scope_draft_path,&value,false).map_err(io::Error::other)?;last_scope_draft=value;}}
        if let Err(error) = app.input_history.save(&input_history_path) {
            if !error.to_string().starts_with("Already owned:") { return Err(error); }
        }
        if app.detached {break;}
    }
    if app.scope_mode(){waywr_backend::execution::write(&scope_draft_path,&app.scope_edit_snapshot(),false).map_err(io::Error::other)?;}
    if app.continuous_mode(){waywr_backend::execution::write(&subject_path,&app.conversation_ui_state(),false).map_err(io::Error::other)?;}
    save_editor_draft(&draft_path,&app.input_history.draft_to_save(app.composer.snapshot()))?;
    app.input_history.save(&input_history_path)?;
    drop(terminal);drop(_guard);
    if let Some(result)=exit_result {
        println!("Exit requested. Checkpoint: {}\nResume: cd {}; waywr resume",
            result["checkpoint"]["path"].as_str().unwrap_or("project state/session/checkpoints"),env::current_dir()?.display());
    }
    Ok(())
}

fn paired_root() -> io::Result<(PathBuf, Value)> {
    let executable = fs::canonicalize(env::current_exe()?)?;
    let root = executable.parent().and_then(|p| p.parent()).ok_or_else(|| io::Error::other("invalid paired release layout"))?.to_path_buf();
    let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    Ok((root,manifest))
}

fn selected_project(args: &[String]) -> io::Result<PathBuf> {
    let value=args.windows(2).find(|pair| pair[0]=="--project").map(|pair| PathBuf::from(&pair[1]));
    waywr_backend::project_workflow::canonical_project(&value.unwrap_or(env::current_dir()?)).map_err(io::Error::other)
}

// Projects belong to a storage/semantic contract, not an application build.
// A legacy release record is retained evidence; its old executable is not needed.
fn check_project_compatibility(root: &Path, manifest: &Value, project: &Path, args: &[String]) -> io::Result<()> {
    waywr_backend::project_layout::check(project).map_err(io::Error::other)?;
    rust_bundle::validate(root)?;
    let configuration=project.join(".waywr/project.toml");
    if !configuration.exists(){return Ok(());}
    let config:toml::Value=toml::from_str(&fs::read_to_string(configuration)?).map_err(io::Error::other)?;
    let key=config["project_key"].as_str().filter(|key|key.len()==32&&key.bytes().all(|b|b.is_ascii_hexdigit()))
        .ok_or_else(||io::Error::other("Invalid project identity"))?;
    let descriptor=project.join(".waywr/release.json");
    if descriptor.exists(){
        let record:Value=serde_json::from_slice(&fs::read(descriptor)?)?;
        if record["schema_version"]!=1||record["project_key"]!=key{
            return Err(io::Error::other("Legacy release record project identity mismatch"));
        }
    }
    let state=args.windows(2).find(|pair|pair[0]=="--state-dir")
        .map(|pair|PathBuf::from(&pair[1])).unwrap_or_else(||project.join(".waywr/state"));
    let state=if state.is_absolute(){state}else{project.join(state)};
    let contract=state.join("state-contract.json");
    if contract.exists(){
        let record:Value=serde_json::from_slice(&fs::read(contract)?)?;
        let schema=record["state_schema"].as_u64().ok_or_else(||io::Error::other("State schema missing"))?;
        if record["schema_version"]!=1||record["project_key"]!=key||record["execution_host"]!="systemd-user-v1"{
            return Err(io::Error::other("Project state identity or execution contract mismatch"));
        }
        if manifest["state_schema_min"].as_u64().unwrap_or(u64::MAX)>schema||
            manifest["state_schema_max"].as_u64().unwrap_or(0)<schema{
            return Err(io::Error::other("Installed application cannot read this project's storage schema; project data preserved"));
        }
    }
    rust_maintenance::require_recovery_contract(&state,manifest)?;
    for (marker,field,required) in [
        ("ollama-transport-v1.json","provider_transport_contract","ollama-http-v1"),
        ("context-delivery-v1.json","context_delivery_contract","evidence-v1"),
        ("context-memory-v2.json","context_memory_contract","working-memory-v2"),
    ]{
        if state.join(marker).exists()&&manifest[field]!=required{
            return Err(io::Error::other(format!("Retained project state requires {required}; project data preserved")));
        }
    }
    Ok(())
}

fn command_name(args: &[String]) -> Option<&str> {
    let mut index=0;
    while index<args.len() {
        let item=args[index].as_str();
        if matches!(item,"--project"|"--state-dir"|"--registry-dir"|"--legacy-origin"|"--color"|
                         "--herdr-socket"|"--ipc-dir"|"--release") {
            index+=2;continue;
        }
        if item.starts_with('-') { return None; }
        return Some(item);
    }
    None
}

fn legacy_command(args: &[String]) -> bool {
    matches!(command_name(args),Some("init"|"legacy-inspect"|"location-rebind"|"preflight"|"integration"|
            "adopt-saved-draft"|"adopt-provisional-revision"|"adopt-reviewed-critique"|
            "open"|"open-link"|"rebind"|"start"|"demo"|"open-demo"|"supervise"|
            "supervisor-install"|"notices"|"resume"|"status"|"workers"|"show"|
            "logs"|"observe"|"cancel"|"decide"|"approve"|"reject"|
            "probe-start"|"probe-resume"|"probe-cancel"|"probe-observe"))
}

fn session_resume(args: &[String]) -> bool {
    if command_name(args) != Some("resume") { return false; }
    let position = args.iter().position(|item| item == "resume").unwrap();
    !args.iter().skip(position + 1).any(|item| !item.starts_with('-'))
}

fn phase(stream: &mut UnixStream) -> io::Result<Value> {
    let reply = request(stream,"snapshot","launcher-phase",json!({}))?;
    if reply.kind != "snapshot" { return Err(io::Error::other("project session status unavailable")); }
    Ok(reply.body)
}

#[cfg(test)]
fn checkpoint_phase(state: &Path) -> Option<Value> {
    let root = state.join("session/checkpoints");
    let pointer: Value = serde_json::from_slice(&fs::read(root.join("latest.json")).ok()?).ok()?;
    let file = pointer["file"].as_str()?;
    if file.contains('/') || !file.ends_with(".json") { return None; }
    let bytes = fs::read(root.join(file)).ok()?;
    if format!("{:x}",Sha256::digest(&bytes)) != pointer["sha256"].as_str()? { return None; }
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    if value["project_key"] != pointer["project_key"] { return None; }
    Some(value["session"].clone())
}

fn legacy_cli(args: &[String]) -> io::Result<()> {
    let (root,manifest)=paired_root()?;
    rust_cli::legacy(&root,&manifest,args)
}

fn session_command(args: &[String]) -> io::Result<()> {
    let (root,manifest)=paired_root()?;
    rust_cli::session(&root,&manifest,args)
}

fn launch(args: &[String]) -> io::Result<()> {
    let (root,manifest)=paired_root()?;
    rust_runtime::launch(&root,&manifest,args)
}

fn value_from_args(args:&[String],flag:&str)->Option<String>{
    args.windows(2).find(|pair|pair[0]==flag).map(|pair|pair[1].clone())
}

fn print_version()->io::Result<()> {
    let root=env::current_exe()?.canonicalize()?;
    let manifest=root.parent().and_then(|p|p.parent()).map(|p|p.join("manifest.json"));
    let release=manifest.and_then(|p|fs::read(p).ok()).and_then(|bytes|serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|m|m["release_id"].as_str().filter(|id|!id.is_empty()&&id.len()<=64&&
            id.bytes().all(|b|b.is_ascii_alphanumeric()||b"._-".contains(&b))).map(str::to_owned));
    println!("waywr {} ({})",env!("CARGO_PKG_VERSION"),release.as_deref().unwrap_or("unbundled"));
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().flat_map(|arg| { if arg.starts_with("--") { if let Some((k,v))=arg.split_once('=') {return vec![k.to_owned(),v.to_owned()]} } vec![arg] }).collect();
    match cli_schema::validate(&args[1..]) {
        Ok(false)=>return,
        Err(error)=>{eprintln!("waywr: {error}");std::process::exit(2)},
        Ok(true)=>{},
    }
    let result = if command_name(&args[1..]).is_none()&&args.iter().any(|x|matches!(x.as_str(),"--version"|"-V")){print_version()}
        else if args.iter().any(|x| x == "--backend-info") { backend_info() }
        else if args.iter().any(|x|x=="--view-output") { rust_views::viewer(&args[1..]) }
        else if command_name(&args[1..])==Some("bundle-build") { rust_bundle::build(&args[1..]) }
        else if command_name(&args[1..])==Some("bundle-install") { rust_bundle::install(&args[1..]) }
        else {
            let value = |flag: &str| -> Option<String> { args.windows(2).find(|pair| pair[0] == flag).map(|pair| pair[1].clone()) };
            let socket = value("--socket").or_else(|| env::var("WAYWR_SESSION_SOCKET").ok());
            let key = value("--project-key");
            let release = value("--release-id");
            match (socket,key,release) {
                (Some(s),Some(k),Some(r)) if args.iter().any(|x| x == "--protocol-probe") => protocol_probe(&s,&k,&r),
                (Some(s),Some(k),Some(r)) if args.iter().any(|x| x == "--lost-reply-probe") => {
                    let path = value("--outbox-dir").map(PathBuf::from).ok_or_else(|| io::Error::other("outbox directory required"));
                    path.and_then(|p| lost_reply_probe(&s,&k,&r,&p))
                },
                (Some(s),Some(k),Some(r)) => {
                    let outbox = value("--outbox-dir").map(PathBuf::from).unwrap_or_else(|| {
                        let base = env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"));
                        base.join("wayward-wrangler/client-outbox").join(&k)
                    });
                    run_ui(s,k,r,value("--title").unwrap_or_else(|| "Wayward Wrangler".into()),outbox,value("--ui-token"),None)
                },
                _ if session_resume(&args[1..]) => launch(&args[1..]),
                _ if legacy_command(&args[1..]) => legacy_cli(&args[1..]),
                _ if command_name(&args[1..])==Some("session") => session_command(&args[1..]),
                _ => launch(&args[1..]),
            }
        };
    if let Err(error) = result {
        if let (Some(token),Some(outbox))=(value_from_args(&args,"--ui-token"),value_from_args(&args,"--outbox-dir")) {
            let directory=PathBuf::from(outbox).join("ui-launch-errors");
            if token.len()==32 && token.bytes().all(|byte|byte.is_ascii_hexdigit()) &&
                fs::create_dir_all(&directory).is_ok() {
                let _=fs::set_permissions(&directory,fs::Permissions::from_mode(0o700));
                let path=directory.join(format!("{token}.json"));
                if let Ok(mut file)=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
                    let _=serde_json::to_writer(&mut file,&json!({"schema_version":1,"ui_token":token,
                        "error":error.to_string()}));
                    let _=file.sync_all();
                }
            }
        }
        eprintln!("waywr: {error}"); std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconnect_retries_lost_snapshot_after_successful_handshake(){
        let root=tempfile::tempdir().unwrap();let socket=root.path().join("peer.sock");
        let server=std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let owner=thread::spawn(move||{
            for attempt in 0..2{
                let (mut peer,_)=server.accept().unwrap();let hello=protocol::read_frame(&mut peer).unwrap();
                assert_eq!(hello.kind,"hello");protocol::write_frame(&mut peer,&Frame::new("hello_result",&hello.request_id,json!({"accepted":true}))).unwrap();
                if attempt==0{continue;}
                let query=protocol::read_frame(&mut peer).unwrap();assert_eq!(query.kind,"snapshot");
                protocol::write_frame(&mut peer,&Frame::new("snapshot",&query.request_id,json!({"cursor":2,"state_revision":2}))).unwrap();
            }
        });
        let start=Instant::now();let (_,snapshot)=reconnect_snapshot(socket.to_str().unwrap(),"project","release",1).unwrap();
        assert_eq!(snapshot.body["cursor"],2);assert!(start.elapsed()<Duration::from_secs(1));owner.join().unwrap();
    }
    #[test]
    fn outbox_persists_exact_payload_for_replay() {
        let root = tempfile::tempdir().unwrap();
        let body = json!({"command_id":"test-1", "payload":{"text":"hello"}});
        persist_outbox(root.path(), "test-1", &body).unwrap();
        assert_eq!(pending_outbox(root.path()).unwrap(), vec![("test-1".into(),body)]);
        let permissions = fs::metadata(root.path().join("test-1.json")).unwrap().permissions().mode() & 0o777;
        assert_eq!(permissions, 0o600);
    }
    #[test]
    fn compatibility_dispatch_uses_the_actual_subcommand() {
        assert!(!legacy_command(&["--project".into(),"status".into()]));
        assert!(legacy_command(&["--project".into(),"status".into(),"status".into(),"demo".into()]));
        assert!(!legacy_command(&["--project".into(),"/tmp/project".into()]));
        assert!(session_resume(&["--project".into(),"/tmp/project".into(),"resume".into()]));
        assert!(session_resume(&["--project".into(),"/tmp/project".into(),"--herdr-socket".into(),
                                 "/tmp/herdr.sock".into(),"--ipc-dir".into(),"/tmp/runtime".into(),"resume".into()]));
        assert!(!session_resume(&["resume".into(),"historical-run".into()]));
    }
    #[test]
    fn draft_and_checkpoint_pointer_are_versioned_and_validated() {
        let root=tempfile::tempdir().unwrap();
        let draft=root.path().join("session/draft.json");
        fs::create_dir_all(draft.parent().unwrap()).unwrap();
        fs::write(&draft,br#"{"schema_version":1,"text":"line one\nline two"}"#).unwrap();
        assert_eq!(load_editor_draft(&draft).unwrap().text,"line one\nline two");
        let selected=Draft{text:"another\nline".into(),row:1,col:3,multiline:true,literal_paste:true};
        save_editor_draft(&draft,&selected).unwrap();
        let restored=load_editor_draft(&draft).unwrap();
        assert_eq!(restored.text,selected.text);
        assert_eq!((restored.row,restored.col,restored.multiline,restored.literal_paste),(1,3,true,true));
        let checkpoint=root.path().join("session/checkpoints");
        fs::create_dir_all(&checkpoint).unwrap();
        let payload=json!({"project_key":"selected","session":{"phase":"paused","generation":2}});
        fs::write(checkpoint.join("2-test.json"),serde_json::to_vec(&payload).unwrap()).unwrap();
        let bytes=fs::read(checkpoint.join("2-test.json")).unwrap();
        fs::write(checkpoint.join("latest.json"),serde_json::to_vec(&json!({
            "project_key":"selected","file":"2-test.json","sha256":format!("{:x}",Sha256::digest(&bytes))})).unwrap()).unwrap();
        assert_eq!(checkpoint_phase(root.path()).unwrap()["phase"],"paused");
        fs::write(checkpoint.join("2-test.json"),b"changed").unwrap();
        assert!(checkpoint_phase(root.path()).is_none());
    }
}
