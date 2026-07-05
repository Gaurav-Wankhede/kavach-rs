use kavach_types::{HookResponse, HookSpecificOutput};

use crate::{HookAction, output};

/// `PreToolUse`: deny with reason via hookSpecificOutput.
#[must_use]
pub fn exit_pre_tool_deny(reason: &str) -> HookAction {
    let resp = HookResponse::new_pre_tool_use_deny(reason);
    output(&resp);
    HookAction::Done
}

/// `PreToolUse`: allow with optional context via hookSpecificOutput.
#[must_use]
pub fn exit_pre_tool_allow(context: Option<&str>) -> HookAction {
    // SOURCE: anthropic.com/engineering/effective-context-engineering-for-ai-agents
    let compressed = context.map(|ctx| kavach_toon::caveman::compress(ctx, kavach_toon::caveman::Level::Full));
    let resp = compressed.as_deref().map_or_else(
        || HookResponse::new_pre_tool_use_allow("allow"),
        |ctx| HookResponse::new_pre_tool_use_with_context("allow", ctx),
    );
    output(&resp);
    HookAction::Done
}

/// `PreToolUse`: ask user for explicit approval via hookSpecificOutput.
///
/// Emits `permissionDecision: "ask"` — Claude Code prompts the user with
/// `reason` before proceeding. Precedence: deny > defer > ask > allow.
#[must_use]
pub fn exit_pre_tool_ask(reason: &str) -> HookAction {
    let resp = HookResponse::new_pre_tool_use_ask(reason);
    output(&resp);
    HookAction::Done
}

/// `PostToolUse`: block with reason + context.
#[must_use]
pub fn exit_post_tool_block(reason: &str, context: &str) -> HookAction {
    let resp = HookResponse::new_post_tool_use_block(reason, context);
    output(&resp);
    HookAction::Done
}

/// `PostToolUse`: context injection.
#[must_use]
pub fn exit_post_tool_context(context: &str) -> HookAction {
    let resp = HookResponse {
        hook_specific_output: Some(HookSpecificOutput {
            hook_event_name: "PostToolUse".into(),
            additional_context: context.into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// `UserPromptSubmit`: context injection via hookSpecificOutput.
#[must_use]
pub fn exit_prompt_context(context: &str) -> HookAction {
    let resp = HookResponse::new_user_prompt_submit_context(context);
    output(&resp);
    HookAction::Done
}

/// `UserPromptSubmit`: block with reason via hookSpecificOutput.
#[must_use]
pub fn exit_prompt_submit_block(reason: &str) -> HookAction {
    let resp = HookResponse::new_user_prompt_submit_block(reason);
    output(&resp);
    HookAction::Done
}

/// `SessionStart`: context via systemMessage (no hookSpecificOutput).
#[must_use]
pub fn exit_session_start_context(context: &str) -> HookAction {
    let resp = HookResponse {
        system_message: context.into(),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// `SessionStart`: context plus CC 2.1.152 `reloadSkills` / `sessionTitle` outputs.
///
/// `reload_skills` asks CC to re-scan skill dirs (Kavach already rebuilt its own
/// registry); `session_title`, when non-empty, names the session in agent/bg views.
/// Empty title is omitted so CC keeps its default. SOURCE: changelog v2.1.152.
#[must_use]
pub fn exit_session_start_full(
    context: &str,
    reload_skills: bool,
    session_title: &str,
) -> HookAction {
    let resp = HookResponse {
        system_message: context.into(),
        hook_specific_output: Some(HookSpecificOutput {
            hook_event_name: "SessionStart".into(),
            reload_skills: reload_skills.then_some(true),
            session_title: (!session_title.is_empty()).then(|| session_title.to_owned()),
            ..Default::default()
        }),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// Stop hooks don't support hookSpecificOutput — use `system_message` instead.
#[must_use]
pub fn exit_stop_context(context: &str) -> HookAction {
    let resp = HookResponse {
        system_message: context.into(),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// Stop: block stopping.
#[must_use]
pub fn exit_stop_block(reason: &str) -> HookAction {
    let resp = HookResponse::new_stop_block(reason);
    output(&resp);
    HookAction::Done
}

/// Notification: context injection via systemMessage (no hookSpecificOutput).
#[must_use]
pub fn exit_notification_context(context: &str) -> HookAction {
    exit_notification_with_sequence(context, "")
}

/// Notification: context plus an optional CC 2.1.141 `terminalSequence`.
///
/// The sequence is written to the controlling tty (e.g. a bell on a permission
/// stall). Empty `seq` omits the field. SOURCE: changelog v2.1.141.
#[must_use]
pub fn exit_notification_with_sequence(context: &str, seq: &str) -> HookAction {
    let resp = HookResponse {
        system_message: context.into(),
        terminal_sequence: (!seq.is_empty()).then(|| seq.to_owned()),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// `PostToolUse`: trim verbose tool output and inject context.
/// Injects trimmed output as `additional_context` — CC 2.1 has no `PostToolUse` output replacement.
#[must_use]
pub fn exit_post_tool_trimmed(trimmed_output: &str, context: &str) -> HookAction {
    let combined = if context.is_empty() {
        trimmed_output.to_owned()
    } else {
        format!("{trimmed_output}\n\n{context}")
    };
    let resp = HookResponse {
        hook_specific_output: Some(HookSpecificOutput {
            hook_event_name: "PostToolUse".into(),
            additional_context: combined,
            ..Default::default()
        }),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// `PostToolUseFailure`: context injection via systemMessage (no hookSpecificOutput).
#[must_use]
pub fn exit_post_tool_failure_context(context: &str) -> HookAction {
    let resp = HookResponse {
        system_message: context.into(),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}
