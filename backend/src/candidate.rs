//! Candidate identity, human validation and checkout ownership over existing records.
use crate::{
    codec::canonical_sha256,
    conversation_store::{commit, encode, event, prior},
    development::{authority, git},
    execution::FileLock,
    session_store::Session,
};
use rusqlite::params;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .ok_or_else(|| format!("Missing candidate {k}"))
}
fn bump(w: &mut Value) {
    w["revision"] = json!(w["revision"].as_u64().unwrap_or(0) + 1)
}
fn save(s: &Session, w: &Value, kind: &str) -> Result<(), String> {
    s.development_save("workflow", text(w, "work_id")?, w, kind)
}
fn clean(path: &Path) -> Result<(), String> {
    if !git(path, &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty() {
        return Err(format!(
            "Checkout {} has local edits; preserved. Commit intended candidate edits and choose Include committed changes, or finish validation without changing source",
            path.display()
        ));
    }
    Ok(())
}
pub fn instruction() -> &'static str {
    "Ordinary project workflow v1: project_workflow freezes baseline, target, completion, human validation and retention. Do not invent a mandatory human gate when human_validation is false. When checks/reviews support a usable candidate, choose action candidate with delivery instructions derived from its actual README/manifests: environment, prerequisites, build/run/inspect commands, expected result and shutdown. Suggestions are not execution receipts. The service presents and binds feedback/acceptance to that commit, handles checkout ownership, policy completion and safe cleanup. Candidate readiness is not delivery. A user acceptance report is separate from check receipts. While validation owns the governing branch, no integration/promotion writes are allowed. Human feedback is context within existing scope, never scope expansion. Use refresh_target to integrate a newly observed target commit before rechecking when the target advanced. Complete automatically after agreed checks/reviews/obligations and required candidate validation; no second approval for unchanged scope."
}
pub fn context_instruction(packet: &Value) -> &'static str {
    if packet["ordinary_project_contract"] != 1 {
        return "";
    }
    if packet["operation_kind"] == "conversation" {
        return "For ordinary-project candidates, give the presented candidate's project instructions and exact branch/commit, distinguish readiness from delivery, and suggest Check out candidate for a normal checkout instead of a private path. Normal candidate feedback can use the existing source-bound propose_steering action when selected_context.interaction.candidate names the exact presented candidate and the human clearly reports an in-scope change; the application records it as candidate feedback and releases validation safely. Questions/hypotheticals remain action:null. Never accept a candidate on the user's behalf; direct human acceptance is application-routed. Ending validation releases checkout ownership without satisfying any required acceptance.";
    }
    if packet["role"] == "coordinator" {
        instruction()
    } else {
        "Project setup/validation/retention authority is in policy.project_workflow. Candidate instructions are suggestions; exact reviews and execution receipts remain distinct. Service owns all checkout handoff, cleanup and completion writes."
    }
}
pub fn delivery_schema() -> Value {
    let fields = [
        "environment",
        "prerequisites",
        "build",
        "run",
        "inspect",
        "expected",
        "shutdown",
        "notes",
    ];
    let mut p = serde_json::Map::new();
    for f in fields {
        p.insert(
            f.into(),
            if f == "environment" {
                json!({"type":"string"})
            } else {
                json!({"type":"array","items":{"type":"string"}})
            },
        );
    }
    json!({"type":"object","properties":p,"required":fields,"additionalProperties":false})
}
pub fn prepare(
    s: &Session,
    w: &mut Value,
    delivery: &Value,
    op: &str,
) -> Result<Option<Value>, String> {
    if !w["policy"]["project_workflow"].is_object() {
        return Err("Candidate lifecycle requires the request's approved project policy".into());
    }
    if w["checks"]["commit"] != w["head"]
        || w["checks"]["result"]["passed"] != true
        || w["checks"]["result"]["checks"]
            .as_array()
            .is_none_or(|a| a.is_empty() || a.iter().any(|c| c["success"] != true))
    {
        return Err("Candidate needs successful exact combined check receipts".into());
    }
    let all = s
        .development_records("unit")?
        .into_iter()
        .filter(|u| u["work_id"] == w["work_id"])
        .collect::<Vec<_>>();
    for unit in all
        .iter()
        .filter(|u| matches!(u["status"].as_str(), Some("integrated" | "accepted")))
    {
        crate::request_routing::require_review(&w["policy"], unit, &all)?;
    }
    let check_hash = hash(&w["checks"])?;
    if w["candidate"]["commit"] == w["head"]
        && w["candidate"]["generation"] == w["requirement_generation"]
        && w["candidate"]["checks_sha256"] == check_hash
        && w["candidate"]["state"] != "changes-requested"
    {
        return Ok(None);
    }
    if delivery.as_object().is_none_or(|v| v.is_empty()) {
        return Err("Candidate requires project-specific delivery instructions, not an internal worktree path alone".into());
    }
    for field in [
        "environment",
        "prerequisites",
        "build",
        "run",
        "inspect",
        "expected",
        "shutdown",
        "notes",
    ] {
        if field == "environment" {
            if delivery[field].as_str().is_none_or(|v| v.len() > 4000) {
                return Err("Candidate environment must be a concise description".into());
            }
        } else if delivery[field].as_array().is_none_or(|a| {
            a.len() > 32 || a.iter().any(|v| v.as_str().is_none_or(|s| s.len() > 4000))
        }) {
            return Err(format!("Invalid candidate delivery {field}"));
        }
    }
    let sequence = w["candidate_sequence"].as_u64().unwrap_or(0) + 1;
    let id = format!(
        "candidate-{}",
        &hash(&json!([
            w["work_id"],
            w["scope_base"],
            w["requirement_generation"],
            w["head"],
            check_hash,
            delivery,
            sequence
        ]))?[..24]
    );
    let record = json!({"version":1,"candidate_id":id,"work_id":w["work_id"],"request_id":w["request_id"],"generation":w["requirement_generation"],"scope_base":w["scope_base"],"commit":w["head"],"branch":w["result_branch"].as_str().or(w["integration_branch"].as_str()),"governing_branch":w["integration_branch"],"checkout":s.project,"service_checkout":w["integration_worktree"],"target":w["main_branch"],"completion":w["policy"]["project_workflow"]["completion"],"human_validation":w["policy"]["project_workflow"]["human_validation"],"checks_sha256":check_hash,"checks":w["checks"],"reviews":all.iter().filter(|u|u["role"]=="review"&&u["status"]=="reviewed").map(|u|json!({"unit_id":u["unit_id"],"operation_id":u["operation_id"],"effective_requires_changes":crate::review_findings::effective_requires_changes(u,&all)})).collect::<Vec<_>>(),"open_obligations":w["obligations"].as_array().into_iter().flatten().filter(|o|o["status"]!="resolved").cloned().collect::<Vec<_>>(),"delivery":delivery,"instruction_provenance":"Coordinator suggestions at this commit, not execution receipts","source_operation":op,"remaining":"Target has not advanced. Required human validation, unresolved obligations or policy completion remain visible separately."});
    w["candidate_sequence"] = json!(sequence);
    w["candidate"] = json!({"candidate_id":id,"commit":w["head"],"generation":w["requirement_generation"],"checks_sha256":check_hash,"state":"ready","presented":false,"validation":null});
    if record["human_validation"] == true {
        w["candidate"]["requires_handoff"] = json!(true);
    }
    Ok(Some(record))
}
pub fn accepted(w: &Value) -> bool {
    w["candidate"]["validation"]["accepted"] == true
        && w["candidate"]["validation"]["commit"] == w["head"]
        && w["candidate"]["generation"] == w["requirement_generation"]
        && w["candidate"]["state"] == "accepted"
}
pub fn writes_held(w: &Value) -> bool {
    w["checkout_handoff"].is_object()
        && !matches!(
            w["checkout_handoff"]["state"].as_str(),
            Some("service-owned" | "released")
        )
}
pub fn start_handoff(s: &Session, w: &mut Value) -> Result<bool, String> {
    if w["candidate"]["requires_handoff"] != true || accepted(w) {
        return Ok(writes_held(w));
    }
    if !w["pending"].is_null() {
        return Ok(true);
    }
    let _lock = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
    if !authority(s, w)? {
        return Err("Candidate handoff authority changed".into());
    }
    let directory = text(w, "integration_worktree")?.to_owned();
    let tree = Path::new(&directory);
    let branch = text(w, "integration_branch")?.to_owned();
    let commit = text(&w["candidate"], "commit")?.to_owned();
    if crate::project_workflow::ref_head(&s.project, &branch)? != commit
        || git(tree, &["rev-parse", "HEAD"])? != commit
    {
        return Err(
            "Candidate branch/checkout moved; preserve and reconcile before handoff".into(),
        );
    }
    clean(tree)?;
    if w["checkout_handoff"].is_null() || w["checkout_handoff"]["state"] == "service-owned" {
        w["checkout_handoff"] = json!({"candidate_id":w["candidate"]["candidate_id"],"commit":commit,"branch":branch,"state":"detach-intent","service_checkout":tree,"owner":null});
        save(s, w, "development.candidate_handoff_intent")?;
    }
    let actual = git(tree, &["branch", "--show-current"])?;
    if actual == branch {
        git(tree, &["checkout", "--detach", &commit])?;
    } else if !actual.is_empty() {
        return Err("Integration checkout now owns another branch; preserved".into());
    }
    w["checkout_handoff"]["state"] = json!("available");
    w["candidate"]["requires_handoff"] = json!(false);
    w["candidate"]["state"] = json!("validating");
    bump(w);
    save(s, w, "development.candidate_available")?;
    Ok(true)
}
fn release(s: &Session, w: &mut Value, include: Option<&str>) -> Result<(), String> {
    let branch = text(w, "integration_branch")?.to_owned();
    let commit = text(&w["candidate"], "commit")?.to_owned();
    let expected = include.unwrap_or(&commit);
    if crate::project_workflow::ref_head(&s.project, &branch)? != expected {
        return Err("Governing ref changed from the candidate/action input; preserve and explicitly include committed changes".into());
    }
    if let Some(owner) = crate::project_workflow::target_checkout(&s.project, &branch)? {
        if owner != PathBuf::from(text(w, "integration_worktree")?) {
            clean(&owner)?;
            if git(&owner, &["rev-parse", "HEAD"])? != expected {
                return Err("Validation checkout moved; no silent reset".into());
            }
            git(&owner, &["checkout", "--detach", expected])?;
        }
    }
    let service = Path::new(text(w, "integration_worktree")?);
    clean(service)?;
    let service_branch = git(service, &["branch", "--show-current"])?;
    if service_branch.is_empty() {
        git(service, &["checkout", &branch])?;
    } else if service_branch != branch {
        return Err("Service checkout ownership changed; preserved".into());
    }
    w["checkout_handoff"] = json!({"candidate_id":w["candidate"]["candidate_id"],"commit":expected,"state":"service-owned","prior_user_checkout":w["checkout_handoff"]["owner"],"note":"Validation ended; user checkout left detached at its exact input, never restored/reset"});
    Ok(())
}
pub fn command(s: &Session, body: &Value) -> Result<Value, String> {
    if let Some(writer) = &s.workflow.writer {
        let b = body.clone();
        return writer.call(move |s| command(s, &b));
    }
    let action = text(body, "action")?;
    if !matches!(
        action,
        "candidate.present"
            | "candidate.checkout"
            | "candidate.accept"
            | "candidate.feedback"
            | "candidate.include"
            | "candidate.end_validation"
    ) {
        return Err("Unknown candidate action".into());
    }
    crate::conversation_store::valid_command(body, action).map_err(|e| e.to_string())?;
    {
        let mut c = s.workflow.connection()?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        if let Some(out) = prior(&tx, body)? {
            return Ok(out);
        }
    }
    let p = &body["payload"];
    let mut w = s
        .development_record("workflow", text(p, "work_id")?)?
        .ok_or("Request missing")?;
    if w["candidate"]["candidate_id"] != p["candidate_id"]
        || w["candidate"]["commit"] != p["commit"]
        || w["candidate"]["generation"] != w["requirement_generation"]
        || !authority(s, &w)?
    {
        return Err("Feedback/acceptance targets an obsolete candidate; review the current card. Original input is retained".into());
    }
    if action != "candidate.present"
        && matches!(
            w["status"].as_str(),
            Some("delivered" | "cancelled" | "cancelling")
        )
    {
        return Err("Candidate belongs to a closed/cancelling request; discussion remains available without reopening work".into());
    }
    let _card = s
        .development_record("candidate", text(p, "candidate_id")?)?
        .ok_or("Candidate record missing")?;
    let _lock = FileLock::exclusive(&s.workflow.state.join("development-git.lock"))?;
    if action != "candidate.present" && w["candidate"]["presented"] != true {
        return Err("Present this exact candidate before validation actions".into());
    }
    let command_id = text(body, "command_id")?;
    let intent = if let Some(saved) = s.development_record("candidate_action", command_id)? {
        if saved["payload"] != *p {
            return Err("Candidate action identity reused with different input".into());
        }
        saved
    } else {
        let mut value = json!({"work_id":w["work_id"],"request_id":w["request_id"],"action":action,"candidate_id":p["candidate_id"],"payload":p,"status":"intent"});
        if action == "candidate.include" {
            value["included_commit"] = json!(crate::project_workflow::ref_head(
                &s.project,
                text(&w, "integration_branch")?
            )?);
        }
        s.development_save(
            "candidate_action",
            command_id,
            &value,
            "development.candidate_action_intent",
        )?;
        value
    };
    let message = p["text"].as_str().unwrap_or("");
    match action {
        "candidate.present" => w["candidate"]["presented"] = json!(true),
        "candidate.checkout" => {
            if !w["pending"].is_null() {
                return Err("Candidate checkout waits for the current coordinator/check turn to settle; no work interrupted".into());
            }
            if crate::project_workflow::ref_head(&s.project, text(&w, "integration_branch")?)?
                != p["commit"]
            {
                return Err("Candidate ref changed; explicitly include committed validation changes before checkout".into());
            }
            if w["checkout_handoff"].is_null() || w["checkout_handoff"]["state"] == "service-owned"
            {
                w["candidate"]["requires_handoff"] = json!(true);
                drop(_lock);
                start_handoff(s, &mut w)?;
                return command(s, body);
            }
            if !matches!(
                w["checkout_handoff"]["state"].as_str(),
                Some("available" | "user-owned" | "checkout-intent")
            ) {
                return Err("Candidate checkout is not available; finish/reconcile the current write handoff".into());
            }
            let selected = p["directory"]
                .as_str()
                .map(PathBuf::from)
                .unwrap_or(s.project.clone())
                .canonicalize()
                .map_err(|e| e.to_string())?;
            let listing = git(&s.project, &["worktree", "list", "--porcelain"])?;
            if !listing
                .lines()
                .any(|l| l == format!("worktree {}", selected.display()))
            {
                return Err("Choose a registered checkout of this project; no new or unrelated worktree is adopted".into());
            }
            clean(&selected)?;
            let branch = text(&w, "integration_branch")?.to_owned();
            if git(&selected, &["branch", "--show-current"])? != branch {
                if crate::project_workflow::target_checkout(&s.project, &branch)?.is_some() {
                    return Err(
                        "Candidate is owned by another checkout; Inspect shows its owner".into(),
                    );
                }
                w["checkout_handoff"]["owner"] = json!(selected);
                w["checkout_handoff"]["prior_branch"] =
                    json!(git(&selected, &["branch", "--show-current"])?);
                w["checkout_handoff"]["state"] = json!("checkout-intent");
                save(s, &w, "development.candidate_checkout_intent")?;
                git(&selected, &["checkout", &branch])?;
            }
            w["checkout_handoff"]["owner"] = json!(selected);
            w["checkout_handoff"]["state"] = json!("user-owned");
        }
        "candidate.accept" | "candidate.feedback" | "candidate.end_validation" => {
            if action == "candidate.feedback"
                && (message.trim().is_empty() || message.len() > 32768)
            {
                return Err("Candidate feedback needs substantive bounded text".into());
            }
            release(s, &mut w, None)?;
            w["candidate"]["state"] = json!(if action == "candidate.accept" {
                "accepted"
            } else if action == "candidate.feedback" {
                "changes-requested"
            } else {
                "ready"
            });
            w["candidate"]["validation"] = json!({"candidate_id":p["candidate_id"],"commit":p["commit"],"accepted":action=="candidate.accept","text":message,"source_command":command_id,"label":"Human report; not an execution receipt"});
            if action == "candidate.accept" {
                let validation = w["candidate"]["validation"].clone();
                for o in w["obligations"].as_array_mut().into_iter().flatten() {
                    if o["name"] == "policy:human-validation" {
                        o["status"] = json!("resolved");
                        o["resolution"] = validation.clone();
                    }
                }
            }
            if action == "candidate.feedback" {
                if !w["answers"].is_array() {
                    w["answers"] = json!([])
                }
                w["answers"].as_array_mut().unwrap().push(json!({"candidate_id":p["candidate_id"],"commit":p["commit"],"text":message,"source_command":command_id,"authority":"Candidate feedback within existing scope; expansion requires proposal approval"}));
            }
            w["status"] = json!("running");
            w["last_context_sha256"] = Value::Null;
        }
        "candidate.include" => {
            let commit = text(&intent, "included_commit")?.to_owned();
            let old = text(&w["candidate"], "commit")?.to_owned();
            if commit == old {
                return Err("Commit intended validation edits before including them; dirty files stay preserved".into());
            }
            git(&s.project, &["merge-base", "--is-ancestor", &old, &commit])?;
            release(s, &mut w, Some(&commit))?;
            let uid = format!("{}-human-{}", text(&w, "work_id")?, &commit[..12]);
            let files = git(&s.project, &["diff", "--name-only", &old, &commit])?
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let u = json!({"schema_version":1,"unit_id":uid,"name":format!("human-{}",&commit[..12]),"work_id":w["work_id"],"request_id":w["request_id"],"requirement_base":w["scope_base"],"requirement_generation":w["requirement_generation"],"title":"Explicitly included human candidate changes","role":"implementation","resource_owner":"human","base_commit":old,"result_commit":commit,"paths":files,"status":"submitted","start_after":[],"accept_after":[],"submission":{"summary":"Human-provided committed input; independent review and combined checks required","remaining_obligations":[]},"input_command":command_id});
            s.development_save("unit", &uid, &u, "development.human_input_included")?;
            w["head"] = json!(commit);
            w["checks"] = Value::Null;
            w["candidate"]["state"] = json!("superseded");
            w["last_context_sha256"] = Value::Null;
            w["status"] = json!("running");
        }
        _ => unreachable!(),
    }
    bump(&mut w);
    let mut c = s.workflow.connection()?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let actual: Value = serde_json::from_str(
        &tx.query_row(
            "SELECT record_json FROM development_records WHERE kind='workflow' AND id=?1",
            [text(&w, "work_id")?],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if actual["status"] != w["status"]
        && matches!(
            actual["status"].as_str(),
            Some("delivered" | "cancelled" | "cancelling")
        )
    {
        return Err("Request closed during candidate action; retained intent must be reconciled without reopening it".into());
    }
    if actual["policy"] != w["policy"]
        || actual["requirement_generation"] != w["requirement_generation"]
    {
        return Err(
            "Candidate authority changed during action; Git evidence retained for reconciliation"
                .into(),
        );
    }
    tx.execute(
        "UPDATE development_records SET record_json=?1 WHERE kind='workflow' AND id=?2",
        params![encode(&w)?, text(&w, "work_id")?],
    )
    .map_err(|e| e.to_string())?;
    let out = json!({"status":"accepted","work_id":w["work_id"],"candidate_id":p["candidate_id"],"candidate_state":w["candidate"]["state"],"checkout":w["checkout_handoff"],"execution_authorized":false});
    let mut done = intent;
    done["status"] = json!("recorded");
    done["result"] = out.clone();
    tx.execute(
        "UPDATE development_records SET record_json=?1 WHERE kind='candidate_action' AND id=?2",
        params![encode(&done)?, command_id],
    )
    .map_err(|e| e.to_string())?;
    let (seq, _) = event(
        &tx,
        if action == "candidate.present" {
            "development.candidate_presented"
        } else {
            "development.candidate_feedback"
        },
        w["work_id"].as_str(),
        &json!({"work_id":w["work_id"],"request_id":w["request_id"],"candidate_id":p["candidate_id"],"commit":p["commit"],"action":action,"text":message,"state":w["candidate"]["state"],"checkout":w["checkout_handoff"]}),
    )?;
    if let Some(r) = w["request_id"].as_str() {
        crate::request_history::tag(&tx, seq, r, "user / exact candidate")?;
    }
    commit(&tx, body, &out)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(out)
}
