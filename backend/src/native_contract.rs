//! Schema-4 packet-only role contract. This module has no dispatch path: native
//! admission stays closed until the separately recorded client boundary passes.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CAPABILITY_PROFILE: &str = "static-file-packet-v1";
pub const COORDINATOR_RESULT_VERSION: u64 = 1;
pub const CONVERSATION_LIMIT: u32 = 6;
pub const CONSULTATION_LIMIT: u32 = 1;
pub const TASK_LIMIT: u32 = 4;
pub const REVIEW_LIMIT: u32 = 2;
pub const PACKET_LIMIT: usize = 96 * 1024;
pub const FILE_BUNDLE_LIMIT: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeAdmission {
    Closed,
}

impl NativeAdmission {
    pub fn require_proof(self) -> Result<(), &'static str> {
        match self {
            Self::Closed => Err(
                "Native provider unavailable: effective capability and startup boundary unproven",
            ),
        }
    }
}

pub const NATIVE_ADMISSION: NativeAdmission = NativeAdmission::Closed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEffect {
    LocalProvisional,
    Forbidden,
    Unclassified,
}

/// Names are pinned to the first static-file profile. Classification never
/// dispatches a tool or grants a model any application authority.
pub fn tool_effect(name: &str) -> ToolEffect {
    match name {
        "clock.curr_time" | "request_user_input_async" | "send_message_to_user_async" => {
            ToolEffect::LocalProvisional
        }
        "apply_patch"
        | "exec_command"
        | "shell_command"
        | "write_stdin"
        | "view_image"
        | "web_search"
        | "request_permissions"
        | "sleep" => ToolEffect::Forbidden,
        _ => ToolEffect::Unclassified,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinatorTurn {
    pub schema_version: u64,
    pub assistant_text: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub action: Option<CoordinatorAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CoordinatorAction {
    AskQuestion {
        question: String,
        rationale: String,
        options: Vec<String>,
    },
    ProposeTask {
        brief: String,
    },
    RequestConsultation {
        question: String,
        evidence_refs: Vec<String>,
    },
    ProposeSteering {
        task_id: String,
        text: String,
    },
}

impl CoordinatorTurn {
    pub fn validate(value: &Value) -> Result<Self, String> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        if bytes.len() > PACKET_LIMIT {
            return Err("Coordinator result exceeds 96 KiB".into());
        }
        let result: Self = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if result.schema_version != COORDINATOR_RESULT_VERSION {
            return Err("Unsupported coordinator result version".into());
        }
        if result.assistant_text.len() > 32 * 1024 || result.evidence_refs.len() > 32 {
            return Err("Coordinator text or evidence exceeds limit".into());
        }
        match &result.action {
            Some(CoordinatorAction::AskQuestion {
                question,
                rationale,
                options,
            }) => {
                if question.trim().is_empty()
                    || rationale.trim().is_empty()
                    || options.len() > 8
                    || options.iter().any(|v| v.trim().is_empty())
                {
                    return Err("Invalid coordinator question".into());
                }
            }
            Some(CoordinatorAction::ProposeTask { brief }) if brief.trim().is_empty() => {
                return Err("Empty task proposal".into());
            }
            Some(CoordinatorAction::RequestConsultation {
                question,
                evidence_refs,
            }) if question.trim().is_empty() || evidence_refs.len() > 16 => {
                return Err("Invalid consultation".into());
            }
            Some(CoordinatorAction::ProposeSteering { task_id, text })
                if task_id.is_empty() || text.trim().is_empty() =>
            {
                return Err("Invalid steering proposal".into());
            }
            _ => {}
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn closed_admission_and_unknown_tool_refuse() {
        assert!(NativeAdmission::Closed.require_proof().is_err());
        assert_eq!(tool_effect("apply_patch"), ToolEffect::Forbidden);
        assert_eq!(tool_effect("clock.curr_time"), ToolEffect::LocalProvisional);
        assert_eq!(tool_effect("unexpected_tool"), ToolEffect::Unclassified);
    }

    #[test]
    fn one_typed_action_and_no_model_authority() {
        let valid = json!({"schema_version":1,"assistant_text":"Let's discuss this.",
            "action":{"kind":"ask_question","payload":{"question":"Which unit?","rationale":"The brief is silent.","options":["C","F"]}}});
        assert!(CoordinatorTurn::validate(&valid).is_ok());
        let mut forged = valid.clone();
        forged["approval"] = json!("approved");
        assert!(CoordinatorTurn::validate(&forged).is_err());
        let mut bad = valid;
        bad["action"]["kind"] = json!("run_shell");
        assert!(CoordinatorTurn::validate(&bad).is_err());
    }
}
