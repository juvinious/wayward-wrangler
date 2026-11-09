//! Frozen native-run authorization inputs. This module never dispatches a CLI.
use crate::codec::canonical_sha256;
use serde_json::{Value, json};

pub fn plan(envelope: &Value) -> Result<Value, String> {
    let config = &envelope["config"];
    let selected = json!({
        "schema_version":1,
        "project_key": required(envelope,"project_key")?,
        "project": required(envelope,"project")?,
        "run_id": required(envelope,"run_id")?,
        "brief_path": required(envelope,"brief_path")?,
        "brief_sha256": required(envelope,"brief_sha256")?,
        "config_path": required(envelope,"config_path")?,
        "config_sha256": required(envelope,"config_sha256")?,
        "rules_sha256": required(envelope,"rules_sha256")?,
        "context_path": required(&envelope["context_snapshot"],"selection_path")?,
        "context_digest": required(envelope,"context_digest")?,
        "application_version": required(envelope,"application_version")?,
        "protocol_version": required(envelope,"protocol_version")?,
        "prompt_contract": required(config,"prompt_contract")?,
        "review_limit": required(config,"review_limit")?,
        "max_invocations": required(config,"max_invocations")?,
        "roles": required(config,"roles")?,
        "target": required(envelope,"target")?,
    });
    Ok(selected)
}
fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing frozen plan field {key}"))
}
pub fn plan_digest(envelope: &Value) -> Result<String, String> {
    canonical_sha256(&plan(envelope)?).map_err(|e| e.to_string())
}
pub fn check_explicit_digest(envelope: &Value, supplied: &str) -> Result<(), String> {
    if plan_digest(envelope)? == supplied {
        Ok(())
    } else {
        Err("Native authorization digest does not match current project/inputs/settings".into())
    }
}
