//! Frozen text-only Claude/Codex CLI contracts. Rust policy never owns credentials.
use crate::codec::{canonical, canonical_sha256};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::{collections::BTreeMap, path::Path};

// Historical proof metadata; admission uses capabilities, not these versions.
pub const CLAUDE_VERSION: &str = "2.1.296";
pub const CODEX_VERSION: &str = "0.162.1";
pub const DISABLED: &[&str] = &[
    "hooks",
    "plugins",
    "apps",
    "shell_tool",
    "shell_snapshot",
    "multi_agent",
    "multi_agent_v2",
    "browser_use",
    "browser_use_external",
    "computer_use",
    "image_generation",
    "view_image",
    "memories",
    "goals",
    "tool_suggest",
    "code_mode",
    "code_mode_host",
    "unified_exec",
    "request_permissions_tool",
];
pub const ENV_KEYS: &[&str] = &[
    "PATH",
    "HOME",
    "LANG",
    "LC_ALL",
    "TERM",
    "COLORTERM",
    "CODEX_HOME",
    "CLAUDE_CONFIG_DIR",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
];

pub fn verify_role(role: &Value, version_output: &str) -> Result<String, String> {
    let provider = role["provider"]
        .as_str()
        .ok_or("Real worker requires provider, explicit model and reasoning")?;
    if !matches!(provider,"claude"|"codex"){
        return Err("Real worker requires provider, explicit model and reasoning".into());
    }
    if role["model"].as_str().is_none_or(str::is_empty)
        || role["reasoning"].as_str().is_none_or(str::is_empty)
    {
        return Err("Real worker requires provider, explicit model and reasoning".into());
    }
    let binary = role["binary"]
        .as_str()
        .ok_or("Worker binary must be an existing absolute path")?;
    if !Path::new(binary).is_absolute() || !Path::new(binary).is_file() {
        return Err("Worker binary must be an existing absolute path".into());
    }
    Ok(version_output.trim().to_owned())
}

pub fn environment(
    provider: &str,
    source: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut selected = source
        .iter()
        .filter(|(k, _)| ENV_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<BTreeMap<_, _>>();
    selected.insert("PYTHONUNBUFFERED".into(), "1".into());
    selected.insert("DISABLE_AUTOUPDATER".into(), "1".into());
    match provider {
        "claude" => {
            selected.insert("CLAUDE_CODE_SAFE_MODE".into(), "1".into());
        }
        "codex" => {}
        _ => return Err("unsupported provider".into()),
    };
    Ok(selected)
}

/// Version-1 instruction bytes were frozen from the installed r9 baseline.
/// The packet and response identity are encoded afresh from one frozen packet.
pub fn prompt(packet: &Value) -> Result<String, String> {
    let instruction = if packet["schema_version"] == 2 {
        let product = packet["prompt_contract"] == "product-spec-v1";
        let finding2 = packet["finding_contract_version"] == 2;
        match (product, finding2, packet["stage"] == "draft") {
            (false, false, _) => include_str!("../resources/prompt-contract-v1/context.txt"),
            (false, true, _) => {
                include_str!("../resources/prompt-contract-v1/context-findings-v2.txt")
            }
            (true, false, _) => include_str!("../resources/prompt-contract-v1/product-spec.txt"),
            (true, true, true) => {
                include_str!("../resources/prompt-contract-v1/product-spec-findings-v2-draft.txt")
            }
            (true, true, false) => {
                include_str!("../resources/prompt-contract-v1/product-spec-findings-v2-later.txt")
            }
        }
    } else {
        include_str!("../resources/prompt-contract-v1/legacy.txt")
    };
    let operation_id = packet["operation_id"]
        .as_str()
        .ok_or("missing operation ID")?;
    let hash = canonical_sha256(packet).map_err(|e| e.to_string())?;
    let suffix = json!({"response_identity":{"operation_id":operation_id,"packet_sha256":hash},
        "context_packet":packet});
    let suffix = canonical(&suffix).map_err(|e| e.to_string())?;
    Ok(format!(
        "{instruction}{}",
        String::from_utf8(suffix).map_err(|e| e.to_string())?
    ))
}

fn obj(properties: Value) -> Value {
    let keys = properties
        .as_object()
        .expect("static schema properties")
        .keys()
        .cloned()
        .map(Value::String)
        .collect::<Vec<_>>();
    json!({"type":"object","properties":properties,"required":keys,"additionalProperties":false})
}
fn obj_req(properties: Value, required: &[&str]) -> Value {
    let mut value = obj(properties);
    value["required"] = json!(required);
    value
}
fn array(item: Value) -> Value {
    json!({"type":"array","items":item})
}
fn string() -> Value {
    json!({"type":"string"})
}

pub fn schema(packet: &Value) -> Result<Value, String> {
    let operation_id = packet["operation_id"]
        .as_str()
        .ok_or("missing operation ID")?;
    let packet_hash = canonical_sha256(packet).map_err(|e| e.to_string())?;
    let mut result = obj(json!({
        "schema_version":{"type":"integer","enum":[packet["schema_version"].as_u64().unwrap_or(1)]},
        "operation_id":{"type":"string","enum":[operation_id]},
        "packet_sha256":{"type":"string","enum":[packet_hash]},
        "content":string(),
        "findings":array(obj_req(json!({"id":string(),"severity":{"type":"string","enum":["minor","substantive"]},
            "evidence":string(),"requested_change":string()}), &["id","severity","evidence","requested_change"])),
        "dispositions":array(obj_req(json!({"id":string(),"status":{"type":"string","enum":["addressed"]},
            "rationale":string()}), &["id","status","rationale"])),
        "questions":array(obj_req(json!({"id":string(),"question":string(),"options":array(string()),
            "recommendation":string(),"consequences":string()}), &["id","question","options","recommendation","consequences"])),
        "needs_revision":{"type":"boolean"}
    }));
    let properties = result["properties"]
        .as_object_mut()
        .ok_or("invalid schema")?;
    if packet["schema_version"] == 2 {
        properties.insert(
            "questions".into(),
            array(obj_req(
                json!({"id":string(),"question":string(),
            "options":array(string()),"recommendation":string(),"consequences":string(),
            "topic_key":string(),"rationale":string(),"evidence_refs":array(string()),
            "related_decision_refs":array(string()),"reconsideration_reason":string()}),
                &[
                    "id",
                    "question",
                    "options",
                    "recommendation",
                    "consequences",
                    "topic_key",
                    "rationale",
                    "evidence_refs",
                    "related_decision_refs",
                    "reconsideration_reason",
                ],
            )),
        );
    }
    if packet["finding_contract_version"] == 2 {
        properties.remove("findings");
        properties.remove("dispositions");
        properties.insert(
            "new_findings".into(),
            array(obj_req(
                json!({"severity":{"type":"string","enum":["minor","substantive"]},
            "evidence":string(),"requested_change":string()}),
                &["severity", "evidence", "requested_change"],
            )),
        );
        properties.insert(
            "assessments".into(),
            array(obj_req(
                json!({"finding_ref":string(),
            "assessment":{"type":"string","enum":["open","proposed-addressed"]},
            "evidence":string(),"rationale":string()}),
                &["finding_ref", "assessment", "evidence", "rationale"],
            )),
        );
    }
    // Python's required list follows construction/insertion order; JSON schema
    // array order does not affect validation, but its frozen bytes do matter.
    let required = if packet["finding_contract_version"] == 2 {
        [
            "schema_version",
            "operation_id",
            "packet_sha256",
            "content",
            "questions",
            "needs_revision",
            "new_findings",
            "assessments",
        ]
        .as_slice()
    } else {
        [
            "schema_version",
            "operation_id",
            "packet_sha256",
            "content",
            "findings",
            "dispositions",
            "questions",
            "needs_revision",
        ]
        .as_slice()
    };
    result["required"] = json!(required);
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct Command {
    pub argv: Vec<String>,
    pub restrictions: Value,
}

/// Construct only arguments. Caller must version-check an absolute binary and
/// persist the frozen packet/schema/prompt before granting any execution permit.
pub fn command(packet: &Value, directory: &Path) -> Result<Command, String> {
    let role = &packet["execution"];
    let provider = packet["provider"].as_str().ok_or("missing provider")?;
    if role["provider"] != packet["provider"] || role["model"] != packet["model"] {
        return Err("Execution identity mismatch".into());
    }
    let binary = role["binary"].as_str().ok_or("missing binary")?;
    if !Path::new(binary).is_absolute() {
        return Err("Worker binary must be an existing absolute path".into());
    }
    let model = role["model"].as_str().ok_or("missing model")?;
    let reasoning = role["reasoning"].as_str().ok_or("missing reasoning")?;
    let (argv, restrictions) = if provider == "claude" {
        let mut argv = vec![
            binary,
            "--print",
            "--safe-mode",
            "--setting-sources",
            "",
            "--strict-mcp-config",
            "--mcp-config",
            "{\"mcpServers\":{}}",
            "--tools",
            "",
            "--permission-mode",
            "dontAsk",
            "--settings",
            "{\"disableAllHooks\":true}",
            "--model",
            model,
            "--effort",
            reasoning,
            "--no-session-persistence",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--json-schema",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        argv.push(
            String::from_utf8(
                crate::codec::canonical(&schema(packet)?).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
        );
        (
            argv,
            json!({"tools":"empty built-in tool list","permission_mode":"dontAsk",
            "hooks":"safe-mode + disableAllHooks","mcp":"strict empty","session_history":"disabled"}),
        )
    } else if provider == "codex" {
        let schema_path = directory.join("output-schema.json");
        let result_path = directory.join("native-final.json");
        let mut argv = vec![
            binary.to_owned(),
            "--no-daemon".into(),
            "--ask-for-approval".into(),
            "never".into(),
            "exec".into(),
            "--ignore-user-config".into(),
            "--strict-config".into(),
            "--skip-git-repo-check".into(),
            "--ephemeral".into(),
            "--sandbox".into(),
            "read-only".into(),
            "--model".into(),
            model.into(),
            "--json".into(),
            "--output-schema".into(),
            schema_path.to_string_lossy().into_owned(),
            "--output-last-message".into(),
            result_path.to_string_lossy().into_owned(),
            "-c".into(),
            format!("model_reasoning_effort={}", json!(reasoning)),
            "-c".into(),
            "web_search=\"disabled\"".into(),
            "-c".into(),
            "project_doc_max_bytes=0".into(),
            "-c".into(),
            "skills.include_instructions=false".into(),
            "-c".into(),
            "skills.bundled.enabled=false".into(),
            "-c".into(),
            "tools.update_plan.enabled=false".into(),
            "-c".into(),
            "tools.experimental_request_user_input.enabled=false".into(),
        ];
        for feature in DISABLED {
            argv.push("--disable".into());
            argv.push((*feature).into());
        }
        argv.push("-".into());
        (
            argv,
            json!({"sandbox":"read-only","approval_policy":"never (deny escalation)",
            "features_disabled":DISABLED,"web_search":"disabled",
            "user_config":"ignored; existing authentication retained",
            "remaining_tool_caveat":"apply_patch may remain advertised, but read-only sandbox denies writes; any observed tool event fails the stage"}),
        )
    } else {
        return Err("Real worker requires provider, explicit model and reasoning".into());
    };
    Ok(Command { argv, restrictions })
}

pub fn inspect_event(provider: &str, event: &Value) -> Result<(), String> {
    if provider == "claude" {
        if event["type"] == "system" && event["subtype"] == "init" {
            if event["mcp_servers"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
                || event["tools"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v != "StructuredOutput"))
            {
                return Err("Claude advertised tools/MCP outside text-only scope".into());
            }
            if !event["permissionMode"].is_null() && event["permissionMode"] != "dontAsk" {
                return Err("Claude permission mode mismatch".into());
            }
        }
        if let Some(content) = event["message"]["content"].as_array()
            && content
                .iter()
                .any(|v| v["type"] == "tool_use" && v["name"] != "StructuredOutput")
        {
            return Err("Claude attempted a tool outside text-only scope".into());
        }
    } else if provider == "codex" {
        if matches!(
            event["item"]["type"].as_str(),
            Some(
                "command_execution"
                    | "file_change"
                    | "mcp_tool_call"
                    | "web_search"
                    | "collab_tool_call"
            )
        ) {
            return Err("Codex attempted a tool outside text-only scope".into());
        }
    } else {
        return Err("unsupported provider".into());
    }
    Ok(())
}

/// Interpret a captured native stream and final file. The caller retains raw
/// events and commits the runtime record immutably before accepting the result.
pub fn finish(
    provider: &str,
    events: &[Value],
    native_final: Option<&[u8]>,
) -> Result<(Value, Value), String> {
    for event in events {
        inspect_event(provider, event)?;
    }
    if provider == "claude" {
        let finals = events
            .iter()
            .filter(|e| e["type"] == "result")
            .collect::<Vec<_>>();
        if finals.len() != 1 || finals[0]["is_error"] == true || finals[0]["subtype"] != "success" {
            return Err("Claude did not return one successful structured result".into());
        }
        let result = &finals[0]["structured_output"];
        if !result.is_object() {
            return Err("Claude structured_output is absent; no retry".into());
        }
        let init = events
            .iter()
            .find(|e| e["type"] == "system" && e["subtype"] == "init");
        let runtime = json!({"resolved_model":init.map(|e|e["model"].clone()).unwrap_or(Value::Null),
            "model_usage":finals[0].get("modelUsage").cloned().unwrap_or_else(||json!({})),
            "reported_tools":init.map(|e|e["tools"].clone()).unwrap_or(Value::Null),
            "reported_mcp":init.map(|e|e["mcp_servers"].clone()).unwrap_or(Value::Null),
            "reported_permission_mode":init.map(|e|e["permissionMode"].clone()).unwrap_or(Value::Null),
            "num_turns":finals[0]["num_turns"],
            "partial_events":events.iter().filter(|e|e["type"]=="stream_event").count()});
        return Ok((result.clone(), runtime));
    }
    if provider == "codex" {
        let diagnostics = events
            .iter()
            .filter(|e| e["item"]["type"] == "error")
            .map(|e| e["item"]["message"].as_str().unwrap_or("").to_owned())
            .collect::<Vec<_>>();
        let allowed = "Code Mode is unavailable because code-mode host is disabled. Code mode will fail closed; enable `features.code_mode_host` and install `codex-code-mode-host`.";
        if diagnostics.iter().any(|d| d != allowed) {
            return Err("Codex reported an unexpected item error; no retry".into());
        }
        if events
            .iter()
            .any(|e| matches!(e["type"].as_str(), Some("turn.failed" | "error")))
            || events
                .iter()
                .filter(|e| e["type"] == "turn.completed")
                .count()
                != 1
        {
            return Err("Codex did not finish exactly one turn; no retry".into());
        }
        let result: Value =
            serde_json::from_slice(native_final.ok_or("Codex final JSON file missing; no retry")?)
                .map_err(|_| "Codex final JSON is invalid; no retry".to_owned())?;
        let event_types: BTreeSet<&str> = events
            .iter()
            .map(|e| e["type"].as_str().unwrap_or(""))
            .collect();
        let runtime = json!({"resolved_model":"not independently reported by exec JSONL; explicit launch model recorded",
            "thread_id":events.iter().find(|e|e["type"]=="thread.started")
                .map(|e|e["thread_id"].clone()).unwrap_or(Value::Null),
            "event_types":event_types,"diagnostics":diagnostics,
            "usage":events.iter().find(|e|e["type"]=="turn.completed")
                .map(|e|e["usage"].clone()).unwrap_or(Value::Null)});
        return Ok((result, runtime));
    }
    Err("unsupported provider".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_visible_tool_use() {
        assert!(
            inspect_event(
                "claude",
                &json!({"message":{"content":[{"type":"tool_use","name":"Read"}]}})
            )
            .is_err()
        );
        assert!(inspect_event("codex", &json!({"item":{"type":"file_change"}})).is_err());
    }
}
