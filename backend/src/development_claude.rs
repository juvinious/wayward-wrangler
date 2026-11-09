//! Claude transport for frozen development packets; policy/receipts remain application-owned.
use crate::{
    codec::{canonical_sha256, sha256},
    development,
    execution::{write, write_bytes},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str().ok_or_else(|| format!("Missing {k}"))
}
pub fn settings(packet: &Value) -> Result<Value, String> {
    let worker = packet["operation_kind"] == "development-worker";
    let mut settings = json!({"disableAllHooks":true,"fallbackModel":[],"switchModelsOnFlag":false,
        "autoMemoryEnabled":false,"availableModels":[packet["model"]],"enabledPlugins":{},
        "permissions":{"defaultMode":"dontAsk","allow":[],"deny":["Agent","Task","SendMessage","WebFetch","WebSearch","Skill","AskUserQuestion","EnterPlanMode","ExitPlanMode","NotebookEdit","TaskOutput","TaskStop"]}});
    if worker {
        // Validate the exact owned worktree/ref, as for the Codex adapter.
        let roots = development::worker_write_roots(packet)?;
        let tree = PathBuf::from(string(packet, "worktree")?);
        let common = PathBuf::from(string(packet, "git_common_dir")?);
        let mut deny = vec![
            common.join("config"),
            common.join("hooks"),
            common.join("HEAD"),
            common.join("index"),
            common.join("refs/heads/main"),
            common.join("logs/refs/heads/main"),
            common
                .join("refs/heads")
                .join(format!("waywr/{}/integration", string(packet, "run_id")?)),
            common
                .join("logs/refs/heads")
                .join(format!("waywr/{}/integration", string(packet, "run_id")?)),
        ];
        if let Some(root) = common.parent() {
            for path in [
                ".waywr/state",
                ".waywr/history",
                ".waywr/coordinator.json",
                ".waywr/project.toml",
                ".waywr/release.json",
            ] {
                deny.push(root.join(path));
            }
        }
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
        deny.extend([
            home.join(".codex"),
            home.join(".claude"),
            home.join(".claude.json"),
            home.join(".ssh"),
            home.join(".aws"),
        ]);
        let mut writes = roots;
        writes.push(tree.clone());
        writes.push(home.join(".cargo"));
        settings["sandbox"] = json!({"enabled":true,"failIfUnavailable":true,"autoAllowBashIfSandboxed":true,
            "allowUnsandboxedCommands":false,"excludedCommands":[],"filesystem":{"allowWrite":writes,"denyWrite":deny},
            "network":{"allowedDomains":if packet["execution"]["dependency_network"]==true{vec!["*"]}else{vec![]},"allowAllUnixSockets":false,"allowLocalBinding":false}});
        let pattern = format!("//{}/**", tree.to_string_lossy().trim_start_matches('/'));
        settings["permissions"]["allow"] = json!([
            format!("Read({pattern})"),
            format!("Edit({pattern})"),
            format!("Write({pattern})"),
            format!("Glob({pattern})"),
            format!("Grep({pattern})"),
            "Bash"
        ]);
        let denies = settings["permissions"]["deny"].as_array_mut().unwrap();
        for path in deny {
            let path = path.to_string_lossy();
            for tool in ["Read", "Edit", "Write"] {
                denies.push(json!(format!(
                    "{tool}(//{}/**)",
                    path.trim_start_matches('/')
                )));
            }
        }
    }
    Ok(settings)
}
pub fn prepare(
    directory: &Path,
    packet: &Value,
    instruction: &str,
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    let p = &packet["execution"];
    development::preflight(p)?;
    let schema = development::result_schema(packet);
    let settings = settings(packet)?;
    // System instructions are supplied once through --system-prompt.
    let prompt = format!("Frozen application packet:\n{}", packet);
    write_bytes(&directory.join("prompt.txt"), prompt.as_bytes(), true)?;
    write(&directory.join("output-schema.json"), &schema, true)?;
    fs::create_dir_all(directory.join("client-scratch")).map_err(|e| e.to_string())?;
    let worker = packet["operation_kind"] == "development-worker";
    let mut args = vec![
        crate::provider_selection::native_binary(p)?.into(),
        "--print".into(),
        "--safe-mode".into(),
        "--restricted".into(),
        "--disable-slash-commands".into(),
        "--setting-sources".into(),
        "".into(),
        "--strict-mcp-config".into(),
        "--mcp-config".into(),
        "{\"mcpServers\":{}}".into(),
        "--tools".into(),
        if worker {
            "Read,Edit,Write,Glob,Grep,Bash"
        } else {
            ""
        }
        .into(),
        "--permission-mode".into(),
        "dontAsk".into(),
        "--permission-prompts".into(),
        "none".into(),
        "--settings".into(),
        settings.to_string(),
        "--model".into(),
        string(p, "model")?.into(),
        "--no-session-persistence".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--include-partial-messages".into(),
        "--json-schema".into(),
        schema.to_string(),
        "--system-prompt".into(),
        instruction.into(),
    ];
    if p["effort"] != "provider_default" {
        args.extend(["--effort".into(), string(p, "effort")?.into()]);
    }
    let home = std::env::var("HOME").map_err(|_| "Authentication home unavailable")?;
    let config = std::env::var("CLAUDE_CONFIG_DIR").unwrap_or_else(|_| format!("{home}/.claude"));
    let env_profile = json!({"auth_home":home,"auth_config_dir":config,"mode":"native"});
    let mut env = crate::claude_chat::environment(&env_profile)?;
    env.insert(
        "PATH".into(),
        format!("{home}/.cargo/bin:{home}/.local/bin:/usr/local/bin:/usr/bin:/bin"),
    );
    env.insert("PYTHONDONTWRITEBYTECODE".into(), "1".into());
    env.insert("CLAUDE_CODE_DISABLE_AUTO_MEMORY".into(), "1".into());
    let cwd = if worker {
        PathBuf::from(string(packet, "worktree")?)
    } else {
        directory.join("client-scratch")
    };
    env.insert(
        "CARGO_TARGET_DIR".into(),
        cwd.join("target").to_string_lossy().into_owned(),
    );
    write(
        &directory.join("execution.json"),
        &json!({"schema_version":1,"provider":"claude","model":p["model"],"effort":p["effort"],
        "selection":p["selection"],"argv":args,"client_cwd":cwd,"permission":"frozen project tool rules; fail-closed Bash sandbox; trusted native client", "prompt_sha256":sha256(prompt.as_bytes())}),
        true,
    )?;
    Ok((args, env))
}
pub fn inspect(e: &Value, packet: &Value) -> Result<(), String> {
    let worker = packet["operation_kind"] == "development-worker";
    if e["type"] == "system" && e["subtype"] == "init" {
        if e["model"] != packet["model"] || e["permissionMode"] != "dontAsk" {
            return Err("Claude model/permission differs from frozen request; aliases need an explicit exact model ID".into());
        }
        for field in ["mcp_servers", "skills", "slash_commands"] {
            if e.get(field).is_some_and(|v| *v != json!([])) {
                return Err(format!("Unexpected Claude {field}"));
            }
        }
        if !crate::continuous_adapter::plugins_valid(e.get("plugins")) {
            return Err("Unexpected Claude plugins".into());
        }
        let allowed = if worker {
            vec![
                "Read",
                "Edit",
                "Write",
                "Glob",
                "Grep",
                "Bash",
                "StructuredOutput",
            ]
        } else {
            vec!["StructuredOutput"]
        };
        if e["tools"].as_array().is_none_or(|tools| {
            tools
                .iter()
                .any(|v| !allowed.contains(&v.as_str().unwrap_or("")))
        }) {
            return Err("Claude tool inventory differs from frozen role".into());
        }
    }
    for block in e["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(e["event"].get("content_block"))
    {
        if block["type"] == "tool_use" {
            let name = block["name"].as_str().unwrap_or("");
            if name != "StructuredOutput"
                && (!worker || !["Read", "Edit", "Write", "Glob", "Grep", "Bash"].contains(&name))
            {
                return Err("Claude attempted a capability outside its role".into());
            }
        }
    }
    Ok(())
}
pub fn finish(
    events: &[Value],
    directory: &Path,
    packet: &Value,
    code: Option<i32>,
) -> Result<(Value, Value), String> {
    for e in events {
        inspect(e, packet)?;
    }
    let inits = events
        .iter()
        .filter(|e| e["type"] == "system" && e["subtype"] == "init")
        .count();
    let finals = events
        .iter()
        .filter(|e| e["type"] == "result")
        .collect::<Vec<_>>();
    if code != Some(0)
        || inits != 1
        || finals.len() != 1
        || finals[0]["subtype"] != "success"
        || finals[0]["is_error"] != false
    {
        return Err("Claude lacks one successful settled result".into());
    }
    let final_ = finals[0];
    let value = if let Some(v) = final_.get("structured_output").filter(|v| v.is_object()) {
        v.clone()
    } else {
        let raw = crate::continuous_adapter::raw_input(events)?;
        crate::scope_contract::strict_json(&raw)?
    };
    crate::output::validate(&value, packet)?;
    write(&directory.join("native-final.json"), &value, true)?;
    Ok((
        value,
        json!({"provider":"claude","requested_model":packet["model"],"reported_model":packet["model"],"usage":final_["usage"],"cost":final_.get("total_cost_usd").cloned().unwrap_or(json!("unknown")),"complete":true,"events_sha256":canonical_sha256(&json!(events)).map_err(|e|e.to_string())?}),
    ))
}
pub fn capacity(events: &[Value], code: Option<i32>) -> Option<Value> {
    if code.is_none() || code == Some(0) {
        return None;
    }
    events.iter().rev().filter(|e|e["type"]=="result"&&e["is_error"]==true||e["type"]=="rate_limit_event").find_map(|e|{
        let explicit=e["error"]=="rate_limit"||e["error"]=="overloaded_error"||e["rate_limit_info"]["status"]=="rejected"||e["subtype"]=="error_rate_limit";
        explicit.then(||json!({"reason":"Explicit Claude capacity refusal","retry_after_secs":e["retry_after_secs"].as_u64().unwrap_or(60).max(60)}))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> Value {
        json!({"provider":"claude","model":"claude-sonnet-5-5","development":true,"operation_kind":"development-review","review_targets":[{"result_commit":"abc"}]})
    }
    fn init() -> Value {
        json!({"type":"system","subtype":"init","model":"claude-sonnet-5-5","permissionMode":"dontAsk","tools":["StructuredOutput"],"mcp_servers":[],"skills":[],"slash_commands":[]})
    }
    #[test]
    fn claude_development_review_has_no_commit_and_checks_model_and_tools() {
        let p = packet();
        let result = json!({"schema_version":1,"summary":"Inspected exact diff","reviewed_commits":["abc"],"findings":["Missing edge case"],"requires_changes":true});
        let events = vec![
            init(),
            json!({"type":"result","subtype":"success","is_error":false,"structured_output":result}),
        ];
        let d = tempfile::tempdir().unwrap();
        assert_eq!(finish(&events, d.path(), &p, Some(0)).unwrap().0, result);
        let mut drift = init();
        drift["model"] = json!("other-model");
        assert!(inspect(&drift, &p).is_err());
        assert!(
            inspect(
                &json!({"message":{"content":[{"type":"tool_use","name":"Bash"}]}}),
                &p
            )
            .is_err()
        );
        let mut stale = result;
        stale["reviewed_commits"] = json!(["different"]);
        assert!(development::validate_result(&stale, &p).is_err());
        assert!(finish(&events, d.path(), &p, Some(1)).is_err());
    }
    #[test]
    fn claude_development_capacity_never_uses_tool_output_or_auth_errors() {
        assert!(
            capacity(
                &[json!({"type":"result","subtype":"error_rate_limit","is_error":true})],
                Some(1)
            )
            .is_some()
        );
        assert!(
            capacity(
                &[json!({"type":"result","subtype":"authentication_error","is_error":true})],
                Some(1)
            )
            .is_none()
        );
        assert!(
            capacity(
                &[json!({"message":{"content":[{"type":"tool_result","content":"rate_limit"}]}})],
                Some(1)
            )
            .is_none()
        );
        assert!(
            capacity(
                &[json!({"type":"result","subtype":"error_rate_limit","is_error":true})],
                None
            )
            .is_none()
        );
        let settings = settings(&packet()).unwrap();
        assert_eq!(settings["permissions"]["defaultMode"], "dontAsk");
        assert_eq!(settings["permissions"]["allow"], json!([]));
    }
}
