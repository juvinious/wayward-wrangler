//! Derived recovery explanations over existing workflow/unit/receipt records.
//! These records never grant authority, prove execution or replace a host attempt.
use crate::{
    codec::canonical_sha256,
    execution::{read, write},
    session_store::Session,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    Preparation,
    Integrity,
    HostObservation,
    UncertainExecution,
    SettledFailure,
    CheckFailure,
    GitEffect,
    Workspace,
    Authority,
    ProviderConfiguration,
    ProviderGrant,
    ProductDecision,
    Capacity,
    EquivalentWork,
    FindingResolution,
    ApplicationFault,
}
impl Cause {
    pub fn guidance(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Preparation => (
                "blocked",
                "Recheck preparation after its recorded inputs or release change",
                "preparation inputs changed; same scope/policy required",
            ),
            Self::Integrity => (
                "blocked",
                "Preserve the frozen packet; application integrity repair is required",
                "verified integrity repair; never rewrite or replay frozen work",
            ),
            Self::HostObservation => (
                "reconciling",
                "Observe the same saved attempt and validate its receipt or retirement",
                "positive receipt, live attempt observation or proven prelaunch retirement",
            ),
            Self::UncertainExecution => (
                "blocked",
                "Reconcile the exact execution records; dependent integration is fenced",
                "validated receipt or positive execution settlement; absence is insufficient",
            ),
            Self::SettledFailure | Self::CheckFailure => (
                "coordinating",
                "Coordinator will adapt using the actual settled failure",
                "new evidence, changed approach or useful follow-up",
            ),
            Self::GitEffect => (
                "reconciling",
                "Compare the saved intent with exact Git parents/head and clean worktree",
                "conclusive applied effect or clean unstarted intent; no speculative replay",
            ),
            Self::Workspace => (
                "blocked",
                "Preserve unexpected files and reconcile the owned workspace",
                "exact expected branch/head and clean owned worktree",
            ),
            Self::Authority | Self::ProviderGrant => (
                "needs-decision",
                "Use the existing exact scope/provider decision; unaffected work remains available",
                "required authority is recorded",
            ),
            Self::ProductDecision => (
                "needs-decision",
                "Answer the saved substantive question in chat",
                "saved answer to the current question",
            ),
            Self::Capacity => (
                "waiting",
                "Wait using the saved provider retry time; no narration call or silent switch",
                "persisted retry-after; configured approved fallback only",
            ),
            Self::ProviderConfiguration => (
                "blocked",
                "Selected provider configuration needs repair; work and approval are retained",
                "explicit supported configuration correction; no authentication inference",
            ),
            Self::FindingResolution => (
                "coordinating",
                "Selected reviewer can resolve the linked finding against exact evidence; no new whole review is required",
                "validated finding disposition or reviewed implementation repair",
            ),
            Self::EquivalentWork => (
                "coordinating",
                "Reassess the prior approach against meaningful evidence changes",
                "useful evidence, code, finding, check or execution change",
            ),
            Self::ApplicationFault => (
                "blocked",
                "Preserve evidence; this application fault needs a concrete repair",
                "positively repaired cause; generic continue cannot authorize replay",
            ),
        }
    }
}
pub fn hash(v: &Value) -> Result<String, String> {
    canonical_sha256(v).map_err(|e| e.to_string())
}
pub fn authority(w: &Value) -> Result<String, String> {
    hash(&json!([
        w["scope_base"],
        w["decision_revision"],
        w["requirement_generation"],
        w["policy"]
    ]))
}
pub fn retain_contract(s: &Session) -> Result<(), String> {
    write(
        &s.workflow.state.join("coordinator-recovery-v1.json"),
        &json!({"version":1,"contract":"recovery-v1"}),
        true,
    )
}
pub fn observe(
    w: &Value,
    cause: Cause,
    subject: &str,
    operation: &str,
    detail: &str,
    proof: Value,
) -> Result<Value, String> {
    let authority = authority(w)?;
    let identity = json!([cause, subject, operation, authority]);
    let (phase, next, wake) = cause.guidance();
    Ok(
        json!({"version":1,"recovery_id":hash(&identity)?,"cause":cause,"phase":phase,
        "subject_id":subject,"operation_id":operation,"authority_sha256":authority,
        "profile_sha256":w["profile"]["selection"]["profile_sha256"],
        "detail":detail,"source_evidence":proof,"source_evidence_sha256":hash(&proof)?,"diagnostic_sha256":hash(&json!(detail))?,"blocked_actions":["dependent dispatch or integration"],
        "next_action":next,"wake_condition":wake,"decision_needed":phase=="needs-decision",
        "can_continue":"Ordinary conversation and unaffected authorized assignments",
        "authority":"derived explanation; receipts, scope, policy and execution fences remain authoritative"}),
    )
}
pub fn resolved(record: &Value, proof: Value) -> Value {
    if !record.is_object() {
        return Value::Null;
    }
    let mut v = record.clone();
    v["phase"] = json!("resolved");
    v["resolution"] = proof;
    v["next_action"] =
        json!("Continue under unchanged authority; prior failures remain in history");
    v
}
pub fn observe_unit(
    w: &Value,
    u: &Value,
    cause: Cause,
    operation: &str,
    detail: &str,
    proof: Value,
) -> Result<Value, String> {
    let mut record = observe(
        w,
        cause,
        u["unit_id"]
            .as_str()
            .ok_or("Missing recovery unit identity")?,
        operation,
        detail,
        proof,
    )?;
    record["profile_sha256"] = u["selection"]["profile_sha256"].clone();
    record["requested_profile"] = u["requested_profile"].clone();
    Ok(record)
}
pub fn same_authority(w: &Value, record: &Value) -> Result<bool, String> {
    Ok(record["authority_sha256"] == authority(w)?)
}
/// A preparation refusal is produced by the scheduler before calling ensure.
/// Its signature excludes recovery diagnostics and records an actual input/release change.
pub fn preparation_signature(s: &Session, w: &Value, all: &[Value]) -> Result<String, String> {
    let preferences = if crate::provider_selection::enabled(s) {
        json!(crate::provider_selection::preferences(s)?.1)
    } else {
        Value::Null
    };
    hash(&json!([
        s.release,
        authority(w)?,
        preferences,
        w["head"],
        all.iter()
            .map(|u| json!([
                u["unit_id"],
                u["status"],
                u["result_commit"],
                u["review"],
                u["attempt_revision"],
                u["evidence_context"]
            ]))
            .collect::<Vec<_>>(),
        w["checks"],
        w["answers"],
        w["steering"]
    ]))
}
/// Review preparation depends on its frozen targets, selected profile and
/// authority, not unrelated coordinator pages, checks or sibling status changes.
pub fn unit_preparation_signature(s: &Session, w: &Value, u: &Value, all: &[Value]) -> Result<String, String> {
    if u["role"] != "review" { return preparation_signature(s,w,all); }
    let selected = crate::provider_selection::execution(s,"review",&u["profile_override"],
        u["fallback_profile"].as_str().or(u["requested_profile"].as_str()).filter(|v|!v.is_empty()),&w["policy"],"review preparation inputs");
    let selection = match selected { Ok((_,v))=>json!([v["profile_sha256"],v["profile"]]),Err(e)=>json!({"unavailable":e}) };
    let targets = |names: &Value| names.as_array().into_iter().flatten().map(|name| {
        all.iter().find(|t|t["name"]==*name).map(|t|json!([t["unit_id"],t["result_commit"],t["requirement_generation"],t["finding_ledger"],t["review"],t["selection"]["profile_sha256"]])).unwrap_or(Value::Null)
    }).collect::<Vec<_>>();
    hash(&json!([s.release,authority(w)?,selection,u["instructions"],u["attempt_revision"],u["evidence_context"],targets(&u["review_target_names"]),targets(&u["resolution_target_names"])]))
}
pub fn unit_preparation_changed(s: &Session,w: &Value,u: &Value,all: &[Value])->Result<bool,String>{
    if u["recovery"]["source_evidence"]["proof"]["signature_kind"] != "assignment-v2" {
        let mut probe=w.clone();probe["recovery"]=u["recovery"].clone();
        return preparation_changed(s,&probe,all); // legacy proof remains immutable
    }
    Ok(preparation_proven(s,w,&u["recovery"])? && u["recovery"]["source_evidence"]["proof"]["signature"] != unit_preparation_signature(s,w,u,all)?)
}
pub fn preparation_proven(s:&Session,w:&Value,r:&Value)->Result<bool,String>{
    if r["cause"]!="preparation"||!same_authority(w,r)?{return Ok(false)}
    let Some(path)=r["source_evidence"]["path"].as_str() else{return Ok(false)};
    let path=Path::new(path).canonicalize().map_err(|e|e.to_string())?;
    if !path.starts_with(s.workflow.state.canonicalize().map_err(|e|e.to_string())?){return Err("Preparation proof leaves project state".into())}
    let proof=read(&path)?;
    let directory=path.parent().ok_or("Preparation directory missing")?;
    Ok(hash(&proof)?==r["source_evidence"]["sha256"]&&proof==r["source_evidence"]["proof"]
        &&proof["no_host_attempt_prepared"]==true&&proof["authority_sha256"]==authority(w)?
        &&!["active-host-attempt.json","invocation-reservation.json","attempts"].iter().any(|p|directory.join(p).exists()))
}
pub fn preparation_refusal(
    s: &Session,
    w: &Value,
    directory: &Path,
    signature: &str,
    error: &str,
) -> Result<Value, String> {
    preparation_refusal_kind(s,w,directory,signature,error,"workflow-v1")
}
pub fn preparation_refusal_kind(s:&Session,w:&Value,directory:&Path,signature:&str,error:&str,kind:&str)->Result<Value,String>{
    if !directory.starts_with(&s.workflow.state) {
        return Err("Preparation proof leaves project state".into());
    }
    // This function is only called on the preparation stack, before ensure/reservation.
    // Refuse to manufacture nonlaunch proof for any retained execution identity.
    if directory.join("active-host-attempt.json").exists()
        || directory.join("invocation-reservation.json").exists()
        || directory.join("attempts").exists()
    {
        return Err(
            "Preparation refusal cannot attest nonlaunch for retained host attempts".into(),
        );
    }
    let proof = json!({"version":1,"source":"scheduler preparation before ensure/reservation","signature":signature,"signature_kind":kind,"authority_sha256":authority(w)?,"no_host_attempt_prepared":true,"error":error});
    write(&directory.join("preparation-refusal.json"), &proof, true)?;
    Ok(
        json!({"path":directory.join("preparation-refusal.json"),"sha256":hash(&proof)?,"proof":proof}),
    )
}
pub fn preparation_changed(s: &Session, w: &Value, all: &[Value]) -> Result<bool, String> {
    let r = &w["recovery"];
    if r["cause"] != "preparation" || r["phase"] == "resolved" || !same_authority(w, r)? {
        return Ok(false);
    }
    let Some(path) = r["source_evidence"]["path"].as_str() else {
        return Ok(false);
    };
    let path = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(s.workflow.state.canonicalize().map_err(|e| e.to_string())?) {
        return Err("Preparation proof leaves project state".into());
    }
    let proof = read(&path)?;
    Ok(hash(&proof)? == r["source_evidence"]["sha256"]
        && proof["no_host_attempt_prepared"] == true
        && proof["authority_sha256"] == authority(w)?
        && proof["signature"] != preparation_signature(s, w, all)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_recovery_identity_ignores_observation_time_and_keeps_authority() {
        let w = json!({"scope_base":"scope","decision_revision":1,"requirement_generation":2,"policy":{"review":"claude"}});
        let a = observe(
            &w,
            Cause::UncertainExecution,
            "worker",
            "operation",
            "Exact receipt missing",
            json!({"observed":1}),
        )
        .unwrap();
        let b = observe(
            &w,
            Cause::UncertainExecution,
            "worker",
            "operation",
            "Still checking",
            json!({"observed":2}),
        )
        .unwrap();
        assert_eq!(a["recovery_id"], b["recovery_id"]);
        assert_eq!(a["phase"], "blocked");
        let mut changed = w.clone();
        changed["requirement_generation"] = json!(3);
        assert!(!same_authority(&changed, &a).unwrap());
        assert_eq!(
            resolved(&a, json!({"receipt":"validated"}))["phase"],
            "resolved"
        );
        assert_eq!(a["phase"], "blocked");
    }
}

fn tree(s: &Session, commit: &Value) -> Value {
    commit
        .as_str()
        .and_then(|c| {
            crate::development::git(&s.project, &["rev-parse", &format!("{c}^{{tree}}")]).ok()
        })
        .map(Value::from)
        .unwrap_or_else(|| commit.clone())
}
/// Compact facts, not narration or task IDs. Timestamps only select the latest
/// observation per ownership/role; they never enter the measured state.
pub fn progress(s: &Session, w: &Value, all: &[Value]) -> Result<Value, String> {
    let mut latest = std::collections::BTreeMap::<String, (f64, Value)>::new();
    for u in all
        .iter()
        .filter(|u| !matches!(u["status"].as_str(), Some("superseded" | "cancelled")))
    {
        let key = hash(&json!([
            u["role"],
            ordered(&u["paths"]),
            u["review_target_names"].as_array().map(|a| a
                .iter()
                .map(|n| unit_approach(s, n, all))
                .collect::<Vec<_>>())
        ]))?;
        let stamp = u["started_at"].as_f64().unwrap_or(0.0);
        let status = match u["status"].as_str() {
            Some("ready" | "planned" | "running") => "active",
            Some("integrated" | "accepted") => "integrated",
            Some(v) => v,
            None => "unknown",
        };
        let mut error = u["receipt"]["error"].as_str().unwrap_or("").to_owned();
        for value in [
            u["operation_id"].as_str(),
            u["attempt_id"].as_str(),
            u["worktree"].as_str(),
            s.workflow.state.to_str(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        {
            error = error.replace(value, "<execution>");
        }
        let findings = u["finding_ledger"].as_array().map(|a| {
            a.iter()
                .map(|f| json!([tree(s, &f["target_commit"]), f["text"], f["disposition"]]))
                .collect::<Vec<_>>()
        });
        let value = json!({"role":u["role"],"paths":ordered(&u["paths"]),"base_tree":tree(s,&u["base_commit"]),"result_tree":tree(s,&u["result_commit"]),"status":status,
            "settled_outcome":u["receipt"]["status"],"error":error,"findings":findings,"review_requires_changes":u["review"]["requires_changes"],"profile":u["selection"]["profile_sha256"]});
        if latest.get(&key).is_none_or(|(at, _)| *at <= stamp) {
            latest.insert(key, (stamp, value));
        }
    }
    let context = crate::development_context::progress_evidence(s, &w["evidence_context"])?;
    let checks = w["checks"]["result"]["checks"].as_array().map(|a| {
        a.iter()
            .map(|c| json!([c["argv"], c["success"], c["exit_code"]]))
            .collect::<Vec<_>>()
    });
    let mut finding_states = std::collections::BTreeSet::new();
    for u in all
        .iter()
        .filter(|u| u["requirement_generation"] == w["requirement_generation"])
    {
        for f in u["finding_ledger"].as_array().into_iter().flatten() {
            finding_states.insert(
                serde_json::to_string(&json!([
                    tree(s, &f["target_commit"]),
                    f["text"],
                    f["disposition"]
                ]))
                .map_err(|e| e.to_string())?,
            );
        }
    }
    let state = json!({"authority":authority(w)?,"source_tree":tree(s,&w["head"]),"inputs":[w["answers"],w["steering"]],
        "findings":finding_states,"units":latest.into_values().map(|(_,v)|v).collect::<Vec<_>>(),"evidence":context,
        "obligations":w["obligations"].as_array().map(|a|a.iter().map(|o|json!([o["name"],o["status"]])).collect::<Vec<_>>()),
        "checks":{"tree":tree(s,&w["checks"]["commit"]),"passed":w["checks"]["result"]["passed"],"outcomes":checks}});
    Ok(json!({"version":1,"sha256":hash(&state)?,"facts":state}))
}

fn unit_approach(s: &Session, name: &Value, all: &[Value]) -> Value {
    all.iter()
        .find(|u| u["name"] == *name || u["unit_id"] == *name)
        .map(|u| {
            json!([
                u["role"],
                u["paths"],
                tree(s, &u["base_commit"]),
                tree(s, &u["result_commit"]),
                u["finding_ledger"].as_array().map(|a| a
                    .iter()
                    .map(|f| json!([tree(s, &f["target_commit"]), f["text"], f["disposition"]]))
                    .collect::<Vec<_>>())
            ])
        })
        .unwrap_or(Value::Null)
}
fn ordered(value: &Value) -> Value {
    let mut values = value.as_array().cloned().unwrap_or_default();
    values.sort_by_key(Value::to_string);
    json!(values)
}
pub fn approach(
    s: &Session,
    result: &Value,
    packet: &Value,
    all: &[Value],
) -> Result<Value, String> {
    let mut tasks=result["tasks"].as_array().into_iter().flatten().map(|task|json!({"role":task["role"].as_str().unwrap_or("implementation"),"paths":ordered(&task["paths"]),"profile":task["profile"],
        "dependencies":task["start_after"].as_array().map(|a|a.iter().map(|n|unit_approach(s,n,all)).collect::<Vec<_>>()),"accept_after":task["accept_after"].as_array().map(|a|a.iter().map(|n|unit_approach(s,n,all)).collect::<Vec<_>>()),
        "review_targets":task["review_targets"].as_array().map(|a|a.iter().map(|n|unit_approach(s,n,all)).collect::<Vec<_>>()),"resolution_targets":task["resolution_targets"].as_array().map(|a|a.iter().map(|n|unit_approach(s,n,all)).collect::<Vec<_>>())})).collect::<Vec<_>>();
    tasks.sort_by_key(Value::to_string);
    let mut reads = result["evidence_requests"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let entry = packet["evidence_index"]["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|e| e["id"] == r["evidence_id"]);
            json!([
                entry.map(|e| json!([e["kind"], e["sha256"], tree(s, &e["commit"])])),
                r["path"],
                r["cursor"]
            ])
        })
        .collect::<Vec<_>>();
    reads.sort_by_key(Value::to_string);
    Ok(
        json!({"action":result["action"],"tasks":tasks,"units":result["units"].as_array().map(|a|a.iter().map(|n|unit_approach(s,n,all)).collect::<Vec<_>>()),"checks":result["checks"],"reads":reads,"resolved_obligations":ordered(&result["resolved_obligations"])}),
    )
}
fn redundant_read(result: &Value, packet: &Value) -> bool {
    let Some(requests) = result["evidence_requests"]
        .as_array()
        .filter(|a| !a.is_empty())
    else {
        return false;
    };
    requests.iter().all(|r| {
        let cursor = r["cursor"].as_str().unwrap_or("");
        let parsed = cursor.split_once(':');
        let offset = if cursor.is_empty() {
            Some(0)
        } else {
            parsed.and_then(|(_, n)| n.parse::<u64>().ok())
        };
        packet["evidence_pages"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|p| {
                p["error"].is_null()
                    && p["evidence_id"] == r["evidence_id"]
                    && p["path"] == r["path"]
                    && offset == p["offset"].as_u64()
                    && p["data"].is_string()
                    && (cursor.is_empty() || parsed.is_some_and(|(sha, _)| p["sha256"] == sha))
            })
    })
}
/// Repetition is a fact comparison, not a call/time/repair allowance. One useful
/// reassessment is requested per unchanged condition; equivalent work then waits
/// for a real delta. Different useful actions and changed facts remain eligible.
pub fn reassess(
    w: &mut Value,
    snapshot: &Value,
    approach: &Value,
    redundant: bool,
) -> Result<bool, String> {
    let state = &snapshot["sha256"];
    let action_hash = hash(approach)?;
    let mut history = w["progress"]["recent"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let equivalent = approach["action"] != "wait"
        && (redundant
            || history
                .iter()
                .any(|old| old["state_sha256"] == *state && old["approach_sha256"] == action_hash));
    let previous = history.last().cloned().unwrap_or(Value::Null);
    if equivalent {
        let waiting = w["progress"]["reassessment_state"] == *state;
        w["progress"]["reassessment_state"] = state.clone();
        w["progress"]["wait_state"] = if waiting { state.clone() } else { Value::Null };
        let mut r = observe(
            w,
            Cause::EquivalentWork,
            w["work_id"].as_str().unwrap_or(""),
            "semantic-progress",
            "Equivalent work has no meaningful code, evidence, finding, check or execution delta",
            json!({"state_sha256":state,"approach_sha256":action_hash,"prior":previous,"redundant_supplied_read":redundant}),
        )?;
        r["phase"] = json!(if waiting { "waiting" } else { "coordinating" });
        r["next_action"] = json!(if waiting {
            "Wait for useful evidence or changed work; conversation and independent tasks remain available"
        } else {
            "Reassess once using the saved approach and actual evidence; choose a useful different action"
        });
        w["recovery"] = r;
        w["progress"]["current"] = snapshot.clone();
        return Ok(true);
    }
    history.push(json!({"state_sha256":state,"approach_sha256":action_hash,"approach":approach}));
    if history.len() > 16 {
        history.drain(..history.len() - 16);
    }
    w["progress"] = json!({"version":1,"current":snapshot,"recent":history,"reassessment_state":w["progress"]["reassessment_state"],"wait_state":null});
    if w["recovery"]["cause"] == "equivalent_work" {
        w["recovery"] = resolved(
            &w["recovery"],
            json!({"different_useful_action":action_hash}),
        );
    }
    Ok(false)
}
pub fn assess_decision(
    s: &Session,
    w: &mut Value,
    packet: &Value,
    result: &Value,
    all: &[Value],
) -> Result<bool, String> {
    let snapshot = if packet["progress_snapshot"].is_object() {
        packet["progress_snapshot"].clone()
    } else {
        progress(s, w, all)?
    };
    reassess(
        w,
        &snapshot,
        &approach(s, result, packet, all)?,
        redundant_read(result, packet),
    )
}
pub fn assess_review_read(
    s: &Session,
    w: &Value,
    u: &mut Value,
    packet: &Value,
    result: &Value,
) -> Result<bool, String> {
    let snapshot = progress(s, w, std::slice::from_ref(u))?;
    let mut scoped = w.clone();
    scoped["progress"] = u["progress"].clone();
    let mut action = result.clone();
    action["action"] = json!("retrieve");
    let stopped = reassess(
        &mut scoped,
        &snapshot,
        &approach(s, &action, packet, &[])?,
        redundant_read(result, packet),
    )?;
    u["progress"] = scoped["progress"].clone();
    if stopped {
        let mut record = observe_unit(
            w,
            u,
            Cause::EquivalentWork,
            u["operation_id"].as_str().unwrap_or(""),
            "Reviewer requested equivalent evidence already supplied; no final review accepted",
            scoped["recovery"]["source_evidence"].clone(),
        )?;
        record["next_action"] = json!(
            "Coordinator will reassess this review continuation using its retained evidence and findings"
        );
        u["recovery"] = record;
    }
    Ok(stopped)
}
pub fn context_progress(record: &Value) -> Value {
    if !record["progress"].is_object() {
        return Value::Null;
    }
    json!({"version":1,"current_sha256":record["progress"]["current"]["sha256"],"facts":record["progress"]["current"]["facts"],"prior_approach":record["progress"]["recent"].as_array().and_then(|a|a.last()),"reassessment_state":record["progress"]["reassessment_state"],"wait_state":record["progress"]["wait_state"]})
}
pub fn wake_progress(w: &mut Value, snapshot: &Value) -> bool {
    if w["progress"]["wait_state"].is_null() {
        return true;
    }
    if w["progress"]["wait_state"] == snapshot["sha256"] {
        return false;
    }
    w["progress"]["wait_state"] = Value::Null;
    w["progress"]["reassessment_state"] = Value::Null;
    w["recovery"] = resolved(
        &w["recovery"],
        json!({"meaningful_changed_state":snapshot["sha256"]}),
    );
    w["last_context_sha256"] = Value::Null;
    true
}
pub const INSTRUCTION: &str = "Recovery v1: workflow.recovery and workflow.progress are derived facts. Reassess equivalent work using actual evidence changes; do not retry unchanged faults or use narration/new task names as progress. A review clarification is role review, resolution_targets naming settled review tasks, and review_targets naming their exact implementation targets. It resolves linked findings without a whole review. When convergence_contract:1 is present, action retire with units and reason explicitly retires obsolete positively unlaunched assignments. Preserve outstanding obligations and finding gates; use it after replacement work rather than leaving a permanently invalid planned task. convergence describes the actionable frontier and exact combined checkout. No new authority or service/configuration repair is granted. If unchanged facts cannot support a useful different action, explain the blocker rather than repeating equivalent work.";

pub fn summary(record: &Value) -> Value {
    let mut value = record.clone();
    if let Some(o) = value.as_object_mut() {
        o.remove("source_evidence");
        o.remove("resolution");
    }
    for key in ["detail", "next_action", "wake_condition", "can_continue"] {
        if let Some(text) = record[key].as_str() {
            value[key] = json!(text.chars().take(2000).collect::<String>());
        }
    }
    value
}
/// Presentation only: a scoped blocker must not disappear behind a healthy
/// workflow. This never changes scheduling, authority or finding records.
pub fn active_recovery(w: &Value, all: &[Value]) -> Value {
    if matches!(w["status"].as_str(), Some("delivered" | "cancelled")) {
        return Value::Null;
    }
    if w["recovery"].is_object() && w["recovery"]["phase"] != "resolved" {
        return summary(&w["recovery"]);
    }
    all.iter()
        .filter(|u| {
            u["work_id"] == w["work_id"]
                && !matches!(
                    u["status"].as_str(),
                    Some("superseded" | "cancelled" | "integrated" | "accepted")
                )
        })
        .find(|u| {
            u["recovery"].is_object()
                && u["recovery"]["phase"] != "resolved"
                && !(u["recovery"]["cause"] == "finding_resolution"
                    && !crate::review_findings::effective_requires_changes(u, all))
        })
        .map(|u| {
            let mut r = summary(&u["recovery"]);
            r["scope"] = json!("assignment");
            r["unit_title"] = u["title"].clone();
            r
        })
        .unwrap_or(Value::Null)
}
pub fn presentation(record: &Value) -> String {
    if !record.is_object() {
        return String::new();
    }
    format!(
        "Recovery · {} · {}\n{}\nNext: {}\nWake: {}\n{} · {}",
        record["cause"].as_str().unwrap_or("observation"),
        record["phase"].as_str().unwrap_or("observed"),
        record["detail"].as_str().unwrap_or("Evidence retained"),
        record["next_action"]
            .as_str()
            .unwrap_or("Inspect exact evidence"),
        record["wake_condition"]
            .as_str()
            .unwrap_or("Positive evidence required"),
        if record["decision_needed"] == true {
            "Your decision is needed"
        } else {
            "No new approval or generic retry permission is needed"
        },
        record["can_continue"]
            .as_str()
            .unwrap_or("Conversation remains available")
    )
}

#[cfg(test)]
mod progress_tests {
    use super::*;
    fn work() -> Value {
        json!({"work_id":"w","scope_base":"scope","decision_revision":1,"requirement_generation":1,"policy":{"review":"claude"},"obligations":[]})
    }
    #[test]
    fn progress_ignores_task_names_operation_ids_timestamps_and_claimed_strategy() {
        let (_dir, s) = crate::provider_selection::recovery_tests::fixture();
        let w = work();
        let u = json!({"unit_id":"first","name":"first","operation_id":"first-operation","attempt_id":"first-attempt","role":"implementation","paths":["model.rs"],"status":"failed","started_at":1,"receipt":{"status":"failed","error":"first-operation failed"}});
        let mut renamed = u.clone();
        renamed["unit_id"] = json!("renamed");
        renamed["name"] = json!("renamed");
        renamed["operation_id"] = json!("other-operation");
        renamed["attempt_id"] = json!("other-attempt");
        renamed["started_at"] = json!(999);
        renamed["receipt"]["error"] = json!("other-operation failed");
        renamed["instructions"] = json!("Try the same thing with different narration");
        assert_eq!(
            progress(&s, &w, &[u]).unwrap(),
            progress(&s, &w, &[renamed]).unwrap()
        );
        let first = json!({"action":"assign","tasks":[{"id":"first","instructions":"Approach A","role":"implementation","paths":["model.rs"]}]});
        let mut second = first.clone();
        second["tasks"][0]["id"] = json!("renamed");
        second["tasks"][0]["instructions"] = json!("Claim a new strategy");
        assert_eq!(
            approach(&s, &first, &Value::Null, &[]).unwrap(),
            approach(&s, &second, &Value::Null, &[]).unwrap()
        );
    }
    #[test]
    fn progress_equivalent_failure_and_circular_revision_reassess_then_wait_until_real_delta() {
        let mut w = work();
        let a = json!({"sha256":"code-evidence-A","facts":{"code":"A"}});
        let b = json!({"sha256":"code-evidence-B","facts":{"code":"B"}});
        let action = json!({"action":"assign","paths":["model.rs"]});
        assert!(!reassess(&mut w, &a, &action, false).unwrap());
        assert!(!reassess(&mut w, &b, &action, false).unwrap());
        assert!(reassess(&mut w, &a, &action, false).unwrap());
        assert_eq!(w["recovery"]["phase"], "coordinating");
        let saved = serde_json::to_vec(&w).unwrap();
        let mut restarted: Value = serde_json::from_slice(&saved).unwrap();
        assert!(reassess(&mut restarted, &a, &action, false).unwrap());
        assert_eq!(restarted["recovery"]["phase"], "waiting");
        assert!(!wake_progress(&mut restarted, &a));
        let new =
            json!({"sha256":"new-evidence-or-finding-disposition","facts":{"new":"exact source"}});
        assert!(wake_progress(&mut restarted, &new));
        assert_eq!(restarted["recovery"]["phase"], "resolved");
        assert!(!reassess(&mut restarted, &new, &action, false).unwrap());
        assert_eq!(w["policy"], restarted["policy"]);
    }
    #[test]
    fn progress_duplicate_supplied_reads_trigger_reassessment_without_retrieval_or_new_permission()
    {
        let p = json!({"evidence_pages":[{"evidence_id":"source:exact","path":"cli.rs","offset":0,"end":100,"sha256":"sha","data":"already supplied"}]});
        let result = json!({"evidence_requests":[{"evidence_id":"source:exact","path":"cli.rs","cursor":""}]});
        assert!(redundant_read(&result, &p));
        let mut released = p.clone();
        released["evidence_pages"] = json!([]);
        assert!(
            !redundant_read(&result, &released),
            "Explicitly released raw evidence can usefully be read again"
        );
        let mut w = work();
        assert!(
            reassess(
                &mut w,
                &json!({"sha256":"same"}),
                &json!({"action":"retrieve"}),
                true
            )
            .unwrap()
        );
        assert_eq!(w["recovery"]["phase"], "coordinating");
    }
    #[test]
    fn progress_finding_check_and_supplied_range_changes_count_but_capacity_wait_is_not_waste() {
        let (_dir, s) = crate::provider_selection::recovery_tests::fixture();
        let mut w = work();
        let u = json!({"role":"review","status":"reviewed","finding_ledger":[{"text":"Verify the source","disposition":"blocking"}]});
        let first = progress(&s, &w, &[u.clone()]).unwrap();
        let mut resolved = u.clone();
        resolved["finding_ledger"][0]["disposition"] = json!("resolved");
        assert_ne!(
            first["sha256"],
            progress(&s, &w, &[resolved]).unwrap()["sha256"]
        );
        w["checks"] = json!({"result":{"passed":true,"checks":[{"argv":["cargo","test"],"success":true,"exit_code":0}]}});
        assert_ne!(first["sha256"], progress(&s, &w, &[u]).unwrap()["sha256"]);
        w["evidence_context"] = json!({"entries":[{"id":"source:exact","kind":"source"}],"pages":[{"evidence_id":"source:exact","path":"cli.rs","sha256":"hash","offset":0,"end":100,"total_bytes":200}]});
        let part = progress(&s, &w, &[]).unwrap();
        w["evidence_context"]["pages"][0]["end"] = json!(200);
        assert_ne!(part["sha256"], progress(&s, &w, &[]).unwrap()["sha256"]);
        let wait = json!({"action":"wait"});
        let snapshot = json!({"sha256":"capacity"});
        assert!(!reassess(&mut w, &snapshot, &wait, false).unwrap());
        assert!(!reassess(&mut w, &snapshot, &wait, false).unwrap());
        assert!(w["progress"]["wait_state"].is_null());
    }
    #[test]
    fn progress_reviewer_duplicate_read_returns_to_coordinator_preserving_findings_and_native_result()
     {
        let (_dir, s) = crate::provider_selection::recovery_tests::fixture();
        let w = work();
        let packet = json!({"evidence_pages":[{"evidence_id":"source:exact","path":"cli.rs","offset":0,"end":100,"sha256":"sha","data":"supplied"}]});
        let result = json!({"evidence_requests":[{"evidence_id":"source:exact","path":"cli.rs","cursor":""}],"findings":["Still unresolved"]});
        let mut u = json!({"unit_id":"claude-review","work_id":"w","operation_id":"saved-review","role":"review","status":"running","selection":{"profile_sha256":"selected-claude"},"native_review":result,"finding_ledger":[{"finding_id":"retained","text":"Still unresolved","disposition":"blocking"}]});
        let native = u["native_review"].clone();
        let ledger = u["finding_ledger"].clone();
        assert!(assess_review_read(&s, &w, &mut u, &packet, &result).unwrap());
        assert_eq!(u["native_review"], native);
        assert_eq!(u["finding_ledger"], ledger);
        assert_eq!(u["recovery"]["profile_sha256"], "selected-claude");
        assert_eq!(u["recovery"]["subject_id"], "claude-review");
        assert!(
            u["recovery"]["next_action"]
                .as_str()
                .unwrap()
                .contains("Coordinator")
        );
        let mut state = work();
        let snapshot = json!({"sha256":"unchanged-settled-failure"});
        let failure = json!({"action":"settled_failure","diagnostic":"same native format refusal"});
        assert!(!reassess(&mut state, &snapshot, &failure, false).unwrap());
        assert!(reassess(&mut state, &snapshot, &failure, false).unwrap());
        assert!(reassess(&mut state, &snapshot, &failure, false).unwrap());
        assert_eq!(state["recovery"]["phase"], "waiting");
    }
}
