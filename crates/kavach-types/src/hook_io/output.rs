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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub system_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_sequence: Option<String>,
}

const NEXT_ACTION_TRAILER: &str = concat!(
    "\n",
    "[NEXT_ACTION] This verdict names your next step. Do the named step this turn, then retry the same call. Act on it rather than pausing or restating it."
);

const fn base_specific_output(hook_event_name: String) -> HookSpecificOutput {
    HookSpecificOutput {
        hook_event_name,
        permission_decision: String::new(),
        permission_decision_reason: String::new(),
        updated_input: None,
        additional_context: String::new(),
        reload_skills: None,
        session_title: None,
    }
}

const fn base_response() -> HookResponse {
    HookResponse {
        decision: String::new(),
        reason: String::new(),
        additional_context: String::new(),
        tool_input: None,
        hook_specific_output: None,
        r#continue: None,
        stop_reason: String::new(),
        suppress_output: None,
        system_message: String::new(),
        terminal_sequence: None,
    }
}

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
        let mut res = base_response();
        res.decision = "approve".into();
        res.reason = reason.into();
        res
    }

    #[must_use]
    pub fn new_block(reason: &str) -> Self {
        let mut res = base_response();
        res.decision = "block".into();
        res.reason = with_next_action(reason);
        res
    }

    #[must_use]
    pub fn new_modify(reason: &str, context: &str) -> Self {
        let mut res = base_response();
        res.decision = "approve".into();
        res.reason = reason.into();
        res.additional_context = context.into();
        res
    }

    #[must_use]
    pub fn new_pre_tool_use_allow(reason: &str) -> Self {
        let mut out = base_specific_output("PreToolUse".into());
        out.permission_decision = "allow".into();
        out.permission_decision_reason = reason.into();
        let mut res = base_response();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_pre_tool_use_deny(reason: &str) -> Self {
        let mut out = base_specific_output("PreToolUse".into());
        out.permission_decision = "deny".into();
        out.permission_decision_reason = with_next_action(reason);
        let mut res = base_response();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_pre_tool_use_ask(reason: &str) -> Self {
        let mut out = base_specific_output("PreToolUse".into());
        out.permission_decision = "ask".into();
        out.permission_decision_reason = reason.into();
        let mut res = base_response();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_pre_tool_use_with_context(reason: &str, context: &str) -> Self {
        let mut out = base_specific_output("PreToolUse".into());
        out.permission_decision = "allow".into();
        out.permission_decision_reason = reason.into();
        out.additional_context = context.into();
        let mut res = base_response();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_pre_tool_use_rewrite(
        reason: &str,
        updated_input: HashMap<String, serde_json::Value>,
        context: Option<&str>,
    ) -> Self {
        let mut out = base_specific_output("PreToolUse".into());
        out.permission_decision = "allow".into();
        out.permission_decision_reason = reason.into();
        out.updated_input = Some(updated_input);
        out.additional_context = context.map_or_else(String::new, Into::into);
        let mut res = base_response();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_post_tool_use_block(reason: &str, context: &str) -> Self {
        let mut out = base_specific_output("PostToolUse".into());
        out.additional_context = context.into();
        let mut res = base_response();
        res.decision = "block".into();
        res.reason = reason.into();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_user_prompt_submit_context(context: &str) -> Self {
        let mut out = base_specific_output("UserPromptSubmit".into());
        out.additional_context = context.into();
        let mut res = base_response();
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_user_prompt_submit_block(reason: &str) -> Self {
        let out = base_specific_output("UserPromptSubmit".into());
        let mut res = base_response();
        res.decision = "block".into();
        res.reason = with_next_action(reason);
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_stop_block(reason: &str) -> Self {
        let out = base_specific_output("Stop".into());
        let mut res = base_response();
        res.decision = "block".into();
        res.reason = with_next_action(reason);
        res.hook_specific_output = Some(out);
        res
    }

    #[must_use]
    pub fn new_permission_allow(reason: &str) -> Self {
        let mut res = base_response();
        res.decision = "approve".into();
        res.reason = reason.into();
        res
    }

    #[must_use]
    pub fn new_permission_deny(reason: &str) -> Self {
        let mut res = base_response();
        res.decision = "block".into();
        res.reason = with_next_action(reason);
        res
    }

    #[must_use]
    pub fn new_session_end_context(context: &str) -> Self {
        let mut res = base_response();
        res.system_message = context.into();
        res
    }

    #[must_use]
    pub fn new_subagent_start_context(context: &str) -> Self {
        let mut res = base_response();
        res.system_message = context.into();
        res
    }

    #[must_use]
    pub fn new_subagent_stop_context(context: &str) -> Self {
        let mut res = base_response();
        res.system_message = context.into();
        res
    }
}
