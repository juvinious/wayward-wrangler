//! Proven pinned-client result sink. No text or tool payload grants authority.
use crate::{
    codec::{canonical_sha256, sha256},
    scope_contract,
};
use serde_json::{Value, json};
/// Exact pinned-client registry identities proven by the credential-free canary fixture.
/// Registration is not permission: tools/skills/MCP, raw effects and framing remain checked.
pub fn plugins_valid(value: Option<&Value>) -> bool {
    let Some(value) = value else { return true }; // Existing mock stream omits metadata.
    let Some(entries) = value.as_array() else {
        return false;
    };
    if entries.is_empty() {
        return true;
    }
    if entries.len() != 2 {
        return false;
    }
    ["cc-plugin-agents-md", "cc-plugin-plugin-authoring"]
        .iter()
        .all(|name| {
            let expected = json!({"name":name,"path":"builtin","source":format!("{name}@builtin")});
            entries.iter().filter(|entry| **entry == expected).count() == 1
        })
}
pub const DENIED: &str = "Read,Write,Edit,Glob,Grep,Bash,Agent,SendMessage,WebFetch,WebSearch,Task,TaskOutput,TaskStop,NotebookEdit,Skill,AskUserQuestion,EnterPlanMode,ExitPlanMode";
pub const SYSTEM: &str = "You are Wayward Wrangler's conversational coordinator. Discuss naturally using only current_message and selected_context. Questions and scopes do not require the user to stop discussing. Use the supplied StructuredOutput result schema for your complete response, including assistant_text. Never claim that an application action was saved: only Waywr reports that after validation. action:null is ordinary discussion. You may suggest one question, exact-base proposal revision, or formal answer/defer sourced verbatim from the current or frozen prior user message. Formal question suggestions require exact question revision, message ID/hash and UTF-8 quote byte range. Proposal question dependencies are named by question: evidence references. A suggestion is not a human decision. Tools, repository access, commands, workers, consultation, delegation and external actions are unavailable. No text authorizes scope or budgets. Return no internal reasoning. Scope authorization is always an explicit human action. Use existing question IDs; do not invent answers, approvals or evidence. Respect the fixed limits and selected subject/base; do not change another project. Deferred question prompts must remain explicit unresolved decisions in a proposal.";
pub fn schema() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/continuous-coordinator/result-v3.schema.json"
    ))
    .expect("frozen schema")
}
pub fn inspect(e: &Value, model: &str) -> Result<(), String> {
    if e["type"] == "system" && e["subtype"] == "init" {
        if e["tools"] != json!(["StructuredOutput"])
            || e["model"] != model
            || e["permissionMode"] != "dontAsk"
        {
            return Err("Continuous coordinator capability/model drift".into());
        }
        for key in ["skills", "mcp_servers"] {
            if e[key] != json!([]) {
                return Err(format!("Unexpected native {key}"));
            }
        }
        if !plugins_valid(e.get("plugins")) {
            return Err(
                "Unexpected native plugin inventory; capability profile needs review".into(),
            );
        }
        if e["plugins"].as_array().is_some_and(|a| !a.is_empty())
            && e["claude_code_version"] != crate::claude_chat::VERSION
        {
            return Err("Builtin plugin policy requires the pinned Claude version".into());
        }
        if e.get("slash_commands").is_some_and(|v| *v != json!([])) {
            return Err("Unexpected native slash commands".into());
        }
    }
    for block in e["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(e["event"].get("content_block"))
    {
        if block["type"] == "tool_use" && block["name"] != "StructuredOutput" {
            return Err("Forbidden coordinator capability".into());
        }
    }
    Ok(())
}
pub fn raw_input(events: &[Value]) -> Result<String, String> {
    let mut text = String::new();
    let mut active = None;
    let mut count = 0;
    for e in events {
        let ev = &e["event"];
        if e["type"] != "stream_event" {
            continue;
        }
        if ev["type"] == "content_block_start" && ev["content_block"]["type"] == "tool_use" {
            if ev["content_block"]["name"] != "StructuredOutput" {
                return Err("Forbidden result channel".into());
            }
            count += 1;
            active = ev["index"].as_u64();
        }
        if ev["delta"]["type"] == "input_json_delta" {
            if active != ev["index"].as_u64() || active.is_none() {
                return Err("Unbound structured input delta".into());
            }
            text.push_str(
                ev["delta"]["partial_json"]
                    .as_str()
                    .ok_or("Missing input delta")?,
            );
            if text.len() > scope_contract::RESULT_LIMIT {
                return Err("Result exceeds 64 KiB".into());
            }
        }
        if ev["type"] == "content_block_stop" && active == ev["index"].as_u64() {
            active = None
        }
    }
    if count != 1 {
        return Err("Expected one inert result submission".into());
    }
    Ok(text)
}
pub fn provisional(events: &[Value]) -> String {
    let plain = events
        .iter()
        .filter_map(|e| crate::claude_chat::delta(e))
        .collect::<String>();
    if !plain.is_empty() {
        return plain.chars().take(32768).collect();
    }
    raw_input_partial(events)
        .map(|s| scope_contract::provisional(&s))
        .unwrap_or_default()
}
fn raw_input_partial(events: &[Value]) -> Option<String> {
    let mut input = String::new();
    let mut active = None;
    for e in events {
        let ev = &e["event"];
        if ev["type"] == "content_block_start" && ev["content_block"]["name"] == "StructuredOutput"
        {
            active = ev["index"].as_u64()
        }
        if ev["delta"]["type"] == "input_json_delta"
            && active == ev["index"].as_u64()
            && active.is_some()
        {
            input.push_str(ev["delta"]["partial_json"].as_str()?);
            if input.len() > 65536 {
                return None;
            }
        }
    }
    Some(input)
}
pub fn completion(events: &[Value], model: &str, code: Option<i32>) -> Value {
    let capability = events.iter().all(|e| inspect(e, model).is_ok());
    let inits = events
        .iter()
        .filter(|e| e["type"] == "system" && e["subtype"] == "init")
        .count();
    let finals = events
        .iter()
        .filter(|e| e["type"] == "result")
        .collect::<Vec<_>>();
    let provider_complete = events
        .iter()
        .filter(|e| e["type"] == "stream_event" && e["event"]["type"] == "message_stop")
        .count()
        == 1;
    let terminal = finals.len() == 1 && inits == 1;
    let final_ = finals.first().copied().unwrap_or(&Value::Null);
    let exit_matches =
        (final_["subtype"] == "success" && final_["is_error"] == false && code == Some(0))
            || (final_["subtype"] == "error_max_turns"
                && final_["is_error"] == true
                && code == Some(1));
    let known = terminal
        && exit_matches
        && capability
        && provider_complete
        && matches!(
            final_["subtype"].as_str(),
            Some("success" | "error_max_turns")
        )
        && matches!(
            final_["stop_reason"].as_str(),
            Some("end_turn" | "tool_use")
        );
    let parsed = raw_input(events).and_then(|raw| scope_contract::parse(&raw));
    let conflict = known
        && parsed.as_ref().is_ok_and(|v| {
            !final_["structured_output"].is_null() && *v != final_["structured_output"]
        });
    let validated = known
        && !conflict
        && parsed
            .as_ref()
            .is_ok_and(|v| v["schema_version"] == 3 && *v == final_["structured_output"])
        && final_["subtype"] == "success"
        && final_["is_error"] == false
        && code == Some(0);
    json!({"schema_version":1,"category":if !capability{"capability"}else if !known{"unavailable"}else if conflict{"integrity"}else if validated{"completed"}else{"format"},"known_completion":known,"settled":true,"capabilities_valid":capability,"exit_code":code,"action_valid":validated,"events_sha256":canonical_sha256(&json!(events)).unwrap_or_default(),"result":if validated{final_["structured_output"].clone()}else{Value::Null},"reason":if !capability{"Coordinator attempted an unapproved capability".into()}else if !known{"Completion framing or client exit status is unproven".into()}else if conflict{"Raw and named structured results disagree".into()}else if validated{String::new()}else{parsed.err().unwrap_or_else(||"Native structured result absent or inconsistent".into())},"raw_input_sha256":raw_input(events).ok().map(|s|sha256(s.as_bytes()))})
}
pub fn finish(events: &[Value], model: &str) -> Result<(Value, Value), String> {
    let proof = completion(events, model, Some(0));
    if proof["action_valid"] != true {
        return Err(proof["reason"].as_str().unwrap_or("Invalid result").into());
    }
    Ok((
        proof["result"].clone(),
        crate::claude_chat::runtime(events, model),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn events(raw: &str) -> Vec<Value> {
        vec![
            json!({"type":"system","subtype":"init","tools":["StructuredOutput"],"skills":[],"mcp_servers":[],"model":"m","permissionMode":"dontAsk"}),
            json!({"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","name":"StructuredOutput"}}}),
            json!({"type":"stream_event","event":{"index":0,"delta":{"type":"input_json_delta","partial_json":raw}}}),
            json!({"type":"stream_event","event":{"type":"message_stop"}}),
            json!({"type":"result","subtype":"success","is_error":false,"stop_reason":"tool_use","structured_output":serde_json::from_str::<Value>(raw).unwrap_or(Value::Null)}),
        ]
    }
    #[test]
    fn only_exact_proven_builtin_pair_is_allowed_without_enabling_skills_or_tools() {
        let raw =
            json!({"schema_version":3,"assistant_text":"Hello","evidence_refs":[],"action":null})
                .to_string();
        let mut e = events(&raw);
        let allowed = json!([{"name":"cc-plugin-agents-md","path":"builtin","source":"cc-plugin-agents-md@builtin"},{"name":"cc-plugin-plugin-authoring","path":"builtin","source":"cc-plugin-plugin-authoring@builtin"}]);
        e[0]["plugins"] = allowed.clone();
        e[0]["claude_code_version"] = json!(crate::claude_chat::VERSION);
        assert_eq!(completion(&e, "m", Some(0))["action_valid"], true);
        for kind in [
            "unknown",
            "spoof-path",
            "spoof-source",
            "extra-field",
            "duplicate",
            "partial",
            "not-array",
            "version",
            "skill",
            "slash",
            "tool",
            "mcp",
        ] {
            let mut changed = e.clone();
            match kind {
                "unknown" => changed[0]["plugins"][0]["name"] = json!("other"),
                "spoof-path" => changed[0]["plugins"][0]["path"] = json!("/tmp/builtin"),
                "spoof-source" => {
                    changed[0]["plugins"][0]["source"] = json!("cc-plugin-agents-md@user")
                }
                "extra-field" => changed[0]["plugins"][0]["version"] = json!("unexpected"),
                "duplicate" => changed[0]["plugins"][1] = changed[0]["plugins"][0].clone(),
                "partial" => changed[0]["plugins"]
                    .as_array_mut()
                    .unwrap()
                    .pop()
                    .map(|_| ())
                    .unwrap(),
                "not-array" => changed[0]["plugins"] = Value::Null,
                "version" => changed[0]["claude_code_version"] = json!("other"),
                "skill" => changed[0]["skills"] = json!(["plugin-authoring"]),
                "slash" => changed[0]["slash_commands"] = json!(["plugin-authoring"]),
                "tool" => changed[0]["tools"] = json!(["StructuredOutput", "Read"]),
                _ => changed[0]["mcp_servers"] = json!([{"name":"hostile"}]),
            }
            assert_eq!(
                completion(&changed, "m", Some(0))["category"],
                "capability",
                "{kind}"
            );
        }
        assert!(plugins_valid(Some(&json!([]))));
        assert!(plugins_valid(None));
        e[0]["plugins"].as_array_mut().unwrap().reverse();
        assert!(finish(&e, "m").is_ok());
    }
    #[test]
    fn strict_raw_result_refuses_normalized_duplicates_and_limits() {
        let r =
            json!({"schema_version":3,"assistant_text":"Hello","evidence_refs":[],"action":null})
                .to_string();
        assert!(finish(&events(&r), "m").is_ok());
        let bad = r.replace(
            "\"schema_version\":3",
            "\"schema_version\":3,\"schema_version\":3",
        );
        assert!(finish(&events(&bad), "m").is_err());
        assert_eq!(
            completion(&events(&bad), "m", Some(0))["category"],
            "format"
        );
        let mut e = events(&r);
        e[1]["event"]["content_block"]["name"] = json!("Bash");
        assert_eq!(completion(&e, "m", Some(0))["category"], "capability");
        let oversized = r.replace("Hello", &"x".repeat(32769));
        assert!(finish(&events(&oversized), "m").is_err());
    }
    #[test]
    fn missing_terminal_and_client_error_are_not_recoverable_format() {
        let r =
            json!({"schema_version":3,"assistant_text":"Hello","evidence_refs":[],"action":null})
                .to_string();
        let mut e = events(&r);
        e.pop();
        assert_eq!(completion(&e, "m", Some(0))["known_completion"], false);
    }

    #[test]
    fn terminal_proof_and_matching_exit_are_required_before_action_admission() {
        let raw =
            json!({"schema_version":3,"assistant_text":"Hello","evidence_refs":[],"action":null})
                .to_string();
        let original = events(&raw);
        for kind in [
            "no-init",
            "no-provider-stop",
            "duplicate-init",
            "duplicate-final",
            "duplicate-provider-stop",
        ] {
            let mut e = original.clone();
            match kind {
                "no-init" => {
                    e.remove(0);
                }
                "no-provider-stop" => {
                    e.remove(3);
                }
                "duplicate-init" => e.push(e[0].clone()),
                "duplicate-final" => e.push(e[4].clone()),
                _ => e.push(e[3].clone()),
            };
            let p = completion(&e, "m", Some(0));
            assert_eq!(p["action_valid"], false, "{kind}");
            assert_eq!(p["known_completion"], false, "{kind}");
            assert_ne!(p["category"], "format", "{kind}");
        }
        for exit in [None, Some(1), Some(17)] {
            let p = completion(&original, "m", exit);
            assert_eq!(p["action_valid"], false);
            assert_eq!(p["category"], "unavailable");
        }
        let mut contradiction = original;
        contradiction[4]["is_error"] = json!(true);
        assert_eq!(
            completion(&contradiction, "m", Some(0))["known_completion"],
            false
        );
    }
    #[test]
    fn result_conflict_is_integrity_but_known_max_turns_format_is_recoverable() {
        let raw =
            json!({"schema_version":3,"assistant_text":"Hello","evidence_refs":[],"action":null})
                .to_string();
        let mut e = events(&raw);
        e[4]["structured_output"]["assistant_text"] = json!("Different");
        let p = completion(&e, "m", Some(0));
        assert_eq!(p["category"], "integrity");
        assert_eq!(p["action_valid"], false);
        let mut failed = events("{");
        failed[4]["subtype"] = json!("error_max_turns");
        failed[4]["is_error"] = json!(true);
        let p = completion(&failed, "m", Some(1));
        assert_eq!(p["category"], "format");
        assert_eq!(p["known_completion"], true);
        assert_eq!(p["action_valid"], false);
        assert_eq!(completion(&failed, "m", Some(0))["category"], "unavailable");
    }
}
