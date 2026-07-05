use kavach_types::HookResponse;
use serde::Serialize;

use crate::{HookAction, output, write_json};

// --- UserPromptSubmit legacy wire format ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserPromptSubmitOutput {
    pub hook_event_name: String,
    pub additional_context: String,
}

/// Write `UserPromptSubmit` JSON to stdout.
#[must_use]
pub fn exit_user_prompt_submit(context: &str) -> HookAction {
    // SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents
    let context = kavach_toon::caveman::compress(context, kavach_toon::caveman::Level::Full);
    let out = UserPromptSubmitOutput {
        hook_event_name: "UserPromptSubmit".into(),
        additional_context: context,
    };
    write_json(&out);
    HookAction::Done
}

/// `UserPromptSubmit` with empty context.
#[must_use]
pub fn exit_user_prompt_submit_silent() -> HookAction {
    exit_user_prompt_submit("")
}

// --- SessionEnd ---

/// `SessionEnd` with context.
#[must_use]
pub fn exit_session_end(context: &str) -> HookAction {
    let context = kavach_toon::caveman::compress(context, kavach_toon::caveman::Level::Full);
    output(&HookResponse::new_session_end_context(&context));
    HookAction::Done
}

// --- Subagent ---

/// `SubagentStart` with context.
#[must_use]
pub fn exit_subagent_start(context: &str) -> HookAction {
    let context = kavach_toon::caveman::compress(context, kavach_toon::caveman::Level::Full);
    output(&HookResponse::new_subagent_start_context(&context));
    HookAction::Done
}

/// `SubagentStop` with context.
#[must_use]
pub fn exit_subagent_stop(context: &str) -> HookAction {
    output(&HookResponse::new_subagent_stop_context(context));
    HookAction::Done
}

// --- Permission ---

/// `PermissionRequest` allow.
#[must_use]
pub fn exit_permission_allow(reason: &str) -> HookAction {
    output(&HookResponse::new_permission_allow(reason));
    HookAction::Done
}

/// `PermissionRequest` deny.
#[must_use]
pub fn exit_permission_deny(reason: &str) -> HookAction {
    output(&HookResponse::new_permission_deny(reason));
    HookAction::Done
}

/// `PermissionRequest` allow (`PreToolUse` permissionRequest event).
#[must_use]
pub fn exit_permission_request_allow(reason: &str) -> HookAction {
    output(&HookResponse::new_permission_allow(reason));
    HookAction::Done
}

/// `PermissionRequest` deny (`PreToolUse` permissionRequest event).
#[must_use]
pub fn exit_permission_request_deny(reason: &str) -> HookAction {
    output(&HookResponse::new_permission_deny(reason));
    HookAction::Done
}

/// Elicitation decline — user declined an elicitation prompt.
#[must_use]
pub fn exit_elicitation_decline() -> HookAction {
    use serde::Serialize;
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ElicitationOutput {
        action: String,
    }
    write_json(&ElicitationOutput {
        action: "decline".into(),
    });
    HookAction::Done
}
