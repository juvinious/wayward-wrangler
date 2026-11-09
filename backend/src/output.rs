//! Frozen output interpretation boundary. A worker cannot add control or approval fields.
use crate::{codec::canonical_sha256, context_policy};
use serde_json::Value;
use std::collections::BTreeSet;

fn exact(value: &Value, required: &[&str]) -> bool {
    value.as_object().is_some_and(|map| {
        map.keys().map(String::as_str).collect::<BTreeSet<_>>()
            == required.iter().copied().collect::<BTreeSet<_>>()
    })
}
fn nonempty(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.trim().is_empty())
}
/// Citation recognition is provenance, not permission or factual verification.
/// Ordinary v1 discussion can retain a useful reply with unrecognized references;
/// actions still require every dependency to be in the frozen allowlist.
pub(crate) fn conversation_references(packet: &Value, refs: &[String]) -> Value {
    let (recognized, unrecognized): (Vec<_>, Vec<_>) = refs.iter().cloned().partition(|reference| {
        packet["selected_context"]["evidence_allowlist"].as_array()
            .is_some_and(|allowed| allowed.contains(&serde_json::json!(reference)))
    });
    serde_json::json!({"recognized":recognized,"unrecognized":unrecognized,
        "meaning":"Recognized references were supplied; unrecognized references are unverified model citations, never dependencies or authority"})
}
pub fn validate(result: &Value, packet: &Value) -> Result<(), String> {
    if packet["development"]==true && packet["operation_kind"]!="conversation"{return crate::development::validate_result(result,packet)}
    if packet["operation_kind"]=="conversation"{
        if matches!(packet["execution"]["profile"].as_str(),Some("claude-proposal-v1"|"claude-proposal-v2"|"codex-development-v1"|"development-profiles-v1")){
            let parsed=crate::scope_contract::validate(result)?;
            if let Some(crate::scope_contract::Action::RetrieveEvidence{requests})=&parsed.action{
                if packet["evidence_index"]["version"]!=1||requests.is_empty(){return Err("Discussion packet grants no evidence retrieval".into())}
                crate::development_context::validate_requests(&serde_json::to_value(requests).map_err(|e|e.to_string())?)?;
            }
            if matches!(parsed.action,Some(crate::scope_contract::Action::ProposeSteering{..}))&&packet["selected_context"]["interaction"]["version"]!=1{return Err("Steering requires a newly submitted conversational context".into())}
            if parsed.schema_version!=if matches!(packet["execution"]["profile"].as_str(),Some("claude-proposal-v2"|"codex-development-v1"|"development-profiles-v1")){3}else{2}{return Err("Proposal profile requires response version 2".into())}
            let references=conversation_references(packet,&parsed.evidence_refs);
            let discussion=parsed.action.is_none()&&packet["selected_context"]["interaction"]["version"]==1;
            if !discussion&&references["unrecognized"].as_array().is_some_and(|refs|!refs.is_empty()){return Err("Evidence reference outside frozen packet".into())}
            if let Some(crate::scope_contract::Action::UpsertProposal{base,..})=parsed.action{if serde_json::to_value(base).map_err(|e|e.to_string())?!=packet["selected_context"]["scope"]["base"]{return Err("Proposal base differs from frozen packet".into())}}
            return Ok(());
        }
        let parsed=crate::native_contract::CoordinatorTurn::validate(result).map_err(|e|e.to_string())?;
        if parsed.action.is_some(){return Err("Conversation-only result cannot request application actions".into());}
        return Ok(());
    }
    if packet["profile"]==crate::static_files::PROFILE{return crate::static_files::validate_result(result,packet);}
    let hash = canonical_sha256(packet).map_err(|e| e.to_string())?;
    if packet["operation_kind"] == "coordinator" {
        if !exact(
            result,
            &[
                "schema_version",
                "operation_id",
                "packet_sha256",
                "proposal",
            ],
        ) || result["schema_version"] != 1
        {
            return Err("Invalid coordinator output envelope".into());
        }
        if result["operation_id"] != packet["operation_id"] || result["packet_sha256"] != hash {
            return Err("Coordinator output refers to different frozen input".into());
        }
        let proposal = &result["proposal"];
        if !exact(
            proposal,
            &[
                "schema_version",
                "action",
                "task_id",
                "task_version",
                "message_sha256",
                "brief",
            ],
        ) {
            return Err("Invalid coordinator proposal contract".into());
        }
        if proposal["schema_version"] != 1
            || proposal["action"] != "propose_task"
            || ["task_id", "task_version", "message_sha256"]
                .iter()
                .any(|key| proposal[*key] != packet[*key])
        {
            return Err("Coordinator proposal references different task input".into());
        }
        if !nonempty(&proposal["brief"]) || proposal["brief"].as_str().unwrap().len() > 32768 {
            return Err("Coordinator brief is missing or oversized".into());
        }
        return Ok(());
    }
    let v2 = packet["finding_contract_version"] == 2;
    let fields = if v2 {
        vec![
            "schema_version",
            "operation_id",
            "packet_sha256",
            "content",
            "questions",
            "needs_revision",
            "new_findings",
            "assessments",
        ]
    } else {
        vec![
            "schema_version",
            "operation_id",
            "packet_sha256",
            "content",
            "questions",
            "needs_revision",
            "findings",
            "dispositions",
        ]
    };
    if !exact(result, &fields)
        || result["schema_version"]
            != packet
                .get("schema_version")
                .cloned()
                .unwrap_or(Value::from(1))
    {
        return Err("Invalid output schema; worker control/approval fields are forbidden".into());
    }
    if result["operation_id"] != packet["operation_id"] || result["packet_sha256"] != hash {
        return Err("Result refers to different input".into());
    }
    if !nonempty(&result["content"]) {
        return Err("Empty result".into());
    }
    if !result["needs_revision"].is_boolean() {
        return Err("Invalid needs_revision".into());
    }
    for key in if v2 {
        vec!["new_findings", "assessments", "questions"]
    } else {
        vec!["findings", "dispositions", "questions"]
    } {
        if !result[key].is_array() {
            return Err(format!("Invalid {key}"));
        }
    }
    let questions = result["questions"].as_array().unwrap();
    if packet["schema_version"].as_u64().unwrap_or(1) == 1
        && packet["provider"] != "fake"
        && !questions.is_empty()
    {
        return Err(
            "Unexpected worker question: this trial uses its controller-owned substantive gate"
                .into(),
        );
    }
    if packet["schema_version"] == 2 {
        context_policy::validate_questions(result, packet)?;
    } else {
        for q in questions {
            if !exact(
                q,
                &[
                    "id",
                    "question",
                    "options",
                    "recommendation",
                    "consequences",
                ],
            ) {
                return Err("Invalid question".into());
            }
            if q["options"].as_array().is_none_or(|a| a.len() < 2) {
                return Err("Question requires substantive options".into());
            }
        }
    }
    if v2 {
        for f in result["new_findings"].as_array().unwrap() {
            if !exact(f, &["severity", "evidence", "requested_change"])
                || !matches!(f["severity"].as_str(), Some("minor" | "substantive"))
                || !nonempty(&f["evidence"])
                || !nonempty(&f["requested_change"])
            {
                return Err("Invalid new finding".into());
            }
        }
        for a in result["assessments"].as_array().unwrap() {
            if !exact(a, &["finding_ref", "assessment", "evidence", "rationale"])
                || !matches!(
                    a["assessment"].as_str(),
                    Some("open" | "proposed-addressed")
                )
                || !nonempty(&a["finding_ref"])
                || !nonempty(&a["evidence"])
                || !nonempty(&a["rationale"])
            {
                return Err("Invalid finding assessment".into());
            }
        }
    } else {
        for f in result["findings"].as_array().unwrap() {
            if !exact(f, &["id", "severity", "evidence", "requested_change"]) {
                return Err("Invalid finding".into());
            }
        }
        for d in result["dispositions"].as_array().unwrap() {
            if !exact(d, &["id", "status", "rationale"]) || d["status"] != "addressed" {
                return Err("Workers may only propose addressed findings".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod conversation_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn ordinary_reply_citations_do_not_become_action_dependencies() {
        let packet=json!({"operation_kind":"conversation","execution":{"profile":"development-profiles-v1"},"selected_context":{"interaction":{"version":1},"evidence_allowlist":["message:human"],"scope":{"base":null}}});
        let reply=json!({"schema_version":3,"assistant_text":"Here is the recap.","evidence_refs":["message:human","workflow","checks","question:missing"],"action":null});
        validate(&reply,&packet).unwrap();
        let refs=conversation_references(&packet,&crate::scope_contract::validate(&reply).unwrap().evidence_refs);
        assert_eq!(refs["recognized"],json!(["message:human"]));
        assert_eq!(refs["unrecognized"],json!(["workflow","checks","question:missing"]));
        let mut action=reply.clone();action["action"]=json!({"kind":"ask_question","payload":{"prompt":"Which format?","rationale":"Missing choice","options":[]}});
        assert!(validate(&action,&packet).unwrap_err().contains("outside frozen packet"));
        let mut legacy=packet.clone();legacy["selected_context"]["interaction"]=Value::Null;
        assert!(validate(&reply,&legacy).is_err());
        let mut malformed=reply.clone();malformed["action"]=json!({"kind":"authorize_scope","payload":{}});
        assert!(validate(&malformed,&packet).is_err());
        malformed=reply.clone();malformed["evidence_refs"]=json!([{"not":"a string"}]);assert!(validate(&malformed,&packet).is_err());
    }
    #[test]
    fn ordinary_reply_finishes_through_both_native_adapters_without_calls() {
        let root=tempfile::tempdir().unwrap();
        let reply=json!({"schema_version":3,"assistant_text":"Atomic replacement prevents partial file replacement; it is not a claim of power-loss durability.","evidence_refs":["workflow"],"action":null});
        for provider in ["codex","claude"] {
            let directory=root.path().join(provider);std::fs::create_dir_all(&directory).unwrap();
            let packet=json!({"development":true,"operation_kind":"conversation","provider":provider,"model":"fixture-model","execution":{"profile":"development-profiles-v1"},"selected_context":{"interaction":{"version":1},"evidence_allowlist":["message:human"]}});
            let raw=reply.to_string();
            let events=if provider=="codex" {
                std::fs::write(directory.join("native-final.json"),&raw).unwrap();
                vec![json!({"type":"item.completed","item":{"type":"agent_message","text":raw}}),json!({"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}})]
            } else {
                vec![json!({"type":"system","subtype":"init","model":"fixture-model","permissionMode":"dontAsk","tools":["StructuredOutput"]}),json!({"type":"result","subtype":"success","is_error":false,"structured_output":reply})]
            };
            let (completed,_)=crate::development::finish_native(&events,&directory,&packet,Some(0)).unwrap();
            assert_eq!(completed,reply); // Retain the exact native result, never rewrite it.
            assert_eq!(crate::development::result_schema(&packet)["properties"]["evidence_refs"]["items"]["enum"],json!(["message:human"]));
            if provider=="codex"{
                let mut execution=events.clone();execution.push(json!({"type":"item.completed","item":{"type":"command_execution"}}));
                assert!(crate::development::inspect_native(execution.last().unwrap(),&packet).is_err());
            }
        }
    }
}
