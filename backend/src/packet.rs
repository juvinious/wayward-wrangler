//! Frozen worker packet construction from already-validated local artifact text.
//! No filesystem scan, registry access, execution or approval side effects.
use crate::{context_policy, delta};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn required<'a>(v: &'a Value, k: &str) -> Result<&'a Value, String> {
    v.get(k).ok_or_else(|| format!("missing packet input {k}"))
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    required(v, k)?
        .as_str()
        .ok_or_else(|| format!("invalid packet input {k}"))
}

pub fn build(
    envelope: &Value,
    state: &Value,
    stage: &str,
    current: &str,
    previous: &str,
) -> Result<Value, String> {
    let config = required(envelope, "config")?;
    let role = if stage.starts_with("critique") {
        "critic"
    } else {
        "author"
    };
    let task = if stage.starts_with("critique") {
        "critique"
    } else if stage.starts_with("revision") {
        "revise"
    } else {
        "draft"
    };
    let role_config = &config["roles"][role];
    if role_config.is_null() {
        return Err(format!("missing role config {role}"));
    }
    let run_id = text(state, "run_id")?;
    let decisions = state.get("decisions").cloned().unwrap_or_else(|| json!([]));
    let findings = state.get("findings").cloned().unwrap_or_else(|| json!({}));
    let finding_map = findings.as_object().ok_or("invalid findings")?;
    let order = if let Some(order) = state.get("finding_order") {
        let order = order
            .as_array()
            .ok_or("invalid finding order")?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or("invalid finding order ID")
                    .map(str::to_owned)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let actual = order.iter().map(String::as_str).collect::<BTreeSet<_>>();
        if actual.len() != order.len() || actual != finding_map.keys().map(String::as_str).collect()
        {
            return Err("finding order disagrees with durable findings".into());
        }
        order
    } else {
        finding_map.keys().cloned().collect::<Vec<_>>()
    };
    let open = order
        .iter()
        .filter_map(|id| finding_map.get(id))
        .filter(|value| value["status"] == "open")
        .cloned()
        .collect::<Vec<_>>();
    let mut result = json!({
        "schema_version":1,"run_id":run_id,"operation_id":format!("{run_id}-{stage}"),
        "stage":stage,"role":role,"provider":required(role_config,"provider")?,
        "model":required(role_config,"model")?,
        "seconds":config["fake"]["seconds"].as_f64().map(|x|json!(x)).unwrap_or_else(||json!(0)),
        "mode":"success","fake_question_count":config["fake"]["questions"].as_u64().unwrap_or(1),
        "brief":required(envelope,"brief")?,"instructions":required(envelope,"instructions")?,
        "skill":required(&envelope["skills"],task)?,
        "rules_sha256":required(envelope,"rules_sha256")?,
        "config_sha256":required(envelope,"config_sha256")?,
        "current_draft":current,"current_ref":state.get("current").cloned().unwrap_or(Value::Null),
        "previous_reviewed":if stage=="critique-2"{previous}else{""},
        "delta":if stage=="critique-2"{delta::unified(previous,current)}else{String::new()},
        "decisions":decisions,"open_findings":open,"finding_dispositions":findings,
        "critique":state.get("last_critique").cloned().unwrap_or(Value::Null),
        "authority":"Return text and findings only. Never approve, execute tools, or reopen accepted decisions."
    });
    let map = result.as_object_mut().ok_or("packet is not object")?;
    if role_config["provider"] != "fake" {
        map.insert("execution".into(), role_config.clone());
        map.insert(
            "planned_decision".into(),
            config.get("decision").cloned().unwrap_or(Value::Null),
        );
        let completed = state["completed"].as_array().map(Vec::len).unwrap_or(0);
        let max = if envelope.get("native_authorization").is_some() {
            config["max_invocations"]
                .as_u64()
                .ok_or("missing max_invocations")? as usize
        } else {
            5
        };
        map.insert(
            "remaining_stage_budget".into(),
            json!(max.saturating_sub(completed)),
        );
        if let Some(auth) = envelope.get("native_authorization") {
            map.insert(
                "max_invocations".into(),
                required(config, "max_invocations")?.clone(),
            );
            map.insert(
                "authorization_plan_sha256".into(),
                required(auth, "plan_sha256")?.clone(),
            );
            map.insert(
                "prompt_contract".into(),
                required(config, "prompt_contract")?.clone(),
            );
        }
    }
    if envelope["workflow_version"] == 2 {
        map.insert("schema_version".into(), json!(2));
        map.insert(
            "context_snapshot".into(),
            required(envelope, "context_snapshot")?.clone(),
        );
        map.insert(
            "context_digest".into(),
            required(envelope, "context_digest")?.clone(),
        );
        map.insert(
            "decisions".into(),
            json!(context_policy::effective_decisions(state)?),
        );
        map.insert(
            "decision_revision".into(),
            required(state, "decision_revision")?.clone(),
        );
        let max = config["max_questions"].as_u64().unwrap_or(1);
        let asked = state["questions_asked"].as_u64().unwrap_or(0);
        map.insert(
            "new_question_budget".into(),
            json!(max.saturating_sub(asked)),
        );
        map.insert("authority".into(),json!("Return provisional text/findings/questions only. Human decisions remain controller-owned. Reconsider inherited policy only with referenced new evidence."));
        map.remove("planned_decision");
        let ids = context_policy::evidence_ids(&Value::Object(map.clone()))?;
        map.insert("evidence_ids".into(), json!(ids));
    }
    if matches!(stage, "critique-2" | "revision-2")
        && state["coverage_gaps"]["revision-1"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
    {
        map.insert(
            "revision_coverage".into(),
            json!({"revision-1":state["coverage_gaps"]["revision-1"]}),
        );
        let authority = map["authority"].as_str().ok_or("missing authority")?;
        map.insert("authority".into(),json!(format!("{authority} Revision-1 omitted dispositions for the listed revision_coverage findings. Their original IDs and evidence remain open; inspect the provisional text and explicitly assess each.")));
    }
    if state["finding_contract_version"] == 2 {
        map.insert("finding_contract_version".into(), json!(2));
        map.insert(
            "finding_assessments".into(),
            state
                .get("finding_assessments")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        let authority = map["authority"].as_str().ok_or("missing authority")?;
        map.insert("authority".into(),json!(format!("{authority} New findings have no worker ID; the application assigns durable IDs. Assess existing findings only through explicit finding_ref entries. Original evidence is immutable; assessments never authorize a human decision.")));
    }
    if let Some(imports) = envelope
        .get("cross_imports")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty())
    {
        map.insert(
            "cross_project_imports".into(),
            json!(context_policy::frozen_import_packet(imports)?),
        );
    }
    Ok(result)
}
