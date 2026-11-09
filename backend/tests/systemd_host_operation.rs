//! Isolated, test-owned D-Bus host-to-runner check. Explicitly ignored so
//! normal offline tests never create a user service.
use serde_json::json;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use waywr_backend::{
    codec::canonical_sha256,
    execution::{read, write},
    host_attempt::Attempt,
    systemd_host::UserManager,
    output_capture,
};

struct OwnedUnit(String);
impl Drop for OwnedUnit {
    fn drop(&mut self) {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", &self.0])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}
#[tokio::test]
#[ignore = "creates one uniquely named, test-owned transient systemd user unit"]
async fn fake_host_runner_claim_permit_receipt_without_herdr() {
    let ephemeral = if std::env::var_os("WAYWR_HOST_TEST_EVIDENCE").is_some() {
        None
    } else {
        Some(tempfile::tempdir().unwrap())
    };
    let root = if let Some(parent) = std::env::var_os("WAYWR_HOST_TEST_EVIDENCE") {
        let parent = std::path::PathBuf::from(parent);
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join(waywr_backend::execution::random_token().unwrap());
        fs::create_dir(&path).unwrap();
        path
    } else {
        ephemeral.as_ref().unwrap().path().to_path_buf()
    };
    let state = root.join("state");
    let operation = state.join("session/coordinator/t-test/call-v1");
    fs::create_dir_all(&operation).unwrap();
    let packet = json!({"schema_version":1,"operation_kind":"coordinator",
        "operation_id":"op-test","run_id":"test","stage":"coordinator",
        "role":"coordinator","provider":"fake","model":"deterministic",
        "task_id":"t-test","task_version":1,"message_sha256":"test-message",
        "message":"Draft a small specification","seconds":0.1});
    write(&operation.join("packet.json"), &packet, true).unwrap();
    write(
        &operation.join("operation.json"),
        &json!({"schema_version":2,
        "operation_id":"op-test","packet_sha256":canonical_sha256(&packet).unwrap(),
            "project_key":"a".repeat(32),
        "provider":"fake","run_id":"test","stage":"coordinator","role":"coordinator",
        "model":"deterministic"}),
        true,
    )
    .unwrap();
    write(
        &state.join("supervision/owner.json"),
        &json!({"generation":1}),
        false,
    )
    .unwrap();
    write(
        &state.join("session/dispatch-gate.json"),
        &json!({"phase":"running","owner_generation":1}),
        false,
    )
    .unwrap();
    let runner = Path::new(env!("CARGO_BIN_EXE_waywr-runner"));
    let attempt = Attempt::create(
        &state,
        &operation,
        &"a".repeat(32),
        runner,
        "test-release",
        1,
    )
    .unwrap();
    let owned = OwnedUnit(attempt.unit.clone());
    attempt.start().await.unwrap();
    // Discard the start reply; reconcile only the immutable exact-name intent.
    let manager = UserManager::connect().await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let claim = loop {
        if attempt.directory.join("claim.json").exists() {
            break read(&attempt.directory.join("claim.json")).unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "runner did not claim: {:?}",
            fs::read_to_string(attempt.directory.join("runner-diagnostic.log"))
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let host = manager.status(&attempt.unit).await.unwrap();
    assert_eq!(host.main_pid, claim["runner"]["pid"]);
    loop {
        match attempt.permit(&host, 1) {
            Ok(()) => break,
            Err(error) if error.starts_with("Already owned:") && Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(50)).await
            }
            Err(error) => panic!("permit refused: {error}"),
        }
    }
    let receipt = loop {
        if attempt.directory.join("receipt.json").exists() {
            break read(&attempt.directory.join("receipt.json")).unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "runner did not publish receipt: {:?}",
            fs::read_to_string(attempt.directory.join("runner-diagnostic.log"))
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(
        receipt["status"],
        "completed",
        "receipt={receipt}; diagnostic={:?}",
        fs::read_to_string(attempt.directory.join("runner-diagnostic.log"))
    );
    assert_eq!(
        attempt.validated_receipt().unwrap().unwrap()["status"],
        "completed"
    );
    assert_eq!(receipt["invocation_id"], host.invocation_id);
    assert_eq!(
        read(&attempt.directory.join("invocations.json")).unwrap()["count"],
        1
    );
    assert_eq!(
        read(&operation.join("invocation-reservation.json")).unwrap()["attempt_id"],
        attempt.id
    );
    assert!(attempt.commit_spawn(&claim["runner"]).is_err());
    drop(owned);
}

#[tokio::test]
#[ignore = "creates one uniquely named, test-owned transient systemd user unit"]
async fn fake_host_runner_bounds_both_streams_and_retains_limit_evidence(){
    let parent=std::env::var_os("WAYWR_HOST_TEST_EVIDENCE").map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&parent).unwrap();
    let root=parent.join(waywr_backend::execution::random_token().unwrap());
    fs::create_dir(&root).unwrap();
    let state=root.join("state");let operation=state.join("session/coordinator/t-flood/call-v1");
    fs::create_dir_all(&operation).unwrap();
    let packet=json!({"schema_version":1,"operation_kind":"coordinator",
        "operation_id":"op-flood","run_id":"flood","stage":"coordinator",
        "role":"coordinator","provider":"fake","model":"deterministic",
        "task_id":"t-flood","task_version":1,"message_sha256":"test",
        "message":"Output flood","mode":"output-flood"});
    write(&operation.join("packet.json"),&packet,true).unwrap();
    write(&operation.join("operation.json"),&json!({"schema_version":2,
        "operation_id":"op-flood","packet_sha256":canonical_sha256(&packet).unwrap(),
        "project_key":"f".repeat(32),"provider":"fake","run_id":"flood",
        "stage":"coordinator","role":"coordinator","model":"deterministic"}),true).unwrap();
    write(&state.join("supervision/owner.json"),&json!({"generation":1}),false).unwrap();
    write(&state.join("session/dispatch-gate.json"),
        &json!({"phase":"running","owner_generation":1}),false).unwrap();
    let runner=Path::new(env!("CARGO_BIN_EXE_waywr-runner"));
    let attempt=Attempt::create(&state,&operation,&"f".repeat(32),runner,"test-release",1).unwrap();
    let _owned=OwnedUnit(attempt.unit.clone());
    attempt.start().await.unwrap();
    let manager=UserManager::connect().await.unwrap();
    let deadline=Instant::now()+Duration::from_secs(25);
    let host=loop{
        if attempt.directory.join("claim.json").exists(){
            let host=manager.status(&attempt.unit).await.unwrap();
            if host.main_pid==read(&attempt.directory.join("claim.json")).unwrap()["runner"]["pid"]{
                break host;
            }
        }
        assert!(Instant::now()<deadline,"fake flood runner did not claim");
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    loop{match attempt.permit(&host,1){Ok(())=>break,
        Err(error) if error.starts_with("Already owned:") && Instant::now()<deadline=>
            tokio::time::sleep(Duration::from_millis(50)).await,
        Err(error)=>panic!("flood permit refused: {error}")}}
    let receipt=loop{
        if attempt.directory.join("receipt.json").exists(){break read(&attempt.directory.join("receipt.json")).unwrap();}
        assert!(Instant::now()<deadline,"fake flood receipt missing: {:?}",
            fs::read_to_string(attempt.directory.join("runner-diagnostic.log")));
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(receipt["status"],"failed");
    let marker=read(&attempt.directory.join("output-limit.json")).unwrap();
    assert_eq!(marker["limit_bytes"],output_capture::TRANSCRIPT_LIMIT);
    assert!(marker["truncated"]==true);
    assert_eq!(fs::metadata(attempt.directory.join("transcript.log")).unwrap().len(),
        output_capture::TRANSCRIPT_LIMIT as u64);
    assert!(fs::metadata(attempt.directory.join("output.index")).unwrap().len()<=
        output_capture::INDEX_LIMIT*32);
    assert_eq!(read(&attempt.directory.join("invocations.json")).unwrap()["count"],1);
    let first=output_capture::page(&attempt.directory,&attempt.id,None,64*1024).unwrap();
    let mut streams=std::collections::BTreeSet::new();let mut cursor=first["cursor"].clone();
    for segment in first["segments"].as_array().unwrap(){streams.insert(segment["stream"].as_str().unwrap().to_owned());}
    for _ in 0..128{
        if streams.len()==2{break;}
        let page=output_capture::page(&attempt.directory,&attempt.id,Some(&cursor),64*1024).unwrap();
        for segment in page["segments"].as_array().unwrap(){streams.insert(segment["stream"].as_str().unwrap().to_owned());}
        cursor=page["cursor"].clone();
    }
    assert!(streams.contains("stdout") && streams.contains("stderr"),"{streams:?}");
    assert_eq!(attempt.validated_receipt().unwrap().unwrap()["status"],"failed");
}
