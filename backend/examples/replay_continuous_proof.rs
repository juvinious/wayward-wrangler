//! Offline decoder replay of retained dummy-credential/no-egress CLI records.
//! Reads evidence only; never starts a provider, runner, service or model call.
use serde_json::{Value, json};
use std::{env, fs, io::Write, path::PathBuf};
use waywr_backend::{claude_chat, continuous_adapter};
fn main() {
    let args = env::args().collect::<Vec<_>>();
    assert!(
        args.len() == 3 || args.len() == 4,
        "Use PROOF_DIRECTORY NEW_SUMMARY_FILE [NEW_AUTHORITY_DIRECTORY]"
    );
    let root = PathBuf::from(&args[1]);
    let verification: Value = serde_json::from_slice(
        &fs::read(root.join(if root.join("verification.json").exists() {
            "verification.json"
        } else {
            "summary.json"
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(verification["binary_sha256"], claude_chat::BINARY_SHA);
    let mut results = Vec::new();
    for case in verification["cases"].as_array().unwrap() {
        let name = case["case"].as_str().unwrap();
        assert!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
        let record: Value =
            serde_json::from_slice(&fs::read(root.join(format!("{name}.json"))).unwrap()).unwrap();
        let events = record["events"].as_array().unwrap();
        let proof = continuous_adapter::completion(
            events,
            "claude-sonnet-5-5",
            record["returncode"].as_i64().map(|v| v as i32),
        );
        let expected = matches!(
            name,
            "valid"
                | "question"
                | "proposal"
                | "suggestion"
                | "valid_claude"
                | "valid_user_agents"
                | "mention"
        );
        assert_eq!(
            proof["action_valid"], expected,
            "Actual decoder admission for {name}: {proof}"
        );
        let payload_valid = continuous_adapter::raw_input(events)
            .and_then(|s| waywr_backend::scope_contract::parse(&s))
            .is_ok();
        assert_eq!(
            payload_valid,
            expected || name.starts_with("control"),
            "Payload shape only, not admission: {name}"
        );
        assert_eq!(case["request_count"], 1, "{name}");
        let expected_category = if expected {
            "completed"
        } else if name.starts_with("control")
            || matches!(
                name,
                "Read"
                    | "Write"
                    | "Edit"
                    | "Glob"
                    | "Grep"
                    | "Bash"
                    | "Agent"
                    | "SendMessage"
                    | "WebFetch"
                    | "WebSearch"
                    | "UnknownTool"
                    | "MCP"
                    | "Skill"
                    | "InheritedSkill"
            )
        {
            "capability"
        } else if name.starts_with("control")
            || matches!(
                name,
                "error" | "overloaded" | "unauthorized" | "rate" | "truncated" | "cancel"
            )
        {
            "unavailable"
        } else {
            "format"
        };
        assert_eq!(
            proof["category"], expected_category,
            "Wrong refusal reason for {name}: {proof}"
        );
        if expected && args.len() == 4 {
            authority(&PathBuf::from(&args[3]).join(name), &proof["result"], name);
        }
        results.push(json!({"case":name,"action_valid":proof["action_valid"],"category":proof["category"],"known_completion":proof["known_completion"],"original_request_count":case["request_count"],"payload_valid_separately":payload_valid}));
    }
    let admitted = results.iter().filter(|r| r["action_valid"] == true).count();
    let output = json!({"records":results.len(),"decoder_admitted":admitted,"new_calls":0,"authority_checked":args.len()==4,"results":results});
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(&output).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    println!(
        "{} unchanged real-client records: {} decoder admissions; zero new calls",
        results.len(),
        admitted
    );
}

fn authority(root: &std::path::Path, result: &Value, name: &str) {
    use waywr_backend::{
        codec::canonical_sha256 as hash, execution::write, session_store::Session,
    };
    assert!(!root.exists());
    let project = root.join("project");
    let state = project.join(".wayward-wrangler/state");
    fs::create_dir_all(&state).unwrap();
    write(&state.join("state-contract.json"),&json!({"schema_version":1,"state_schema":7,"execution_host":"systemd-user-v1","project_key":"replay-isolated"}),true).unwrap();
    let profile = json!({"schema_version":1,"profile":"claude-proposal-v2","mode":"mock-cli","provider":"claude","model":"claude-sonnet-5-5","effort":"medium","call_limit":6,"binary":PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/continuous-coordinator/mock-cli.py"),"auth_home":project,"auth_config_dir":project});
    write(
        &project.join(".wayward-wrangler/coordinator.json"),
        &profile,
        true,
    )
    .unwrap();
    let s = Session::open(
        &project,
        &state,
        "replay-isolated",
        "offline-decoder-replay",
    )
    .unwrap();
    let ready = s.chat_readiness().unwrap();
    assert_eq!(s.conversation_select_coordinator(&json!({"command_id":"mock-only-grant","action":"select_coordinator","expected_generation":1,"payload":{"authorize":true,"profile_sha256":ready["profile_sha256"]}})).unwrap()["status"],"accepted");
    assert_eq!(s.conversation_submit(&json!({"command_id":"send","action":"submit_message","payload":{"text":"Discuss a weather page.","subject_id":"discussion"}})).unwrap()["status"],"accepted");
    let next = s.conversation_next_queued().unwrap().unwrap();
    let packet = s
        .scope_packet(&next, &profile, &json!(hash(&profile).unwrap()))
        .unwrap();
    let turn = next["turn_id"].as_str().unwrap();
    let digest = hash(&packet).unwrap();
    write(
        &state
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
    let outcome = s.conversation_apply(turn, &digest, result);
    if name == "suggestion" {
        assert!(
            outcome
                .unwrap_err()
                .contains("Suggestion source outside frozen messages")
        );
        assert!(
            s.conversation_snapshot().unwrap()["actions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    } else {
        let saved = outcome.unwrap();
        assert_eq!(saved["status"], "accepted");
        assert_eq!(
            s.conversation_apply(turn, &digest, result).unwrap()["replayed"],
            true
        );
        if name == "proposal" {
            assert!(saved["scope"]["base"]["revision"].as_u64().is_some());
            assert_eq!(
                s.scope_detail(
                    s.snapshot().unwrap()["tasks"][0]["task_id"]
                        .as_str()
                        .unwrap()
                )
                .unwrap()
                .unwrap()["authorized"],
                false
            );
        }
    }
    assert_eq!(
        s.conversation_snapshot().unwrap()["budgets"]["conversation"]["charged"],
        1
    );
    write(&root.join("admission.json"),&json!({"mode":"mock-only authority, no process launch","unchanged_result":true,"accepted":name!="suggestion","reason":if name=="suggestion"{"source outside frozen messages"}else{"valid current dependencies"},"duplicate_application_once":name!="suggestion","workers":0,"calls_launched":0}),true).unwrap();
}
