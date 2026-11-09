//! Existing drafting/review policy, separate from durable effect and answer storage.
use crate::{codec::canonical_sha256, context_policy, findings};
use serde_json::{Value, json};

pub const STAGES: [&str; 5] = [
    "draft",
    "critique-1",
    "revision-1",
    "critique-2",
    "revision-2",
];
pub const DEFINITION: &str = "draft-review-v1";
pub fn definition_hash() -> String {
    crate::codec::sha256(DEFINITION.as_bytes())
}

pub fn initial(run: &str, envelope: &Value) -> Result<Value, String> {
    let version = envelope["workflow_version"].as_u64().unwrap_or(1);
    if !matches!(version, 1 | 2) {
        return Err("Unsupported workflow version".into());
    }
    let inherited = if version == 2 {
        context_policy::inherited(&envelope["context_snapshot"])?
    } else {
        vec![]
    };
    let mut findings = json!({});
    let mut order = Vec::<String>::new();
    if version == 2 {
        for entry in envelope["context_snapshot"]["entries"]
            .as_array()
            .ok_or("missing context entries")?
        {
            if entry["kind"] == "findings" {
                let id = entry["qualified_id"]
                    .as_str()
                    .ok_or("invalid inherited finding")?;
                findings[id] = entry["content"].clone();
                order.push(id.into());
            }
        }
    }
    Ok(
        json!({"schema_version":2,"run_id":run,"workflow_version":version,
        "next_stage":"draft","completed":[],"current":null,"last_reviewed":null,
        "last_critique":null,"findings":findings,"finding_order":order,
        "decisions":[],"decision_revision":0,"questions":[],"pending":null,
        "outcome":"","coverage_gaps":{},"finding_assessments":[],
        "finding_contract_version":envelope["config"]["finding_contract_version"].as_u64().unwrap_or(1),
        "inherited_decisions":inherited,"context_digest":envelope["context_digest"],
        "questions_asked":0,"max_questions":envelope["config"]["max_questions"].as_u64().unwrap_or(1),
        "revision_required":false,"blocked_reason":"","question_origin":{}}),
    )
}
pub fn next_stage(stage: &str) -> Result<&'static str, String> {
    match stage {
        "draft" => Ok("critique-1"),
        "critique-1" => Ok("revision-1"),
        "revision-1" => Ok("critique-2"),
        "critique-2" => Ok("revision-2"),
        "revision-2" => Ok("final"),
        _ => Err("Completed or invalid stage cannot dispatch".into()),
    }
}
pub fn after_result(
    state: &Value,
    envelope: &Value,
    stage: &str,
    packet: &Value,
    result: &Value,
    reference: &Value,
    classified: &Value,
) -> Result<Value, String> {
    if state["next_stage"] != stage
        || state["completed"]
            .as_array()
            .is_none_or(|a| a.iter().any(|x| x == stage))
    {
        return Err("Completed or unexpected stage cannot apply".into());
    }
    let mut next = state.clone();
    let original_order = next["finding_order"]
        .as_array()
        .ok_or("invalid finding order")?
        .clone();
    let mut order = original_order;
    for item in classified["new"].as_array().ok_or("invalid new findings")? {
        order.push(item["id"].clone());
    }
    next["finding_order"] = json!(order);
    next["findings"] = findings::apply(
        &state["findings"],
        classified,
        reference["id"].as_str().ok_or("invalid artifact ID")?,
        stage,
        &state["current"],
    )?;
    let mut history = next["finding_assessments"]
        .as_array()
        .ok_or("invalid assessment history")?
        .clone();
    history.extend(
        classified["assessments"]
            .as_array()
            .ok_or("invalid assessments")?
            .clone(),
    );
    next["finding_assessments"] = json!(history);
    if stage.starts_with("revision") {
        next["coverage_gaps"][stage] = classified["missing"].clone();
    }
    let mut completed = next["completed"]
        .as_array()
        .ok_or("invalid completed")?
        .clone();
    completed.push(stage.into());
    next["completed"] = json!(completed);
    let mut route = next_stage(stage)?.to_owned();
    let version = state["workflow_version"].as_u64().unwrap_or(1);
    if version == 2 {
        let known = context_policy::effective_decisions(state)?;
        let mut questions = Vec::new();
        let mut blocked = String::new();
        for q in result["questions"].as_array().ok_or("invalid questions")? {
            let previous = known
                .iter()
                .find(|d| d["question"]["topic_key"] == q["topic_key"]);
            if let Some(prior) = previous {
                if q["reconsideration_reason"]
                    .as_str()
                    .unwrap_or("")
                    .is_empty()
                {
                    if q["question"] == prior["question"]["question"]
                        && q["options"] == prior["question"]["options"]
                    {
                        continue;
                    }
                    blocked="Question conflicts with an accepted topic without evidenced reconsideration; inspect retained result".into();
                } else {
                    let qualified = prior["qualified_id"]
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            format!(
                                "{}/{}",
                                state["run_id"].as_str().unwrap_or(""),
                                prior["request_id"].as_str().unwrap_or("")
                            )
                        });
                    if !q["related_decision_refs"]
                        .as_array()
                        .is_some_and(|a| a.iter().any(|r| r == &qualified))
                    {
                        blocked =
                            "Reconsideration does not reference the accepted topic's decision"
                                .into();
                    }
                }
            }
            questions.push(q.clone());
        }
        let asked = state["questions_asked"].as_u64().unwrap_or(0);
        let allowed = state["max_questions"].as_u64().unwrap_or(1);
        if asked + questions.len() as u64 > allowed {
            blocked = format!(
                "Frozen {allowed}-question limit reached; result retained, no automatic retry"
            );
        }
        let mut needs_answer = state["revision_required"] == true;
        if stage.starts_with("revision") && questions.is_empty() {
            needs_answer = false;
        }
        if stage.starts_with("critique")
            && result["needs_revision"] == false
            && questions.is_empty()
            && !needs_answer
            && !(stage == "critique-2"
                && state["coverage_gaps"]["revision-1"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty()))
        {
            route = "final".into();
        }
        next["questions"] = json!(questions);
        next["questions_asked"] = json!(asked + questions.len() as u64);
        next["revision_required"] = needs_answer.into();
        next["blocked_reason"] = blocked.into();
        next["question_origin"] = json!({"operation_id":result["operation_id"],"artifact":reference,
            "result_sha256":canonical_sha256(result).map_err(|e|e.to_string())?,
            "packet_sha256":canonical_sha256(packet).map_err(|e|e.to_string())?});
    } else {
        let mut questions = result["questions"]
            .as_array()
            .ok_or("invalid questions")?
            .clone();
        if envelope["config"]["trial_id"].as_str().is_some() && stage == "critique-1" {
            questions = vec![envelope["config"]["decision"].clone()];
        }
        if stage.starts_with("critique")
            && result["needs_revision"] == false
            && !(envelope["config"]["trial_id"].as_str().is_some() && stage == "critique-1")
        {
            route = "final".into();
        }
        next["questions"] = json!(questions);
    }
    next["next_stage"] = route.into();
    if stage.starts_with("critique") {
        next["last_reviewed"] = state["current"].clone();
        next["last_critique"] = json!({"artifact":reference,"result":result});
    } else {
        next["current"] = reference.clone();
    }
    Ok(next)
}

pub fn request(state: &Value) -> Result<Option<Value>, String> {
    if !state["blocked_reason"].as_str().unwrap_or("").is_empty() {
        return Ok(None);
    }
    let run = state["run_id"].as_str().ok_or("missing run ID")?;
    let revision = state["decision_revision"]
        .as_u64()
        .ok_or("missing decision revision")?;
    let candidate = &state["current"];
    if let Some(question) = state["questions"].as_array().and_then(|a| a.first()) {
        let id = question["id"].as_str().ok_or("invalid question ID")?;
        let request_id = if state["workflow_version"] == 2 {
            let binding = json!({"origin":state["question_origin"],"id":id});
            format!(
                "D-{}",
                &canonical_sha256(&binding).map_err(|e| e.to_string())?[..24]
            )
        } else {
            format!("D-{id}")
        };
        let mut request = json!({"request_id":request_id,"kind":"choice","question":question,
            "artifact_sha256":candidate["sha256"],"decision_revision":revision,
            "dependent_stage":state["next_stage"]});
        if state["workflow_version"] == 2 {
            request["context_digest"] = state["context_digest"].clone();
            request["source"] = state["question_origin"].clone();
            request["inherited_decisions"] = state["inherited_decisions"].clone();
        }
        return Ok(Some(request));
    }
    if state["next_stage"] == "final" {
        if state["revision_required"] == true {
            return Err("Current artifact predates the accepted answer".into());
        }
        let mut notice="Final revisions may not have been independently re-reviewed; inspect last_reviewed. Open findings remain unresolved unless explicitly addressed; review provisional sections before approval.".to_owned();
        let findings = state["findings"].as_object().ok_or("invalid findings")?;
        let unresolved = findings
            .iter()
            .filter(|(_, v)| v["status"] == "open")
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if !unresolved.is_empty() {
            notice.push_str(&format!(
                " Provisional specification: unresolved findings {}.",
                unresolved.join(", ")
            ));
        }
        let mut result = json!({"request_id":"A1","kind":"final-review","artifact_sha256":candidate["sha256"],
            "artifact":candidate,"decision_revision":revision,"last_reviewed":state["last_reviewed"],
            "findings":state["findings"],"decisions":state["decisions"],"notice":notice,
            "coverage_gaps":state["coverage_gaps"],"finding_assessments":state["finding_assessments"]});
        if state["workflow_version"] == 2 {
            result["context_digest"] = state["context_digest"].clone();
            result["decisions"] = json!(context_policy::effective_decisions(state)?);
        }
        return Ok(Some(result));
    }
    let _ = run;
    Ok(None)
}
