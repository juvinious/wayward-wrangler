//! A runner is an actual process in an owned Herdr pane. It records immutable
//! execution evidence and never advances workflow state or answers a gate.
use crate::{
    adapters,
    codec::canonical_sha256,
    execution::{FileLock, alive, now, process_identity, read, write, write_bytes},
    fake,
    herdr::Herdr,
    host_attempt::Attempt,
    output,
    output_capture::{self,Recorder},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::process::CommandExt,
    path::{Path,PathBuf},
    process::Stdio,
    thread,
    time::{Duration, Instant},
};
use tokio::{io::{AsyncRead,AsyncReadExt},process::Command as TokioCommand,
    sync::mpsc,time::timeout};

const STREAM_QUEUE_CHUNKS:usize=16;
const TRANSCRIPT_LIMIT:usize=output_capture::TRANSCRIPT_LIMIT;
const EVENT_LIMIT:usize=50_000;

// Herdr's pane is the live display, not completion evidence. A stalled PTY
// must not stop pipe draining, receipts, or cancellation. The complete bounded
// transcript remains available when display output is dropped.
fn display_bytes(fd:i32,bytes:&[u8])->bool{
    let flags=unsafe{libc::fcntl(fd,libc::F_GETFL)};
    if flags<0 || unsafe{libc::fcntl(fd,libc::F_SETFL,flags|libc::O_NONBLOCK)}<0{return false;}
    let deadline=Instant::now()+Duration::from_millis(500);
    let mut offset=0;
    while offset<bytes.len(){
        let length=(bytes.len()-offset).min(4096);
        let written=unsafe{libc::write(fd,bytes[offset..offset+length].as_ptr().cast(),length)};
        if written>0{offset+=written as usize;continue;}
        if written==0 || Instant::now()>=deadline{return false;}
        let code=std::io::Error::last_os_error().raw_os_error();
        if !matches!(code,Some(libc::EAGAIN|libc::EINTR)){return false;}
        let mut poll=libc::pollfd{fd,events:libc::POLLOUT,revents:0};
        unsafe{libc::poll(&mut poll,1,50)};
    }
    true
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().ok_or_else(|| format!("missing {key}"))
}
fn evidence(directory: &Path, name: &str, value: &Value) -> Result<(), String> {
    write(&directory.join(name), value, true)
}
fn reserve_native(directory: &Path, op: &Value) -> Result<(), String> {
    let run = directory
        .parent()
        .and_then(Path::parent)
        .ok_or("operation lacks run directory")?;
    let _lock = FileLock::exclusive(&run.join("invocation-budget.lock"))?;
    let limit = op["max_invocations"]
        .as_u64()
        .ok_or("missing native invocation limit")?;
    if !(1..=5).contains(&limit) || directory.join("invocations.json").exists() {
        return Err("Duplicate dispatch or frozen native invocation limit".into());
    }
    let mut used = 0u64;
    for entry in fs::read_dir(directory.parent().ok_or("operation lacks stage parent")?)
        .map_err(|e| e.to_string())?
    {
        let path = entry
            .map_err(|e| e.to_string())?
            .path()
            .join("invocations.json");
        if path.exists() {
            used += read(&path)?["count"]
                .as_u64()
                .ok_or("invalid invocation counter")?;
        }
    }
    if used >= limit {
        return Err("Duplicate dispatch or frozen native invocation limit".into());
    }
    evidence(
        directory,
        "invocations.json",
        &json!({"operation_id":op["operation_id"],"count":1,
        "meaning":"reserved native process launch attempt"}),
    )
}

fn report(herdr: &Herdr, pane: &str, op: &Value, state: &str, message: &str) {
    let provider = op["provider"].as_str().unwrap_or("fake");
    let role = op["role"].as_str().unwrap_or("worker");
    let model = op["model"].as_str().unwrap_or("");
    let label = format!(
        "Wayward Wrangler {} {} {provider}/{model} {}",
        op["run_id"].as_str().unwrap_or(""),
        role,
        op["stage"].as_str().unwrap_or("")
    );
    let _ = herdr.call(
        "pane.report_agent",
        json!({"pane_id":pane,"source":"awp-runner",
        "agent":format!("awp-{provider}"),"state":state,"message":message}),
    );
    let _ = herdr.call(
        "pane.report_metadata",
        json!({"pane_id":pane,"source":"awp-runner",
        "title":format!("{label} {message}"),"display_agent":format!("{role} {provider}/{model}")}),
    );
}

fn terminate_group(identity: &Value, signal: i32) -> Result<(), String> {
    if !alive(identity) {
        return Ok(());
    }
    let pid = identity["pid"].as_i64().ok_or("invalid child PID")? as libc::pid_t;
    if unsafe { libc::getpgid(pid) } != pid {
        return Err("Child process group changed; no signal sent".into());
    }
    // Verify process identity immediately before targeting its own process group.
    if !alive(identity) {
        return Err("Child identity changed before group signal".into());
    }
    if unsafe { libc::kill(-pid, signal) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

struct StreamChunk {
    name: &'static str,
    bytes: Vec<u8>,
    error: Option<String>,
}
type WorkerOutcome = (String, Option<i32>, Option<Value>, Option<String>);
fn output_failure(runtime:&tokio::runtime::Runtime,child:&mut tokio::process::Child,
    identity:&Value,directory:&Path,reason:&str,retained:usize,observed:usize,
    stream:&str)->Result<WorkerOutcome,String>{
    terminate_group(identity,libc::SIGTERM)?;
    let code=match runtime.block_on(async {timeout(Duration::from_secs(3),child.wait()).await}){
        Ok(Ok(status))=>status.code(),
        Ok(Err(error))=>return Err(error.to_string()),
        Err(_)=>{terminate_group(identity,libc::SIGKILL)?;
            runtime.block_on(child.wait()).map_err(|e|e.to_string())?.code()}
    };
    evidence(directory,"output-limit.json",&json!({"schema_version":1,
        "limit_bytes":TRANSCRIPT_LIMIT,"index_limit_records":output_capture::INDEX_LIMIT,
        "retained_bytes":retained,"observed_at_least":observed,"stream":stream,
        "truncated":true,"reason":reason,"policy":"fail invocation without retry"}))?;
    Ok(("failed".into(),code,None,Some(reason.into())))
}
fn invalid_output(runtime:&tokio::runtime::Runtime,child:&mut tokio::process::Child,
    identity:&Value,directory:&Path,recorder:&mut Recorder,reason:&str)->Result<WorkerOutcome,String>{
    terminate_group(identity,libc::SIGTERM)?;
    let code=match runtime.block_on(async {timeout(Duration::from_secs(3),child.wait()).await}){
        Ok(Ok(status))=>status.code(),Ok(Err(error))=>return Err(error.to_string()),
        Err(_)=>{terminate_group(identity,libc::SIGKILL)?;
            runtime.block_on(child.wait()).map_err(|e|e.to_string())?.code()}
    };
    recorder.flush()?;
    evidence(directory,"output-invalid.json",&json!({"schema_version":1,
        "reason":reason,"retained_bytes":recorder.committed(),
        "policy":"failed invocation without retry"}))?;
    Ok(("failed".into(),code,None,Some(reason.into())))
}
// Corroborate settled process ownership after positive child wait evidence.
fn group_settled(identity:&Value)->Result<bool,String>{
 let pgid=identity["pid"].as_u64().ok_or("Missing child identity")?;
 for entry in fs::read_dir("/proc").map_err(|e|e.to_string())? {
  let path=entry.map_err(|e|e.to_string())?.path();if path.file_name().is_none_or(|v|!v.to_string_lossy().bytes().all(|b|b.is_ascii_digit())){continue}
  let raw=match fs::read_to_string(path.join("stat")){Ok(v)=>v,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>continue,Err(e)=>return Err(format!("Cannot establish process-group settlement: {e}"))};
  let fields=raw.rsplit_once(')').ok_or("Invalid process stat")?.1.split_whitespace().collect::<Vec<_>>();
  if fields.get(2).and_then(|v|v.parse::<u64>().ok())==Some(pgid)&&fields.first().is_some_and(|v|*v!="Z"){return Ok(false)}
 }Ok(true)
}
async fn stream(mut pipe: impl AsyncRead + Unpin + Send + 'static, name: &'static str, tx: mpsc::Sender<StreamChunk>) {
        let mut buf = [0u8; 65536];
        loop {
            match pipe.read(&mut buf).await {
                Ok(0) => break,
                Err(error) => {
                    let _=tx.send(StreamChunk{name,bytes:Vec::new(),
                        error:Some(format!("{name} pipe read failed: {error}"))}).await;
                    break;
                },
                Ok(n) => {
                    if tx
                        .send(StreamChunk {
                            name,
                            bytes: buf[..n].to_vec(),
                            error:None,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
}
fn execute(
    directory: &Path,
    op: &Value,
    packet: &Value,
    me: &Value,
    display: Option<(&Herdr, &str)>,
) -> Result<WorkerOutcome, String> {
    if directory.join("cancel.json").exists(){
        return Ok(("cancelled".into(),None,None,Some("Cancelled before child spawn".into())));
    }
    let provider = required(op, "provider")?;
    let development=packet["development"]==true;
    let local=packet["operation_kind"]=="development-check";
    let native = provider != "fake" && !local;
    let chat=packet["operation_kind"]=="conversation";
    let continuous=packet["execution"]["profile"]=="claude-proposal-v2";
    let (argv, environment) = if native {
        if development {
            match crate::development::prepare_native(directory,packet){
                Ok(prepared)=>prepared,
                Err(reason) if packet["execution"]["profile"]=="development-profiles-v1"=>{
                    // This branch precedes provider process construction/spawn. It is positive nonlaunch evidence.
                    evidence(directory,"completion.json",&json!({"schema_version":1,"provider_selection_v1":true,"category":"configuration","known_completion":true,"settled":true,"capabilities_valid":true,"action_valid":false,"no_provider_process_spawned":true,"reason":reason}))?;
                    return Ok(("failed".into(),None,None,Some(reason)));
                },Err(reason)=>return Err(reason)
            }
        }else{return Err("Legacy native execution is retired; configure the shared model runtime".into())}
    } else if local {
        (vec![std::env::current_exe().map_err(|e|e.to_string())?.to_string_lossy().into_owned(),"development-check-child".into(),directory.join("packet.json").to_string_lossy().into_owned()],{let mut env=crate::adapters::environment("codex",&std::env::vars().collect())?;let home=std::env::var("HOME").unwrap_or_default();env.insert("PATH".into(),format!("{home}/.cargo/bin:/usr/local/bin:/usr/bin:/bin"));env.insert("CARGO_TARGET_DIR".into(),PathBuf::from(packet["worktree"].as_str().ok_or("Missing check worktree")?).join("target").to_string_lossy().into_owned());env})
    } else {
        (
            vec![
                std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .into_owned(),
                "fake-child".into(),
                directory.join("packet.json").to_string_lossy().into_owned(),
                directory
                    .join("invocations.json")
                    .to_string_lossy()
                    .into_owned(),
            ],
            BTreeMap::from([
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("LANG".into(), "C.UTF-8".into()),
                ("HOME".into(), directory.to_string_lossy().into_owned()),
            ]),
        )
    };
    if native && op["schema_version"]!=2 {
        reserve_native(directory, op)?;
    }
    let mut transcript=Recorder::create(directory)?;
    evidence(
        directory,
        "dispatch.json",
        &json!({"runner":me,"time":now()}),
    )?;
    let runtime=tokio::runtime::Builder::new_multi_thread().worker_threads(2)
        .enable_all().build().map_err(|e|e.to_string())?;
    let mut command = TokioCommand::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(if development&&packet["operation_kind"]=="development-worker"{PathBuf::from(required(packet,"worktree")?)}else if chat||development{directory.join("client-scratch")}else{directory.to_path_buf()})
        .env_clear()
        .envs(environment)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.as_std_mut().process_group(0);
    if native {
        command.stdin(Stdio::from(
            File::open(directory.join("prompt.txt")).map_err(|e| e.to_string())?,
        ));
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = runtime.block_on(async {command.spawn()})
        .map_err(|e| format!("Spawn outcome uncertain after dispatch reservation: {e}"))?;
    let identity = process_identity(child.id().ok_or("Child lacks PID after spawn")?)?;
    evidence(
        directory,
        "child.json",
        &json!({"identity":identity,"pgid":child.id()}),
    )?;
    if let Some((herdr,pane))=display {report(herdr, pane, op, "working", "running");}
    println!(
        "RUNNING child PID {}; Ctrl-C cancels this worker only.",
        child.id().unwrap_or(0)
    );
    let (tx, mut rx) = mpsc::channel(STREAM_QUEUE_CHUNKS);
    let stdout_task=runtime.spawn(stream(
        child.stdout.take().ok_or("missing child stdout")?,
        "stdout",
        tx.clone(),
    ));
    let stderr_task=runtime.spawn(stream(
        child.stderr.take().ok_or("missing child stderr")?,
        "stderr",
        tx.clone(),
    ));
    drop(tx);
    let mut stdout = Vec::new();
    let mut events = Vec::new();
    let mut line = Vec::new();
    let mut retained=0usize;
    let mut display_degraded=false;
    let mut chat_text=String::new();
    let mut structured_text=String::new();let mut structured_index=None;let mut last_progress=Instant::now()-Duration::from_secs(1);
    let started = Instant::now();
    let mut sent_term = None::<Instant>;
    let mut exit = None;
    loop {
        if exit.is_none() {
            exit = child.try_wait().map_err(|e| e.to_string())?;
        }
        if native && !development && started.elapsed() > Duration::from_secs(if chat{120}else{600}) && exit.is_none() {
            terminate_group(&identity, libc::SIGTERM)?;
            if chat{return output_failure(&runtime,&mut child,&identity,directory,"Native invocation deadline exceeded; no retry",transcript.committed(),retained,"deadline");}
            return Err("Native worker exceeded ten-minute limit; no retry".into());
        }
        if directory.join("cancel.json").exists() && exit.is_none() {
            if sent_term.is_none() {
                terminate_group(&identity, libc::SIGTERM)?;
                sent_term = Some(Instant::now());
            } else if sent_term.is_some_and(|at| at.elapsed() > Duration::from_secs(3)) {
                terminate_group(&identity, libc::SIGKILL)?;
            }
        }
        match runtime.block_on(async {timeout(Duration::from_millis(100),rx.recv()).await}) {
            Ok(Some(chunk)) => {
                if let Some(error)=chunk.error{
                    return output_failure(&runtime,&mut child,&identity,directory,&error,
                        transcript.committed(),retained,chunk.name);
                }
                if retained.saturating_add(chunk.bytes.len())>TRANSCRIPT_LIMIT{
                    let allowed=TRANSCRIPT_LIMIT-retained;
                    if let Err(error)=transcript.push(chunk.name,&chunk.bytes[..allowed]){
                        return output_failure(&runtime,&mut child,&identity,directory,&error,
                            transcript.committed(),retained+chunk.bytes.len(),chunk.name);
                    }
                    if let Err(error)=transcript.flush(){
                        return output_failure(&runtime,&mut child,&identity,directory,&error,
                            transcript.committed(),retained+chunk.bytes.len(),chunk.name);
                    }
                    return output_failure(&runtime,&mut child,&identity,directory,
                        "Combined stdout/stderr exceeded 32 MiB",transcript.committed(),
                        retained+chunk.bytes.len(),chunk.name);
                }
                retained+=chunk.bytes.len();
                if let Err(error)=transcript.push(chunk.name,&chunk.bytes){
                    return output_failure(&runtime,&mut child,&identity,directory,&error,
                        transcript.committed(),retained,chunk.name);
                }
                if display.is_some() && !display_degraded && !display_bytes(if chunk.name=="stderr"{2}else{1},&chunk.bytes){
                    display_degraded=true;
                    evidence(directory,"display-degraded.json",&json!({"schema_version":1,
                        "reason":"Herdr terminal output stalled or closed; live display truncated",
                        "transcript":"transcript.log","operator_action":"Inspect the retained transcript and receipt"}))?;
                }
                if chunk.name == "stdout" {
                    stdout.extend_from_slice(&chunk.bytes);
                    if native {
                        let mut file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(directory.join("native-events.jsonl"))
                            .map_err(|e| e.to_string())?;
                        file.write_all(&chunk.bytes).map_err(|e| e.to_string())?;
                        file.sync_all().map_err(|e| e.to_string())?;
                    }
                    for byte in &chunk.bytes {
                        if *byte == b'\n' {
                            if !line.iter().all(u8::is_ascii_whitespace) {
                                let value: Value = match if continuous{crate::scope_contract::strict_json(std::str::from_utf8(&line).unwrap_or(""))}else{serde_json::from_slice(&line).map_err(|e|e.to_string())}{
                                    Ok(value)=>value,
                                    Err(error)=>return invalid_output(&runtime,&mut child,&identity,
                                        directory,&mut transcript,&format!("Invalid structured event: {error}")),
                                };
                                if native {
                                    if let Err(error)=if development{crate::development::inspect_native(&value,packet)}else if continuous{crate::continuous_adapter::inspect(&value,packet["model"].as_str().unwrap_or(""))}else if chat{crate::claude_chat::inspect(&value,packet["model"].as_str().unwrap_or(""))}else{adapters::inspect_event(provider,&value)}{
                                        return invalid_output(&runtime,&mut child,&identity,
                                            directory,&mut transcript,&error);
                                    }
                                }
                                if development {
                                    crate::model_runtime::progress(directory,&value)?;
                                    if chat&&value["type"]=="runtime.text" {
                                        chat_text.push_str(value["text"].as_str().unwrap_or(""));
                                        if chat_text.len()>65536{return invalid_output(&runtime,&mut child,&identity,directory,&mut transcript,"Conversation stream exceeds bound")}
                                        write(&directory.join("conversation-stream.json"),&json!({"schema_version":1,"text":chat_text,"provisional":true}),false)?;
                                    }
                                }else if continuous {
                                    let ev=&value["event"];
                                    if ev["type"]=="content_block_start"&&ev["content_block"]["name"]=="StructuredOutput"{structured_index=ev["index"].as_u64();}
                                    if ev["delta"]["type"]=="input_json_delta"&&structured_index.is_some()&&structured_index==ev["index"].as_u64(){structured_text.push_str(ev["delta"]["partial_json"].as_str().unwrap_or(""));}
                                    if let Some(text)=crate::claude_chat::delta(&value){chat_text.push_str(text);}
                                    if structured_text.len()>65536||chat_text.len()>32768{return invalid_output(&runtime,&mut child,&identity,directory,&mut transcript,"Conversation output exceeds bound")}
                                    if last_progress.elapsed()>=Duration::from_millis(100){
                                      let preview=if structured_text.is_empty(){chat_text.clone()}else{crate::scope_contract::provisional(&structured_text)};
                                      write(&directory.join("conversation-stream.json"),&json!({"schema_version":1,"text":preview,"provisional":true}),false)?;last_progress=Instant::now();
                                    }
                                }else if chat {if let Some(delta)=crate::claude_chat::delta(&value){
                                    if chat_text.len()+delta.len()>if packet["execution"]["profile"]=="claude-proposal-v1"{crate::scope_contract::RESULT_LIMIT}else{crate::claude_chat::TEXT_LIMIT}{return invalid_output(&runtime,&mut child,&identity,directory,&mut transcript,"Conversation text exceeds 32 KiB");}
                                    chat_text.push_str(delta);
                                    write(&directory.join("conversation-stream.json"),&json!({"schema_version":1,"text":if packet["execution"]["profile"]=="claude-proposal-v1"{crate::scope_contract::provisional(&chat_text)}else{chat_text.clone()},"provisional":true}),false)?;
                                }}
                                if events.len()>=EVENT_LIMIT{return invalid_output(&runtime,&mut child,&identity,
                                    directory,&mut transcript,"Worker structured-event count exceeds 50,000");}
                                events.push(value);
                            }
                            line.clear();
                        } else {
                            line.push(*byte);
                        }
                    }
                }
            }
            Ok(None) if exit.is_some() => break,
            Ok(None) | Err(_) => {
                if let Err(error)=transcript.flush(){
                    return output_failure(&runtime,&mut child,&identity,directory,&error,
                        transcript.committed(),retained,"index");
                }
            }
        }
    }
    runtime.block_on(async {let _=stdout_task.await;let _=stderr_task.await;});
    transcript.flush()?;
    if !line.iter().all(u8::is_ascii_whitespace) {
        let value: Value = if continuous{crate::scope_contract::strict_json(std::str::from_utf8(&line).map_err(|e|e.to_string())?)?}else{serde_json::from_slice(&line).map_err(|e|e.to_string())?};
        if native {
            if development{crate::development::inspect_native(&value,packet)?;}else{adapters::inspect_event(provider, &value)?;}
        }
        if events.len()>=EVENT_LIMIT{return Err("Worker structured-event count exceeds 50,000".into());}
        events.push(value);
    }
    let status = runtime.block_on(child.wait()).map_err(|e| e.to_string())?;
    if directory.join("cancel.json").exists() {
        if development {evidence(directory,"completion.json",&json!({"schema_version":1,"category":"interrupted","known_completion":true,"settled":group_settled(&identity)?,"capabilities_valid":true,"action_valid":false,"client_abandoned":packet["provider"]=="ollama","remote_completion":"unknown after cancellation; no action adopted"}))?;}
        if continuous {evidence(directory,"completion.json",&json!({"schema_version":1,"category":"interrupted","known_completion":true,"settled":group_settled(&identity)?,"capabilities_valid":events.iter().all(|e|crate::continuous_adapter::inspect(e,packet["model"].as_str().unwrap_or("")).is_ok()),"exit_code":status.code(),"action_valid":false,"events_sha256":canonical_sha256(&json!(events)).map_err(|e|e.to_string())?}))?;}
        return Ok(("cancelled".into(), status.code(), None, None));
    }
    if development && native {
      let settled=group_settled(&identity)?;
      let decoded=crate::development::finish_native(&events,directory,packet,status.code());
      let terminal=events.iter().rev().find(|e|e["type"]=="runtime.completed");
      let refusal=terminal.filter(|e|e["category"]=="capacity").cloned();let capacity=refusal.is_some();
      let remote_unknown=terminal.is_none()||terminal.is_some_and(|e|e["outcome"]!="completed"&&e["request_sent"]==true&&e["response_complete"]!=true);
      let no_request=terminal.is_some_and(|e|e["request_sent"]==false);
      let category=if remote_unknown||!settled{"uncertain"}else if no_request{"configuration"}else if capacity{"capacity"}else if decoded.is_ok(){"completed"}else{"format"};
      evidence(directory,"completion.json",&json!({"schema_version":1,"provider_selection_v1":true,"category":category,"no_model_request_sent":no_request,"known_completion":status.code().is_some()&&!remote_unknown,"settled":settled,"capabilities_valid":true,"action_valid":decoded.is_ok()&&settled,"reason":decoded.as_ref().err(),"capacity":refusal}))?;
      if !settled{return Ok(("failed".into(),status.code(),None,Some("Worker descendants not quiescent; no synchronization".into())))}
      return match decoded {Ok((result,runtime))=>{evidence(directory,"runtime-result.json",&runtime)?;evidence(directory,"result.json",&result)?;Ok(("completed".into(),status.code(),Some(result),None))},Err(reason)=>Ok((if capacity{"capacity-waiting"}else{"failed"}.into(),status.code(),None,Some(reason)))};
    }
    if local && !group_settled(&identity)?{return Ok(("failed".into(),status.code(),None,Some("Combined check descendants are not quiescent; acceptance refused".into())))}
    if continuous {
      let mut proof=crate::continuous_adapter::completion(&events,packet["model"].as_str().unwrap_or(""),status.code());
      proof["settled"]=json!(group_settled(&identity)?);
      if proof["settled"]!=true{proof["category"]=json!("uncertain");proof["action_valid"]=json!(false);}
      evidence(directory,"completion.json",&proof)?;evidence(directory,"runtime-result.json",&crate::claude_chat::runtime(&events,packet["model"].as_str().unwrap_or("")))?;
      if proof["action_valid"]!=true{return Ok((if proof["category"]=="format"{"output-invalid"}else{"failed"}.into(),status.code(),None,Some(proof["reason"].as_str().unwrap_or("Invalid structured completion").into())))}
    }
    if !status.success() {
        if chat{evidence(directory,"runtime-result.json",&crate::claude_chat::runtime(&events,packet["model"].as_str().unwrap_or("")))?;}
        return Ok((
            "failed".into(),
            status.code(),
            None,
            Some(if chat{crate::claude_chat::failure_reason(&events)}else{format!("Worker exited {:?}", status.code())}),
        ));
    }
    let result = if native {
        let final_bytes = if provider == "codex" {
            Some(fs::read(directory.join("native-final.json")).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let interpreted=if continuous{crate::continuous_adapter::finish(&events,packet["model"].as_str().unwrap_or(""))}else if chat{crate::claude_chat::finish_profile(&events,packet["model"].as_str().unwrap_or(""),packet["execution"]["profile"]=="claude-proposal-v1")}else{adapters::finish(provider, &events, final_bytes.as_deref())};
        let (result,runtime)=match interpreted{Ok(v)=>v,Err(e)=>return Ok((if packet["execution"]["profile"]=="claude-proposal-v1"{"output-invalid"}else{"failed"}.into(),status.code(),None,Some(e)))};
        evidence(directory, "runtime-result.json", &runtime)?;
        result
    } else {
        let results = events
            .iter()
            .filter(|e| e["type"] == "result")
            .collect::<Vec<_>>();
        if results.len() != 1 {
            return Err("Expected one structured worker result".into());
        }
        results[0]["result"].clone()
    };
    if let Err(e)=output::validate(&result,packet){
        if chat{return Ok(("output-invalid".into(),status.code(),None,Some(e)))}
        return Err(format!("Output invalid: {e}"));
    }
    evidence(directory, "result.json", &result)?;
    Ok(("completed".into(), status.code(), Some(result), None))
}

pub fn run(directory: &Path, state: &Path, launch_token: &str) -> Result<(), String> {
    let directory = crate::execution::contained_operation(state, directory)?;
    let op = read(&directory.join("operation.json"))?;
    if required(&op, "launch_token")? != launch_token {
        return Err("Launch token mismatch".into());
    }
    if op["protocol_version"].as_u64().is_some_and(|v| v != 1) {
        return Err("Runner protocol mismatch".into());
    }
    let _lock = FileLock::exclusive(&directory.join("runner.lock"))?;
    if directory.join("claim.json").exists() {
        return Err("Duplicate runner refused: operation already claimed".into());
    }
    if !directory.join("launch.json").exists() {
        return Err("Runner lacks launch intent".into());
    }
    let pane = std::env::var("HERDR_PANE_ID")
        .map_err(|_| "Runner requires actual Herdr pane".to_owned())?;
    let tab =
        std::env::var("HERDR_TAB_ID").map_err(|_| "Runner requires actual Herdr tab".to_owned())?;
    let workspace = std::env::var("HERDR_WORKSPACE_ID")
        .map_err(|_| "Runner requires actual Herdr workspace".to_owned())?;
    let me = process_identity(std::process::id())?;
    evidence(
        &directory,
        "claim.json",
        &json!({"runner":me,"herdr":{"pane_id":pane,"tab_id":tab,"workspace_id":workspace},"time":now()}),
    )?;
    let launch = read(&directory.join("launch.json"))?;
    let socket = required(&launch["target"], "socket")?;
    let herdr = Herdr::new(socket);
    println!(
        "Wayward Wrangler {} {} {}/{} {}\nOperation: {}\nPacket: {}\nPane: {}\nWaiting for persisted dispatch permit.",
        op["run_id"].as_str().unwrap_or(""),
        op["role"].as_str().unwrap_or(""),
        op["provider"].as_str().unwrap_or(""),
        op["model"].as_str().unwrap_or(""),
        op["stage"].as_str().unwrap_or(""),
        op["operation_id"],
        op["packet_sha256"],
        pane
    );
    report(&herdr, &pane, &op, "blocked", "awaiting-dispatch");
    while !directory.join("permit.json").exists() && !directory.join("cancel.json").exists() {
        thread::sleep(Duration::from_millis(100));
    }
    let mut exit_code = None;
    let mut result = None;
    let mut error = None;
    let status = if directory.join("cancel.json").exists() {
        "cancelled".to_owned()
    } else {
        let outcome = (|| {
            let permit = read(&directory.join("permit.json"))?;
            if permit["runner"] != me || permit["launch_token"] != op["launch_token"] {
                return Err("Permit does not authorize this runner".to_owned());
            }
            let packet = read(&directory.join("packet.json"))?;
            if canonical_sha256(&packet).map_err(|e| e.to_string())? != op["packet_sha256"] {
                return Err("Packet was modified".into());
            }
            execute(&directory, &op, &packet, &me, Some((&herdr, &pane)))
        })();
        match outcome {
            Ok((status, code, value, reason)) => {
                exit_code = code;
                result = value;
                error = reason;
                status
            }
            Err(reason) => {
                let child = read(&directory.join("child.json")).ok();
                let live_child = child.as_ref().is_some_and(|value| alive(&value["identity"]));
                if live_child || (directory.join("dispatch.json").exists() && child.is_none()) {
                    if let Some(child) = child.as_ref() {
                        let _ = terminate_group(&child["identity"], libc::SIGTERM);
                    }
                    evidence(
                        &directory,
                        "uncertain.json",
                        &json!({"error":reason,"time":now()}),
                    )?;
                    report(&herdr, &pane, &op, "blocked", "uncertain");
                    println!("UNCERTAIN: {reason}; dispatch or owned child may have run. No retry.");
                    loop {
                        thread::sleep(Duration::from_secs(2));
                    }
                }
                error = Some(reason);
                "failed".to_owned()
            }
        }
    };
    let receipt = json!({"schema_version":1,"operation_id":op["operation_id"],
        "packet_sha256":op["packet_sha256"],"launch_token":op["launch_token"],
        "runner":me,"status":status,"exit_code":exit_code,"error":error,
        "completed":now(),"result_sha256":result.as_ref().map(|value|canonical_sha256(value).unwrap_or_default())});
    evidence(&directory, "receipt.json", &receipt)?;
    report(&herdr, &pane, &op, "idle", &status);
    let _=herdr.call("pane.rename",json!({"pane_id":pane,"label":format!("Wayward Wrangler {} {}",op["run_id"],status.to_uppercase())}));
    println!(
        "\n{}\nRetained output, no active agent.\nReceipt: {}\nTranscript: {}\nTerminal text is never a workflow decision.",
        status.to_uppercase(),
        directory.join("receipt.json").display(),
        directory.join("transcript.log").display()
    );
    loop {
        thread::sleep(Duration::from_secs(2));
    }
}

/// Systemd-hosted runner for the new execution contract. The process and its
/// child belong to the operation unit, not to a Herdr pane or the project
/// service. All externally meaningful outcomes remain in durable records.
pub fn run_host(operation: &Path, state: &Path, attempt_id: &str) -> Result<(), String> {
    let attempt=Attempt::load(state,operation)?;
    if attempt.id!=attempt_id{return Err("Host attempt identity mismatch".into());}
    let _lifetime=FileLock::exclusive(&attempt.directory.join("runner.lock"))?;
    let mut diagnostics=OpenOptions::new().create_new(true).write(true)
        .open(attempt.directory.join("runner-diagnostic.log")).map_err(|e|e.to_string())?;
    let invocation=std::env::var("INVOCATION_ID").map_err(|_|"Runner requires systemd invocation identity")?;
    let me=process_identity(std::process::id())?;
    writeln!(diagnostics,"host runner claimed unit={} invocation={} pid={}",attempt.unit,invocation,std::process::id())
        .map_err(|e|e.to_string())?;
    diagnostics.sync_all().map_err(|e|e.to_string())?;
    attempt.record_claim(&me,&invocation)?;
    let deadline=Instant::now()+Duration::from_secs(30);
    loop {
        if attempt.directory.join("retired.json").exists() ||
            attempt.directory.join("pre-dispatch-refusal.json").exists(){
            return Err("Host attempt retired before invocation".into());
        }
        if attempt.directory.join("cancel.json").exists(){
            attempt.refuse_before_dispatch(&me,"cancelled before invocation")?;
            return Err("Host attempt cancelled before invocation".into());
        }
        if attempt.directory.join("permit.json").exists(){
            match attempt.commit_spawn(&me){
                Ok(())=>break,
                Err(reason) if deadline.saturating_duration_since(Instant::now())>Duration::ZERO=>{
                    writeln!(diagnostics,"permit observed but dispatch not yet authorized: {reason}")
                        .map_err(|e|e.to_string())?;
                }
                Err(reason)=>{
                    attempt.refuse_before_dispatch(&me,&reason)?;
                    return Err(format!("Dispatch refused before invocation: {reason}"));
                }
            }
        } else if Instant::now()>=deadline {
            attempt.refuse_before_dispatch(&me,"permit deadline exceeded")?;
            return Err("Permit deadline exceeded without invocation".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
    let op=read(&attempt.operation.join("operation.json"))?;
    let packet=read(&attempt.operation.join("packet.json"))?;
    if canonical_sha256(&packet).map_err(|e|e.to_string())?!=op["packet_sha256"] {
        evidence(&attempt.directory,"uncertain.json",&json!({"schema_version":1,
            "reason":"Frozen packet changed after spawn commitment","time":now()}))?;
        return Err("Frozen packet changed after spawn commitment; execution uncertain".into());
    }
    // Child input stays inside its immutable attempt. This is a byte copy of
    // the logical operation packet; the hash was checked before publication.
    write_bytes(&attempt.directory.join("packet.json"),
        &fs::read(attempt.operation.join("packet.json")).map_err(|e|e.to_string())?,true)?;
    let outcome=execute(&attempt.directory,&op,&packet,&me,None);
    let (mut status,code,mut result,error)=match outcome {
        Ok(value)=>value,
        Err(reason)=>{
            let child=read(&attempt.directory.join("child.json")).ok();
            let possible=attempt.directory.join("dispatch.json").exists() ||
                child.as_ref().is_some_and(|value|alive(&value["identity"]));
            if possible {
                evidence(&attempt.directory,"uncertain.json",&json!({"schema_version":1,
                    "reason":reason,"child":child,"time":now()}))?;
                writeln!(diagnostics,"uncertain: {reason}").map_err(|e|e.to_string())?;
                diagnostics.sync_all().map_err(|e|e.to_string())?;
                return Err(format!("Execution uncertain: {reason}"));
            }
            ("failed".into(),None,None,Some(reason))
        }
    };
    let _project=FileLock::exclusive(&attempt.state.join("supervision/fence.lock"))?;
    let _operation=FileLock::exclusive(&attempt.operation.join("host-operation.lock"))?;
    // Cancellation and receipt publication share this gate. A cancellation
    // recorded first fences adoption even if decoding just finished. Keep the
    // original result and completion bytes as evidence, never overwrite them.
    if status == "completed" && attempt.directory.join("cancel.json").exists() {
        evidence(&attempt.directory,"adoption-disposition.json",&json!({"status":"cancelled","action_adopted":false,"reason":"Cancellation won the receipt publication gate","retained_result_sha256":result.as_ref().map(|v|canonical_sha256(v).unwrap_or_default())}))?;
        status="cancelled".into(); result=None;
    }
    let continuous=packet["execution"]["profile"]=="claude-proposal-v2";
    let completion_sha=if continuous {
      let mut proof=read(&attempt.directory.join("completion.json")).unwrap_or(json!({"schema_version":1,"category":"unavailable","known_completion":false,"settled":false,"capabilities_valid":false,"action_valid":false}));
      if status=="output-invalid"&&proof["category"]=="completed"{proof["category"]=json!("integrity");proof["action_valid"]=json!(false);}
      proof["packet_sha256"]=op["packet_sha256"].clone();proof["attempt_id"]=json!(attempt.id);proof["exit_code"]=json!(code);proof["status"]=json!(status);
      proof["native_events_sha256"]=json!(fs::read(attempt.directory.join("native-events.jsonl")).ok().map(|b|crate::codec::sha256(&b)));
      if attempt.directory.join("completion.json").exists(){fs::rename(attempt.directory.join("completion.json"),attempt.directory.join("decoder-completion.json")).map_err(|e|e.to_string())?;}
      evidence(&attempt.directory,"completion.json",&proof)?;Some(canonical_sha256(&proof).map_err(|e|e.to_string())?)
    }else{None};
    let receipt=json!({"schema_version":if continuous{3}else{2},"completion_sha256":completion_sha,"operation_id":op["operation_id"],
        "attempt_id":attempt.id,"unit":attempt.unit,"invocation_id":invocation,
        "packet_sha256":op["packet_sha256"],"runner":me,"status":status,
        "exit_code":code,"error":error,"completed":now(),
        "result_sha256":result.as_ref().map(|value|canonical_sha256(value).unwrap_or_default())});
    evidence(&attempt.directory,"receipt.json",&receipt)?;
    writeln!(diagnostics,"receipt published status={status}").map_err(|e|e.to_string())?;
    diagnostics.sync_all().map_err(|e|e.to_string())?;
    Ok(())
}

pub fn fake_child(packet_file: &Path, counter_file: &Path) -> Result<(), String> {
    let packet = read(packet_file)?;
    if counter_file.exists() {
        return Err("Duplicate fake invocation refused".into());
    }
    evidence(
        counter_file.parent().ok_or("counter path has no parent")?,
        counter_file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("invalid counter name")?,
        &json!({"count":1}),
    )?;
    if packet["mode"]=="output-flood"{
        // Isolated migration fault fixture: exercise both real pipes and the
        // runner limit without calling an authenticated provider.
        let chunk=vec![b'X';32*1024];
        let mut stdout=std::io::stdout().lock();let mut stderr=std::io::stderr().lock();
        for _ in 0..1024{
            stdout.write_all(&chunk).map_err(|e|e.to_string())?;
            stderr.write_all(&chunk).map_err(|e|e.to_string())?;
        }
        return Ok(());
    }
    let seconds = packet["seconds"].as_f64().unwrap_or(0.1).clamp(0.0, 30.0);
    for event in fake::events(&packet)? {
        println!(
            "{}",
            serde_json::to_string(&event).map_err(|e| e.to_string())?
        );
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        thread::sleep(Duration::from_secs_f64(seconds / 6.0));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stalled_terminal_write_is_bounded_and_does_not_block_pipe_drain(){
        let mut fds=[-1,-1];assert_eq!(unsafe{libc::pipe(fds.as_mut_ptr())},0);
        let started=Instant::now();
        let visible=display_bytes(fds[1],&vec![b'x';1024*1024]);
        assert!(!visible);
        eprintln!("stalled_display_observed_max_ms={}",started.elapsed().as_millis());
        assert!(started.elapsed()<Duration::from_secs(1));
        unsafe{libc::close(fds[0]);libc::close(fds[1]);}
    }
    #[test]
    fn native_budget_reservation_is_durable_and_refuses_duplicates_or_sixth() {
        let root=tempfile::tempdir().unwrap();
        let run=root.path().join("runs/r");
        let stages=run.join("operations");
        fs::create_dir_all(&stages).unwrap();
        for number in 1..=5 {
            let directory=stages.join(format!("stage-{number}"));
            fs::create_dir_all(&directory).unwrap();
            let operation=json!({"operation_id":format!("op-{number}"),"max_invocations":5});
            reserve_native(&directory,&operation).unwrap();
            assert!(reserve_native(&directory,&operation).unwrap_err().contains("Duplicate dispatch"));
        }
        let sixth=stages.join("stage-6");
        fs::create_dir_all(&sixth).unwrap();
        assert!(reserve_native(&sixth,&json!({"operation_id":"op-6","max_invocations":5}))
            .unwrap_err().contains("limit"));
        assert!(!sixth.join("invocations.json").exists());
    }
}

#[cfg(test)]mod provider_preflight_tests{
    use super::*;
    #[test]fn provider_preflight_failure_has_positive_nonlaunch_proof_and_is_not_capacity(){
        let d=tempfile::tempdir().unwrap();let packet=json!({"development":true,"provider":"claude","operation_kind":"development-worker","execution":{"profile":"development-profiles-v1","provider":"claude","binary":"/missing/client","model":"claude-sonnet-5-5","effort":"medium"}});
        let outcome=execute(d.path(),&json!({"provider":"claude"}),&packet,&Value::Null,None).unwrap();
        assert_eq!(outcome.0,"failed");let proof=read(&d.path().join("completion.json")).unwrap();assert_eq!(proof["category"],"configuration");assert_eq!(proof["no_provider_process_spawned"],true);assert_eq!(proof["settled"],true);assert!(!d.path().join("child-process.json").exists());
    }
}
