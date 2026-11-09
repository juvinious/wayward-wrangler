use serde_json::{Value, json};
use std::path::Path;
use waywr_backend::{
    codec::canonical_sha256,
    execution::{read, write},
    fake,
    workflow_store::Store,
};

fn envelope(run: &str) -> Value {
    json!({"schema_version":1,"workflow_version":1,"run_id":run,"thread_id":run,
        "brief":"Offline synthetic workflow fixture","instructions":[],
        "skills":{"draft":"fake","critique":"fake","revise":"fake"},
        "rules_sha256":"test","config_sha256":"test","target":{"socket":"/tmp/test-only.sock","workspace":"w-test"},
        "config":{"roles":{"author":{"provider":"fake","model":"test"},"critic":{"provider":"fake","model":"test"}},
            "fake":{"seconds":0},"review_limit":2,"max_invocations":5}})
}
fn new(run: &str) -> (tempfile::TempDir, Store) {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    store.create(&envelope(run)).unwrap();
    (root, store)
}
fn saved(store: &Store, run: &str, mut change: impl FnMut(&mut Value)) -> Value {
    let reservation = store.reserve_stage(run).unwrap();
    let packet = &reservation["packet"];
    let stage = reservation["stage"].as_str().unwrap();
    let directory = Path::new(reservation["directory"].as_str().unwrap());
    let mut result = fake::result(packet).unwrap();
    change(&mut result);
    let operation = read(&directory.join("operation.json")).unwrap();
    let runner = json!({"pid":0,"start":"synthetic","boot":"synthetic"});
    write(
        &directory.join("claim.json"),
        &json!({"runner":runner,"test_only":true}),
        true,
    )
    .unwrap();
    write(&directory.join("result.json"), &result, true).unwrap();
    write(
        &directory.join("receipt.json"),
        &json!({"schema_version":1,
        "operation_id":operation["operation_id"],"packet_sha256":operation["packet_sha256"],
        "launch_token":operation["launch_token"],"runner":runner,"status":"completed",
        "result_sha256":canonical_sha256(&result).unwrap(),"test_only":true}),
        true,
    )
    .unwrap();
    assert_eq!(stage, packet["stage"]);
    result
}

#[test]
fn incomplete_revision_retains_finding_and_exposes_coverage_to_second_critic() {
    let (_root, store) = new("incomplete");
    let run = "incomplete";
    saved(&store, run, |_| {});
    store.apply_saved(run, "draft").unwrap();
    saved(&store, run, |_| {});
    store.apply_saved(run, "critique-1").unwrap();
    let request = store.prepare_gate(run).unwrap();
    store
        .record_answer(
            run,
            request["request_id"].as_str().unwrap(),
            "answered",
            "7 days",
            None,
        )
        .unwrap();
    store.consume_answer(run).unwrap();
    saved(&store, run, |result| result["dispositions"] = json!([]));
    let candidate = store.apply_saved(run, "revision-1").unwrap();
    assert_eq!(candidate["state"]["findings"]["F1"]["status"], "open");
    assert_eq!(
        candidate["state"]["coverage_gaps"]["revision-1"][0]["id"],
        "F1"
    );
    let next = store.reserve_stage(run).unwrap();
    assert_eq!(next["stage"], "critique-2");
    assert_eq!(
        next["packet"]["revision_coverage"]["revision-1"][0]["id"],
        "F1"
    );
    assert_eq!(
        next["packet"]["open_findings"][0]["evidence"],
        "Draft does not specify retention."
    );
}

#[test]
fn record_only_answer_survives_restart_and_releases_once() {
    let (root,store)=new("held-answer");
    let run="held-answer";
    saved(&store,run,|_|{});store.apply_saved(run,"draft").unwrap();
    saved(&store,run,|_|{});store.apply_saved(run,"critique-1").unwrap();
    let gate=store.prepare_gate(run).unwrap();
    let request=gate["request_id"].as_str().unwrap();
    let original=store.record_answer_mode(run,request,"answered","7 days",None,true).unwrap();
    drop(store);
    let reopened=Store::open(root.path()).unwrap();
    let replay=reopened.record_answer_mode(run,request,"answered","7 days",None,true).unwrap();
    // The first return precedes JSON/SQLite round-tripping; its fractional
    // float timestamp can differ by one ULP. Compare every authoritative
    // decision field exactly and bound only that incidental representation.
    assert!((replay["time"].as_f64().unwrap()-original["time"].as_f64().unwrap()).abs()<0.001);
    let mut original_fields=original.clone();original_fields.as_object_mut().unwrap().remove("time");
    let mut replay_fields=replay.clone();replay_fields.as_object_mut().unwrap().remove("time");
    assert_eq!(replay_fields,original_fields);
    assert_eq!(reopened.consume_answer(run).unwrap()["status"],"awaiting-input");
    assert!(reopened.record_answer_mode(run,request,"answered","30 days",None,true).is_err());
    let first=reopened.release_record_only(run,request).unwrap();
    assert_eq!(reopened.release_record_only(run,request).unwrap(),first);
    assert_eq!(reopened.consume_answer(run).unwrap()["status"],"running");
    assert_eq!(reopened.release_record_only(run,request).unwrap(),first);
}

#[test]
fn invalid_result_stops_durably_without_a_new_effect() {
    let (_root, store) = new("invalid");
    let run = "invalid";
    saved(&store, run, |result| result["approval"] = "approved".into());
    assert!(
        store
            .apply_saved(run, "draft")
            .unwrap_err()
            .contains("OutputInvalid")
    );
    assert_eq!(store.snapshot(run).unwrap()["status"], "output-invalid");
    assert!(
        store
            .reserve_stage(run)
            .unwrap_err()
            .contains("not dispatchable")
    );
    assert_eq!(
        store.snapshot(run).unwrap()["state"]["completed"],
        json!([])
    );
}

#[test]
fn early_final_and_unknown_definition_refusal() {
    let (root, store) = new("early");
    let run = "early";
    saved(&store, run, |_| {});
    store.apply_saved(run, "draft").unwrap();
    saved(&store, run, |result| {
        result["needs_revision"] = false.into();
        result["questions"] = json!([]);
    });
    let state = store.apply_saved(run, "critique-1").unwrap();
    assert_eq!(state["state"]["next_stage"], "final");
    assert_eq!(store.prepare_gate(run).unwrap()["kind"], "final-review");
    assert!(store.reserve_stage(run).is_err());
    let db = rusqlite::Connection::open(root.path().join("state.sqlite")).unwrap();
    db.execute("UPDATE ww_workflows SET definition_hash='unrecognized'", [])
        .unwrap();
    assert!(
        store
            .snapshot(run)
            .unwrap_err()
            .contains("Unknown state/definition")
    );
    db.execute(
        "UPDATE ww_workflows SET definition_hash=?1, schema_version=3",
        [waywr_backend::workflow_policy::definition_hash()],
    )
    .unwrap();
    assert!(
        store
            .snapshot(run)
            .unwrap_err()
            .contains("Unknown state/definition")
    );
    assert!(waywr_backend::workflow_policy::next_stage("critique-3").is_err());
}

#[test]
fn published_result_can_be_applied_once_after_reentry() {
    let (root, store) = new("reentry");
    let run = "reentry";
    saved(&store, run, |_| {});
    let directory = root.path().join("runs/reentry");
    let packet = read(&directory.join("operations/draft/packet.json")).unwrap();
    let result = read(&directory.join("operations/draft/result.json")).unwrap();
    let text = result["content"].as_str().unwrap();
    // Simulate an immutable artifact publication before the graph/state commit.
    waywr_backend::execution::write_bytes(
        &directory.join("artifacts/draft.md"),
        text.as_bytes(),
        true,
    )
    .unwrap();
    assert_eq!(packet["stage"], "draft");
    let first = store.apply_saved(run, "draft").unwrap();
    let second = store.apply_saved(run, "draft").unwrap();
    assert_eq!(first, second);
    assert_eq!(first["state"]["completed"], json!(["draft"]));
    let db = rusqlite::Connection::open(root.path().join("state.sqlite")).unwrap();
    let applied: i64 = db
        .query_row(
            "SELECT count(*) FROM ww_effects WHERE applied_revision IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(applied, 1);
}

#[test]
fn unresolved_execution_and_frozen_budget_never_dispatch_again() {
    let (_root, store) = new("uncertain");
    let reservation = store.reserve_stage("uncertain").unwrap();
    let directory = Path::new(reservation["directory"].as_str().unwrap());
    let operation = read(&directory.join("operation.json")).unwrap();
    write(
        &directory.join("launch.json"),
        &json!({"status":"may-have-been-sent",
        "server":{"pid":0,"start":"test","boot":"test"},"target":operation["target"]}),
        true,
    )
    .unwrap();
    write(
        &directory.join("claim.json"),
        &json!({"runner":{"pid":0,"start":"test","boot":"test"},
        "herdr":{"pane_id":"test","tab_id":"test","workspace_id":"test"}}),
        true,
    )
    .unwrap();
    assert!(
        store
            .apply_saved("uncertain", "draft")
            .unwrap_err()
            .contains("No validated completion")
    );
    assert_eq!(store.snapshot("uncertain").unwrap()["status"], "uncertain");
    assert!(
        store
            .reserve_stage("uncertain")
            .unwrap_err()
            .contains("not dispatchable")
    );

    let root = tempfile::tempdir().unwrap();
    let limited = Store::open(root.path()).unwrap();
    let mut input = envelope("limited");
    input["config"]["max_invocations"] = 1.into();
    limited.create(&input).unwrap();
    saved(&limited, "limited", |_| {});
    limited.apply_saved("limited", "draft").unwrap();
    assert!(
        limited
            .reserve_stage("limited")
            .unwrap_err()
            .contains("limit")
    );
    assert_eq!(
        limited.snapshot("limited").unwrap()["state"]["completed"],
        json!(["draft"])
    );
}
