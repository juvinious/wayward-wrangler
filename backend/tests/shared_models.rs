//! Shared provider selection through normal project commands; no model/account calls.
use serde_json::{json, Value};
use std::fs;
use waywr_backend::{execution::write, provider_selection as providers, session_store::Session};

fn preferences() -> Value {
    let profile = |provider: &str| json!({"provider":provider,"binary":"/usr/bin/node",
        "model":"catalog-independent-model","effort":"provider_default","options":{"temperature":0.2},
        "context_bytes":98304,"transport":{"entry":"/tmp/not-executed-runtime.mjs","config":null}});
    json!({"schema_version":3,"profile":"development-profiles-v1",
        "profiles":{"first":profile("provider-one"),"second":profile("provider-two")},
        "coordinator":"first","worker_default":"first","roles":{
            "coordinator":{"default":"first","allowed":["first","second"]},
            "implementation":{"default":"first","allowed":["first","second"]},
            "review":{"default":"first","allowed":["first","second"]}},
        "fallback":{"coordinator":[],"implementation":[],"review":[]},
        "concurrency":2,"dependency_network":false,"main_policy":"checked-fast-forward","quality_preference":""})
}

#[test]
fn shared_models_can_save_select_and_reopen_without_a_core_provider_or_model_entry() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join(".waywr/state");
    fs::create_dir_all(&state).unwrap();
    let key = "a".repeat(32);
    write(&state.join("state-contract.json"), &json!({"schema_version":1,"state_schema":9,
        "execution_host":"systemd-user-v1","project_key":key}), true).unwrap();
    write(&root.path().join(".waywr/coordinator.json"), &preferences(), true).unwrap();
    let session = Session::open(root.path(), &state, &key, "shared-models")
        .expect("A shared provider profile must open without a compiled provider/model allowlist");
    let mut edited = preferences();
    edited["worker_default"] = json!("second");
    edited["roles"]["implementation"]["default"] = json!("second");
    let saved = providers::save(&session, &json!({"command_id":"save-model-choice","action":"providers.save",
        "payload":{"expected_revision":1,"preferences":edited}})).unwrap();
    assert_eq!(saved["status"], "accepted");
    let (prefs, revision) = providers::preferences(&session).unwrap();
    let policy = json!({"routing":prefs.allowed_policy().unwrap(),"dependency_network":false});
    let selected = prefs.resolve("implementation", &Value::Null, None, &policy, revision, "ordinary task").unwrap();
    assert_eq!(selected["profile"]["provider"], "provider-two");
    assert_eq!(selected["profile"]["options"]["temperature"], 0.2);
    drop(session);
    let reopened = Session::open(root.path(), &state, &key, "shared-models").unwrap();
    assert_eq!(providers::preferences(&reopened).unwrap().0.worker_default, "second");
}

#[test]
fn shared_models_freeze_runtime_and_configuration_without_granting_a_changed_endpoint() {
    let root=tempfile::tempdir().unwrap();let script=root.path().join("runtime.mjs");fs::write(&script,"// metadata fixture only").unwrap();
    let config=root.path().join("models.json");
    write(&config,&json!({"providers":[{"id":"new.provider/id","api":"custom","base_url":"http://127.0.0.1:1"}]}),true).unwrap();
    let mut transport=json!({"entry":script,"config":config});
    transport["destination"]=waywr_backend::model_runtime::destination("new.provider/id",&transport).unwrap();
    let profile=json!({"provider":"new.provider/id","binary":waywr_backend::model_runtime::node().unwrap(),"model":"new model identifier",
        "transport":transport,"options":{}});
    let first=providers::with_client(profile.clone());waywr_backend::model_runtime::preflight(&first).unwrap();
    fs::write(&script,"// updated runtime fixture").unwrap();
    assert!(waywr_backend::model_runtime::preflight(&first).unwrap_err().contains("changed"));
    let next=providers::with_client(profile.clone());waywr_backend::model_runtime::preflight(&next).unwrap();
    write(&config,&json!({"providers":[{"id":"new.provider/id","api":"custom","base_url":"http://127.0.0.1:2"}]}),false).unwrap();
    assert!(waywr_backend::model_runtime::preflight(&next).is_err());
    assert!(providers::with_client(profile)["client_error"].as_str().unwrap().contains("destination changed"));
}

// Real runner/SDK loop. Host permit is an isolated fixture, not a systemd proof.
#[test]
fn shared_models_runner_executes_two_protocols_with_tools_and_retains_usage() {
    use std::{io::{BufRead,BufReader},path::Path,process::{Command,Stdio},thread,time::{Duration,Instant}};
    use waywr_backend::{host_attempt::Attempt,operation,systemd_host::HostStatus,execution::read};
    for api in ["openai-completions","anthropic-messages"] {
        let root=tempfile::tempdir().unwrap();
        let project=root.path().join("project");fs::create_dir(&project).unwrap();
        let git=|args:&[&str]|{let output=Command::new("git").args(["-c","user.name=Fixture","-c","user.email=fixture@example.invalid","-c","core.hooksPath=/dev/null"])
            .current_dir(&project).args(args).output().unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));};
        git(&["init","--quiet"]);git(&["commit","--quiet","--allow-empty","-m","Fixture baseline"]);
        let tree=root.path().join("worker");git(&["worktree","add","--quiet","-b","waywr/request/tasks/job/work",tree.to_str().unwrap(),"HEAD"]);
        let mut server=Command::new("node").arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../model-runtime/tests/http-fixture.mjs"))
            .arg(api).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        let mut endpoint=String::new();BufReader::new(server.stdout.take().unwrap()).read_line(&mut endpoint).unwrap();
        let config=root.path().join("models.json");
        let provider=format!("custom-{api}");
        write(&config,&json!({"providers":[{"id":provider,"api":api,"base_url":endpoint.trim(),"models":[{"id":"model-outside-core","context_window":32768}]}]}),true).unwrap();
        let state=root.path().join("state");let directory=state.join("session/development/request/work");fs::create_dir_all(&directory).unwrap();
        write(&state.join("supervision/owner.json"),&json!({"generation":7}),true).unwrap();
        write(&state.join("session/dispatch-gate.json"),&json!({"phase":"running","owner_generation":7}),true).unwrap();
        let entry=Path::new(env!("CARGO_MANIFEST_DIR")).join("../model-runtime/dist/bridge.mjs").canonicalize().unwrap();
        let mut transport=json!({"entry":entry,"config":config});
        transport["destination"]=waywr_backend::model_runtime::destination(&provider,&transport).unwrap();
        let execution=providers::with_client(json!({"profile":"development-profiles-v1","provider":provider,
            "binary":waywr_backend::model_runtime::node().unwrap(),"model":"model-outside-core","effort":"provider_default",
            "options":{"temperature":0.2},"transport":transport,"context_bytes":98304}));
        let packet=json!({"schema_version":1,"development":true,"operation_kind":"development-worker","operation_id":"work",
            "run_id":"request","stage":"work","role":"implementation","provider":provider,"model":"model-outside-core",
            "worktree":tree,"git_common_dir":project.join(".git"),"assignment":{"name":"job","branch":"waywr/request/tasks/job/work"},"execution":execution});
        let key="a".repeat(32);operation::prepare_host(&state,&directory,&packet,&key).unwrap();
        let a=Attempt::create(&state,&directory,&key,Path::new(env!("CARGO_BIN_EXE_waywr-runner")),"fixture",7).unwrap();
        a.mark_maybe_sent().unwrap();
        let mut child=Command::new(env!("CARGO_BIN_EXE_waywr-runner")).args(["host-run","--operation-dir",directory.to_str().unwrap(),"--state-dir",state.to_str().unwrap(),"--attempt-id",&a.id])
            .env("INVOCATION_ID","b".repeat(32)).env("HOME",root.path()).env("XDG_CONFIG_HOME",root.path().join("config")).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        let deadline=Instant::now()+Duration::from_secs(15);
        while !a.directory.join("claim.json").exists(){assert!(Instant::now()<deadline);thread::sleep(Duration::from_millis(10));}
        let cgroup=fs::read_to_string(format!("/proc/{}/cgroup",child.id())).unwrap().lines().find_map(|s|s.strip_prefix("0::").map(str::to_owned)).unwrap();
        a.permit(&HostStatus{name:a.unit.clone(),active:"active".into(),sub:"running".into(),invocation_id:"b".repeat(32),control_group:cgroup,
            main_pid:child.id(),result:"success".into(),restart:"no".into(),exit_type:"cgroup".into(),remain_after_exit:true,collect_mode:"inactive".into()},7).unwrap();
        while child.try_wait().unwrap().is_none(){if Instant::now()>deadline {let _=child.kill();panic!("Runner failed to settle")};thread::sleep(Duration::from_millis(10));}
        drop(server.stdin.take());server.wait().unwrap();
        let receipt=a.validated_receipt().unwrap().expect("Settled runner receipt");
        assert_eq!(receipt["status"],"completed","{}",fs::read_to_string(a.directory.join("runner-diagnostic.log")).unwrap_or_default());
        assert_eq!(fs::read_to_string(tree.join("delivered.txt")).unwrap(),"Created by the actual SDK tool loop");
        let runtime=read(&a.directory.join("runtime-result.json")).unwrap();assert!(runtime["usage"]["input"].as_u64().unwrap()>0);
        assert!(read(&a.directory.join("runtime-progress.json")).unwrap()["usage"].is_object());
    }
}
