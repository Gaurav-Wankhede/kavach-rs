// split: Cursor native edge — its own input DTO + output renderer, kept off the
// generic vendor hub so the Cursor dialect (field names, camelCase events,
// continue/permission output, fail-open) lives in one place.
//! Cursor IDE native edge.
//!
//! Cursor's hook payloads use different field NAMES than Claude Code
//! (`conversation_id`, `workspace_roots[]`, `metadata.tool_name`) and camelCase
//! event names (`beforeShellExecution`, `beforeSubmitPrompt`, `stop`). Its output
//! contract is `{continue, permission, userMessage, agentMessage}` (or
//! `{followup_message}` for `stop`), and it fails OPEN — a hook error lets the
//! action through rather than wedging the IDE.
//! SOURCE: <https://cursor.com/docs/hooks>

use kavach_types::{HookInput, HookResponse};
use serde::Serialize;

/// Lower a raw Cursor payload into the canonical [`HookInput`].
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<HookInput, String> {
    // Reuse the W1 null-scrubbing parse to a Value, then map native names.
    let value: serde_json::Value =
        serde_json::from_str(raw_payload).map_err(|e| format!("JSON parse error: {e}"))?;
    let obj = value.as_object().ok_or_else(|| "cursor payload is not an object".to_owned())?;

    let input = HookInput {
        session_id: get_str(obj, "conversation_id"),
        prompt: get_str(obj, "prompt"),
        cwd: first_workspace_root(obj.get("workspace_roots")),
        tool_name: cursor_tool_name(obj),
        hook_event_name: canonical_event(&get_str(obj, "hook_event_name")),
        ..HookInput::default()
    };
    Ok(input)
}

/// Render a canonical verdict into Cursor's native output JSON.
///
/// `block` ⇒ `{continue:false, permission:"deny", userMessage, agentMessage}`;
/// otherwise ⇒ `{continue:true, permission:"allow"}`, carrying any reason/context
/// as the user + agent message.
#[must_use]
pub fn render(resp: &HookResponse) -> String {
    let blocked = resp.decision == "block";
    let msg = if resp.reason.is_empty() { resp.additional_context.clone() } else { resp.reason.clone() };
    let out = CursorResponse {
        r#continue: !blocked,
        permission: if blocked { "deny" } else { "allow" },
        user_message: if msg.is_empty() { None } else { Some(msg.clone()) },
        agent_message: if msg.is_empty() { None } else { Some(msg) },
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_open())
}

/// Cursor's NATIVE failure default: fail OPEN.
///
/// An unparseable payload or internal error must let the action through (Cursor's
/// own model) rather than block and wedge the editor. Emits a permissive body;
/// the caller also logs to stderr.
#[must_use]
pub fn fail_open() -> String {
    r#"{"continue":true,"permission":"allow"}"#.to_owned()
}

/// Cursor's native output schema (`camelCase` on the wire).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorResponse {
    r#continue: bool,
    permission: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_message: Option<String>,
}

/// Map a Cursor camelCase event name to the canonical event the gates dispatch on.
/// Unknown/empty passes through unchanged (a forward-compatible default).
fn canonical_event(cursor_event: &str) -> String {
    match cursor_event {
        "beforeShellExecution" | "beforeMCPExecution" | "beforeReadFile" => "PreToolUse",
        "afterFileEdit" => "PostToolUse",
        "beforeSubmitPrompt" => "UserPromptSubmit",
        "stop" => "Stop",
        other => return other.to_owned(),
    }
    .to_owned()
}

/// Read a string field from the object, tolerating absent/null (→ empty).
fn get_str(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned()
}

/// Cursor sends `workspace_roots` as an array (VS Code multi-root); the canonical
/// `cwd` is its first entry. Absent/empty ⇒ "".
fn first_workspace_root(roots: Option<&serde_json::Value>) -> String {
    roots
        .and_then(serde_json::Value::as_array)
        .and_then(|a| a.first())
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Cursor nests the tool name under `metadata.tool_name` (falling back to a
/// top-level `tool_name` if a future payload promotes it).
fn cursor_tool_name(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    obj.get("metadata")
        .and_then(|m| m.get("tool_name"))
        .or_else(|| obj.get("tool_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
