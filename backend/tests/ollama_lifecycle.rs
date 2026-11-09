//! Real runner + loopback HTTP fixtures. No user manager or model is invoked.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use waywr_backend::{
    execution::{read, write},
    host_attempt::Attempt,
    operation,
    systemd_host::HostStatus,
};

struct Server {
    endpoint: String,
    chats: Arc<AtomicUsize>,
    release: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(mode: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let chats = Arc::new(AtomicUsize::new(0));
        let release = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (count, go, quit) = (chats.clone(), release.clone(), stop.clone());
        let join = thread::spawn(move || {
            while !quit.load(Ordering::SeqCst) {
                let (socket, _) = match listener.accept() {
                    Ok(v) => v,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(socket);
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let route = line.split_whitespace().nth(1).unwrap().to_owned();
                let mut length = 0;
                loop {
                    line.clear();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = n.trim().parse::<usize>().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let mut socket = reader.into_inner();
                let body = match route.as_str() {
                    "/api/version" => json!({"version":"fixture"}).to_string(),
                    "/api/tags" => {
                        json!({"models":[{"name":"fixture:1","digest":"sha256:fixture"}]})
                            .to_string()
                    }
                    "/api/chat" => {
                        count.fetch_add(1, Ordering::SeqCst);
                        let request: Value = serde_json::from_slice(&body).unwrap();
                        assert_eq!(request["model"], "fixture:1");
                        assert_eq!(request["stream"], true);
                        if mode == "hold" {
                            while !go.load(Ordering::SeqCst) && !quit.load(Ordering::SeqCst) {
                                thread::sleep(Duration::from_millis(5));
                            }
                        }
                        if mode == "lost" {
                            continue;
                        }
                        let result = json!({"schema_version":1,"summary":"HTTP fixture inspected supplied exact commit only","reviewed_commits":["abc"],"requires_changes":false,"findings":[]});
                        format!(
                            "{}\n{}\n",
                            json!({"model":"fixture:1","message":{"content":result.to_string(),"thinking":"NEVER_PERSIST_HIDDEN"},"done":false}),
                            json!({"model":"fixture:1","done":true,"done_reason":"stop","eval_count":12})
                        )
                    }
                    other => panic!("Unexpected HTTP route: {other}"),
                };
                // A failed write says nothing about server generation completion.
                let _ = write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        Self {
            endpoint,
            chats,
            release,
            stop,
            join: Some(join),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.release.store(true, Ordering::SeqCst);
        self.join.take().unwrap().join().unwrap();
    }
}
fn until(mut predicate: impl FnMut() -> bool) {
    let limit = Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(Instant::now() < limit, "Fixture failed to settle");
        thread::sleep(Duration::from_millis(10));
    }
}
struct Run {
    _temp: tempfile::TempDir,
    attempt: Attempt,
    child: Option<Child>,
}
impl Run {
    fn new(server: &Server) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state");
        let op = state.join("session/development/r/review");
        fs::create_dir_all(&op).unwrap();
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
        let packet = json!({"schema_version":1,"development":true,"operation_kind":"development-review","operation_id":"review","run_id":"r","stage":"review","role":"review","provider":"ollama","model":"fixture:1","review_targets":[{"result_commit":"abc"}],"execution":{"profile":"development-profiles-v1","provider":"ollama","model":"fixture:1","effort":"provider_default","context_bytes":98304,"transport":{"kind":"ollama-http-v1","endpoint":server.endpoint,"server_version":"fixture","digest":"sha256:fixture","capabilities":["completion"],"options":{"num_ctx":32768,"num_predict":4096}}}});
        operation::prepare_host(&state, &op, &packet, &"a".repeat(32)).unwrap();
        let attempt = Attempt::create(
            &state,
            &op,
            &"a".repeat(32),
            Path::new(env!("CARGO_BIN_EXE_waywr-runner")),
            "fixture",
            7,
        )
        .unwrap();
        Self {
            _temp: temp,
            attempt,
            child: None,
        }
    }
    fn start(&mut self) {
        let a = &self.attempt;
        a.mark_maybe_sent().unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_waywr-runner"))
            .args([
                "host-run",
                "--operation-dir",
                a.operation.to_str().unwrap(),
                "--state-dir",
                a.state.to_str().unwrap(),
                "--attempt-id",
                &a.id,
            ])
            .env("INVOCATION_ID", "b".repeat(32))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        self.child = Some(child);
        until(|| a.directory.join("claim.json").exists());
        let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup"))
            .unwrap()
            .lines()
            .find_map(|s| s.strip_prefix("0::").map(str::to_owned))
            .unwrap();
        a.permit(
            &HostStatus {
                name: a.unit.clone(),
                active: "active".into(),
                sub: "running".into(),
                invocation_id: "b".repeat(32),
                control_group: cgroup,
                main_pid: pid,
                result: "success".into(),
                restart: "no".into(),
                exit_type: "cgroup".into(),
                remain_after_exit: true,
                collect_mode: "inactive".into(),
            },
            7,
        )
        .unwrap();
    }
    fn finish(&mut self) -> Value {
        until(|| self.child.as_mut().unwrap().try_wait().unwrap().is_some());
        let a = &self.attempt;
        a.validated_receipt().unwrap().unwrap_or_else(|| {
            panic!(
                "No receipt; diagnostics: {}",
                fs::read_to_string(a.directory.join("runner-diagnostic.log")).unwrap()
            )
        })
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
#[test]
fn ollama_http_cancel_before_send_has_positive_nonlaunch() {
    let server = Server::new("ok");
    let run = Run::new(&server);
    run.attempt.request_cancel("cancel-before").unwrap();
    run.attempt.mark_maybe_sent().unwrap();
    let a = &run.attempt;
    let status = Command::new(env!("CARGO_BIN_EXE_waywr-runner"))
        .args([
            "host-run",
            "--operation-dir",
            a.operation.to_str().unwrap(),
            "--state-dir",
            a.state.to_str().unwrap(),
            "--attempt-id",
            &a.id,
        ])
        .env("INVOCATION_ID", "b".repeat(32))
        .output()
        .unwrap();
    assert!(!status.status.success());
    assert!(a.directory.join("pre-dispatch-refusal.json").exists());
    assert!(!a.operation.join("invocation-reservation.json").exists());
    assert_eq!(server.chats.load(Ordering::SeqCst), 0);
}
#[test]
fn ollama_http_cancel_after_send_settles_client_without_claiming_server_completion() {
    let server = Server::new("hold");
    let mut run = Run::new(&server);
    run.start();
    until(|| server.chats.load(Ordering::SeqCst) == 1);
    assert_eq!(
        run.attempt.request_cancel("cancel-after").unwrap()["status"],
        "cancellation-requested"
    );
    let receipt = run.finish();
    assert_eq!(receipt["status"], "cancelled");
    let proof = read(&run.attempt.directory.join("completion.json")).unwrap();
    assert_eq!(proof["settled"], true);
    assert_eq!(proof["action_valid"], false);
    assert!(proof["remote_completion"]
        .as_str()
        .unwrap()
        .starts_with("unknown"));
    assert!(!run.attempt.directory.join("result.json").exists());
    assert_eq!(server.chats.load(Ordering::SeqCst), 1);
}
#[test]
fn ollama_http_terminal_receipt_survives_owner_restart_and_wins_late_cancel() {
    let server = Server::new("ok");
    let mut run = Run::new(&server);
    run.start();
    let receipt = run.finish();
    assert_eq!(receipt["status"], "completed");
    let a = &run.attempt;
    write(
        &a.state.join("supervision/owner.json"),
        &json!({"generation":8}),
        false,
    )
    .unwrap();
    let adopted = Attempt::load(&a.state, &a.operation).unwrap();
    assert_eq!(adopted.validated_receipt().unwrap().unwrap(), receipt);
    assert_eq!(
        adopted.request_cancel("late-cancel").unwrap()["status"],
        "completed-race"
    );
    assert_eq!(adopted.validated_receipt().unwrap().unwrap(), receipt);
    assert!(!fs::read_to_string(a.directory.join("transcript.log"))
        .unwrap()
        .contains("NEVER_PERSIST_HIDDEN"));
    assert_eq!(server.chats.load(Ordering::SeqCst), 1);
}
#[test]
fn ollama_http_ambiguous_loss_never_allows_duplicate_dispatch() {
    let server = Server::new("lost");
    let mut run = Run::new(&server);
    run.start();
    let receipt = run.finish();
    assert_eq!(receipt["status"], "failed");
    let a = &run.attempt;
    let proof = read(&a.directory.join("completion.json")).unwrap();
    assert_eq!(proof["known_completion"], false);
    assert_eq!(proof["settled"], true);
    assert_eq!(proof["category"], "uncertain");
    assert!(Attempt::create(
        &a.state,
        &a.operation,
        &"a".repeat(32),
        Path::new(env!("CARGO_BIN_EXE_waywr-runner")),
        "fixture",
        7
    )
    .is_err());
    assert!(a.retire_proven_no_launch("connection closed").is_err());
    assert_eq!(server.chats.load(Ordering::SeqCst), 1);
}

#[test]
fn ollama_http_terminal_response_and_cancel_race_has_one_effective_outcome() {
    let server = Server::new("hold");
    let mut run = Run::new(&server);
    run.start();
    until(|| server.chats.load(Ordering::SeqCst) == 1);
    server.release.store(true, Ordering::SeqCst);
    let cancellation = run.attempt.request_cancel("terminal-race").unwrap();
    let receipt = run.finish();
    if cancellation["status"] == "completed-race" {
        assert_eq!(receipt["status"], "completed");
        assert_eq!(cancellation["receipt"], receipt);
    } else {
        assert_eq!(receipt["status"], "cancelled");
        assert!(receipt["result_sha256"].is_null());
    }
    assert_eq!(run.attempt.validated_receipt().unwrap().unwrap(), receipt);
    assert_eq!(server.chats.load(Ordering::SeqCst), 1);
}
