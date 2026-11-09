//! Deterministic simulated coordinator/worker policy. No authentication or model calls.
use crate::codec::canonical_sha256;
use serde_json::{Value, json};

pub fn result(packet: &Value) -> Result<Value, String> {
    if packet["profile"]==crate::static_files::PROFILE{return crate::static_files::fake_result(packet);}
    let operation_id = packet["operation_id"]
        .as_str()
        .ok_or("missing operation ID")?;
    let hash = canonical_sha256(packet).map_err(|e| e.to_string())?;
    if packet["mode"] == "failure" {
        return Err("Deliberate fake failure: no valid result".into());
    }
    if packet["operation_kind"] == "coordinator" {
        return Ok(
            json!({"schema_version":1,"operation_id":operation_id,"packet_sha256":hash,
            "proposal":{"schema_version":1,"action":"propose_task",
                "task_id":packet["task_id"],"task_version":packet["task_version"],
                "message_sha256":packet["message_sha256"],"brief":packet["message"]}}),
        );
    }
    let stage = packet["stage"].as_str().ok_or("missing stage")?;
    let brief = packet["brief"]
        .as_str()
        .unwrap_or("Visible execution foundation.");
    let mut output = json!({"schema_version":packet["schema_version"].as_u64().unwrap_or(1),
        "operation_id":operation_id,"packet_sha256":hash,
        "content":format!("# Simulated draft\n\n{brief}"),
        "findings":[],"dispositions":[],"questions":[],"needs_revision":false});
    if packet["schema_version"] == 2 && stage == "draft" {
        let questions = output["questions"]
            .as_array_mut()
            .ok_or("invalid fake questions")?;
        questions.push(json!({"id":"sharing","question":"Should this report be shared outside the project?",
            "options":["Project only","Share externally"],"recommendation":"Project only",
            "consequences":"Project only limits distribution; external sharing requires a separate handoff.",
            "topic_key":"sharing","rationale":"The brief does not settle report distribution.",
            "evidence_refs":["brief"],"related_decision_refs":[],"reconsideration_reason":""}));
        if packet["fake_question_count"].as_u64().unwrap_or(1) >= 2 {
            questions.push(json!({"id":"retention","question":"How long should local review notes remain?",
                "options":["7 days","30 days"],"recommendation":"7 days",
                "consequences":"Shorter retention reduces stored data; longer retention aids inspection.",
                "topic_key":"review.retention","rationale":"The brief does not set a retention period.",
                "evidence_refs":["brief"],"related_decision_refs":[],"reconsideration_reason":""}));
        }
    }
    match stage {
        "critique-1" => {
            output["content"] =
                "# Independent critique 1\n\nChoose a retention period before publication.".into();
            output["findings"] = json!([{"id":"F1","severity":"substantive",
                "evidence":"Draft does not specify retention.",
                "requested_change":"Apply the operator's retention choice."}]);
            output["questions"] = if packet["schema_version"] == 2 {
                json!([])
            } else {
                json!([{"id":"retention","question":"How long should local transcripts be retained?",
                    "options":["7 days","30 days"],"recommendation":"7 days",
                    "consequences":"7 days reduces stored data; 30 days allows longer inspection."}])
            };
            output["needs_revision"] = true.into();
        }
        "critique-2" => {
            if packet["delta"].as_str().is_none_or(str::is_empty)
                || packet["decisions"].as_array().is_none_or(Vec::is_empty)
            {
                return Err("Second critique is missing delta or accepted decisions".into());
            }
            output["content"] =
                "# Independent critique 2\n\nAdd an explicit manual deletion statement.".into();
            output["findings"] = json!([{"id":"F2","severity":"minor",
                "evidence":"Automatic deletion is outside prototype scope.",
                "requested_change":"State that retention is a proposal and deletion remains manual."}]);
            output["needs_revision"] = true.into();
        }
        s if s.starts_with("revision") => {
            let decisions = packet["decisions"]
                .as_array()
                .ok_or("Revision is missing accepted decision")?;
            let answer = decisions
                .first()
                .and_then(|d| d["answer"].as_str())
                .ok_or("Revision is missing accepted decision")?;
            let mut text = format!(
                "# Revised simulated draft\n\n{brief}\n\nProposed transcript retention: {answer}.\n"
            );
            if packet["schema_version"] == 2 {
                text.push_str("Accepted decisions: ");
                text.push_str(
                    &decisions
                        .iter()
                        .map(|d| d["answer"].as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join("; "),
                );
                text.push_str(".\n");
            }
            if stage == "revision-2" {
                text.push_str("Deletion remains manual in this prototype; no automatic deletion is implemented.\n");
            }
            output["content"] = text.into();
            output["dispositions"] = json!(
                packet["open_findings"]
                    .as_array()
                    .ok_or("invalid open findings")?
                    .iter()
                    .map(|f| json!({"id":f["id"],"status":"addressed",
                    "rationale":"Updated the draft using the supplied finding and decision."}))
                    .collect::<Vec<_>>()
            );
        }
        _ => {}
    }
    if packet["mode"] == "forged-approval" {
        output["approval"] = "approved".into();
    }
    if packet["finding_contract_version"] == 2 {
        let object = output.as_object_mut().ok_or("invalid fake output")?;
        let new = object
            .remove("findings")
            .and_then(|v| v.as_array().cloned())
            .ok_or("missing fake findings")?
            .into_iter()
            .map(|mut f| {
                f.as_object_mut().unwrap().remove("id");
                f
            })
            .collect::<Vec<_>>();
        let assessments = object
            .remove("dispositions")
            .and_then(|v| v.as_array().cloned())
            .ok_or("missing fake dispositions")?
            .into_iter()
            .map(|d| {
                json!({
                "finding_ref":d["id"],"assessment":"proposed-addressed",
                "evidence":d["rationale"],"rationale":d["rationale"]})
            })
            .collect::<Vec<_>>();
        object.insert("new_findings".into(), json!(new));
        object.insert("assessments".into(), json!(assessments));
    }
    Ok(output)
}

pub fn events(packet: &Value) -> Result<Vec<Value>, String> {
    let stage = packet["stage"].as_str().unwrap_or("");
    let mut out = (1..=5)
        .map(|step| {
            json!({"type":"activity","step":step,"stage":stage,
        "message":format!("simulated work {step}/5")})
        })
        .collect::<Vec<_>>();
    out.push(json!({"type":"result","result":result(packet)?}));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v2_fake_uses_application_owned_finding_identity_contract() {
        let packet = json!({"schema_version":2,"finding_contract_version":2,"operation_id":"op",
            "stage":"critique-1","run_id":"r","provider":"fake","decisions":[],
            "context_snapshot":{"entries":[]},"finding_dispositions":{},"open_findings":[]});
        let result = result(&packet).unwrap();
        assert!(result.get("findings").is_none());
        assert!(result["new_findings"][0].get("id").is_none());
        assert!(crate::output::validate(&result, &packet).is_ok());
    }
}
