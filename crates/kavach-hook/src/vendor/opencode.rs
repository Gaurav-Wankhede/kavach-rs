//! OpenCode native edge — tool execution & lifecycle interceptor.
//!
//! OpenCode hook contract is JSON-stdin → JSON-stdout with a top-level
//! `{"decision":"allow"|"deny","reason":...,"updated_input":...}` object.
//! It maps plugin events like `tool.execute.before` onto canonical `PreToolUse`.

use std::collections::HashMap;
use kavach_types::{HookInput, HookResponse};
use serde::Serialize;

#[cfg(test)]
#[path = "opencode_test.rs"]
mod tests;

/// Lower a raw OpenCode payload into the canonical [`HookInput`].
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<HookInput, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw_payload).map_err(|e| format!("JSON parse error: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "opencode payload is not an object".to_owned())?;

    let event = obj
        .get("hook_event_name")
        .or_else(|| obj.get("event"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let tool_name = obj
        .get("tool_name")
        .or_else(|| obj.get("tool"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let tool_input = obj
        .get("tool_input")
        .or_else(|| obj.get("parameters"))
        .or_else(|| obj.get("input"))
        .and_then(|v| serde_json::from_value::<HashMap<String, serde_json::Value>>(v.clone()).ok());

    let session_id = obj
        .get("session_id")
        .or_else(|| obj.get("conversation_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let prompt = obj
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    let cwd = obj
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    Ok(HookInput {
        session_id,
        prompt,
        cwd,
        tool_name,
        hook_event_name: canonical_event(event),
        tool_input,
        ..HookInput::default()
    })
}

/// Map OpenCode events to canonical event names.
fn canonical_event(raw: &str) -> String {
    match raw {
        "tool.execute.before" | "pre_tool" | "PreToolUse" => "PreToolUse".to_owned(),
        "tool.execute.after" | "post_tool" | "PostToolUse" => "PostToolUse".to_owned(),
        "agent.prompt.before" | "prompt" | "UserPromptSubmit" => "UserPromptSubmit".to_owned(),
        "agent.stop" | "stop" | "Stop" => "Stop".to_owned(),
        "session.start" | "session_start" | "SessionStart" => "SessionStart".to_owned(),
        other => other.to_owned(),
    }
}

/// OpenCode decision output object.
#[derive(Debug, Serialize)]
struct OpenCodeDecision {
    decision: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_input: Option<HashMap<String, serde_json::Value>>,
}

/// True when this verdict denies/asks.
fn is_denied(resp: &HookResponse) -> bool {
    if resp.decision == "block" {
        return true;
    }
    resp.hook_specific_output
        .as_ref()
        .is_some_and(|h| h.permission_decision == "deny" || h.permission_decision == "ask")
}

/// Agent-visible message from the response.
fn message(resp: &HookResponse) -> String {
    if !resp.reason.is_empty() {
        return resp.reason.clone();
    }
    if !resp.system_message.is_empty() {
        return resp.system_message.clone();
    }
    if !resp.additional_context.is_empty() {
        return resp.additional_context.clone();
    }
    resp.hook_specific_output
        .as_ref()
        .map(|h| {
            if h.additional_context.is_empty() {
                h.permission_decision_reason.clone()
            } else {
                h.additional_context.clone()
            }
        })
        .unwrap_or_default()
}

/// Render a canonical verdict into OpenCode native JSON output.
#[must_use]
pub fn render(resp: &HookResponse) -> String {
    let denied = is_denied(resp);
    let msg = message(resp);
    let updated = if denied {
        None
    } else {
        resp.hook_specific_output
            .as_ref()
            .and_then(|h| h.updated_input.clone())
            .or_else(|| resp.tool_input.clone())
    };

    let out = OpenCodeDecision {
        decision: if denied { "deny" } else { "allow" },
        reason: (!msg.is_empty()).then_some(msg),
        updated_input: updated,
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_closed())
}

/// Fail closed on serialization failure.
#[must_use]
pub fn fail_closed() -> String {
    r#"{"decision":"deny","reason":"kavach: hook internal error"}"#.to_owned()
}
