use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook response builder; non_exhaustive => E0639"
)]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub permission_decision: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub permission_decision_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub additional_context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_skills: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_title: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate DTO; hook constructors + callers; non_exhaustive => E0639"
)]
pub struct HookResponse {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub decision: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub additional_context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<HashMap<String, serde_json::Value>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#continue: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stop_reason: String,
    #[serde(default)]
    pub suppress_output: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub system_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_sequence: Option<String>,
}

const NEXT_ACTION_TRAILER: &str = "\n[NEXT_ACTION] Verdict = redirect, not a dead end: do the named step THIS turn, then RETRY this exact call. Never report BLOCKED, never surrender, never describe instead of doing.";

/// Appends the standard action-imperative trailer unless already composed.
fn with_next_action(reason: &str) -> String {
    if reason.contains("[NEXT_ACTION]") {
        reason.into()
    } else {
        format!("{reason}{NEXT_ACTION_TRAILER}")
    }
}

impl HookResponse {
    #[must_use]
    pub fn new_approve(reason: &str) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_block(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: with_next_action(reason),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_modify(reason: &str, context: &str) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            additional_context: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_allow(reason: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "allow".into(),
                permission_decision_reason: reason.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_deny(reason: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "deny".into(),
                permission_decision_reason: with_next_action(reason),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_ask(reason: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "ask".into(),
                permission_decision_reason: reason.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_pre_tool_use_with_context(reason: &str, context: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PreToolUse".into(),
                permission_decision: "allow".into(),
                permission_decision_reason: reason.into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_post_tool_use_block(reason: &str, context: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "PostToolUse".into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_user_prompt_submit_context(context: &str) -> Self {
        Self {
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "UserPromptSubmit".into(),
                additional_context: context.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_user_prompt_submit_block(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "UserPromptSubmit".into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_stop_block(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            hook_specific_output: Some(HookSpecificOutput {
                hook_event_name: "Stop".into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_permission_allow(reason: &str) -> Self {
        Self {
            decision: "approve".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_permission_deny(reason: &str) -> Self {
        Self {
            decision: "block".into(),
            reason: reason.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_session_end_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_subagent_start_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_subagent_stop_context(context: &str) -> Self {
        Self {
            system_message: context.into(),
            ..Default::default()
        }
    }
}
