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
    let obj = value
        .as_object()
        .ok_or_else(|| "cursor payload is not an object".to_owned())?;

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

/// Render a canonical verdict into Cursor's native output JSON, dispatching on the
/// event Cursor is waiting for.
///
/// `Stop` ⇒ `{continue, followup_message}` (Cursor's stop contract — a reblock
/// rides `followup_message`, NOT the permission body). Everything else ⇒
/// `{continue, permission, userMessage, agentMessage}`. Crucially the ALLOW path
/// still carries `system_message`/`additional_context` as `agentMessage`: Cursor
/// has no `SessionStart`, so `beforeSubmitPrompt`'s allow is the ONLY door for the
/// mistake ledger + global rules + kanban to reach the agent — every turn.
#[must_use]
pub fn render(resp: &HookResponse, event: &str) -> String {
    // The answered event is authoritative (the edge passes it from the lowered
    // input); fall back to whatever the response stamped on itself.
    let event = if event.is_empty() {
        response_event(resp)
    } else {
        event
    };
    if event == "Stop" {
        return render_stop(resp);
    }
    let blocked = resp.decision == "block";
    let msg = context_message(resp);
    let out = CursorResponse {
        r#continue: !blocked,
        permission: if blocked { "deny" } else { "allow" },
        user_message: if msg.is_empty() {
            None
        } else {
            Some(msg.clone())
        },
        agent_message: if msg.is_empty() { None } else { Some(msg) },
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_open())
}

/// Cursor's `stop` hook contract: `{continue, followup_message}`. A stop-gate
/// BLOCK (unfinished work) becomes `continue:false` with the reblock reason as the
/// follow-up; otherwise the agent is free to stop.
fn render_stop(resp: &HookResponse) -> String {
    let blocked = resp.decision == "block";
    let msg = context_message(resp);
    let out = CursorStopResponse {
        r#continue: !blocked,
        followup_message: if msg.is_empty() { None } else { Some(msg) },
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_open())
}

/// The message Cursor should surface: prefer the verdict `reason`, else the
/// injected context. Context can live in three places depending on the emitter:
/// `system_message` (SessionStart-style ledger), the top-level `additional_context`,
/// or — the common `UserPromptSubmit` case — nested in
/// `hook_specific_output.additional_context`. The nested field is where the intent
/// gate puts per-prompt context, so Cursor's every-turn injection depends on it.
fn context_message(resp: &HookResponse) -> String {
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
        .map(|h| h.additional_context.clone())
        .unwrap_or_default()
}

/// The canonical event this response answers, read from its `hookSpecificOutput`
/// (gates stamp it there). Empty when the gate emitted a bare verdict.
fn response_event(resp: &HookResponse) -> &str {
    resp.hook_specific_output
        .as_ref()
        .map_or("", |h| h.hook_event_name.as_str())
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

/// Cursor's `stop` hook output schema (`camelCase` on the wire).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorStopResponse {
    r#continue: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    followup_message: Option<String>,
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
    obj.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
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
