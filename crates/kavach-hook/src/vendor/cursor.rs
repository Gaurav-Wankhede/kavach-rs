// split: Cursor native edge — its own input DTO + output renderer, kept off the
// generic vendor hub so the Cursor dialect (field names, camelCase events,
// continue/permission output, fail-open) lives in one place.
//! Cursor IDE native edge.
//!
//! Cursor's hook payloads use different field NAMES than Claude Code
//! (`conversation_id`, `workspace_roots[]`, `metadata.tool_name`) and camelCase
//! event names (`beforeShellExecution`, `beforeSubmitPrompt`, `sessionStart`,
//! `stop`). Its output contract is PER-EVENT (`snake_case` throughout):
//!
//! - `PreToolUse` (`beforeShell`/MCP/`ReadFile`) → `{permission, user_message, agent_message}`
//! - `beforeSubmitPrompt` → `{continue, user_message}` (`user_message` is user-facing only)
//! - `sessionStart`/`workspaceOpen` → `{additional_context}` (the ONLY agent-readable door)
//! - `afterFileEdit` → no honored output fields
//! - `stop`/`subagentStop` → `{followup_message}`
//!
//! It fails OPEN — a hook error lets the action through rather than wedging the IDE.
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

    let raw_event = get_str(obj, "hook_event_name");
    let input = HookInput {
        session_id: get_str(obj, "conversation_id"),
        prompt: get_str(obj, "prompt"),
        cwd: first_workspace_root(obj.get("workspace_roots")),
        tool_name: cursor_tool_name(obj, &raw_event),
        hook_event_name: canonical_event(&raw_event),
        tool_input: cursor_tool_input(obj),
        // Cursor `loop_count>0` maps to CC `stop_hook_active` (the stop gate's
        // retry/verify pivot). See decision.hook.cursor-loop-count-map.
        stop_hook_active: get_u64(obj, "loop_count") > 0,
        ..HookInput::default()
    };
    Ok(input)
}

/// Render a canonical verdict into Cursor's native output JSON, dispatching on the
/// event Cursor is waiting for.
///
/// Dispatch on the answered event to Cursor's PER-EVENT output contract
/// (<https://cursor.com/docs/hooks>). The mistake ledger + global rules + kanban
/// reach the agent via `sessionStart`'s `additional_context` (the ONLY
/// agent-readable door) — `beforeSubmitPrompt` CANNOT inject model context, so on
/// Cursor that boot context lands once per conversation, not every turn.
#[must_use]
pub fn render(resp: &HookResponse, event: &str) -> String {
    // The answered event is authoritative (the edge passes it from the lowered
    // input); fall back to whatever the response stamped on itself.
    let event = if event.is_empty() {
        response_event(resp)
    } else {
        event
    };
    // Cursor's output contract is PER-EVENT (cursor.com/docs/hooks) — a single
    // blob carrying every field is imprecise (Cursor silently drops the fields an
    // event doesn't honor) AND wrong for context injection (see SessionStart).
    match event {
        "Stop" => render_stop(resp),
        "SessionStart" => render_session_start(resp),
        // afterFileEdit / lifecycle hooks honor NO output fields — emit `{}`, not a
        // spurious permission blob. Lifecycle context (preCompact, subagentStart)
        // is queued to session relay and flushed on the next preToolUse
        // agent_message door instead.
        "PostToolUse" | "PreCompact" | "SubagentStart" | "SubagentStop" | "SessionEnd" => {
            "{}".to_owned()
        }
        // beforeSubmitPrompt honors ONLY {continue, user_message}; user_message is
        // shown to the USER (not the model), so it carries the block reason only.
        "UserPromptSubmit" => render_submit(resp),
        // PreToolUse (beforeShell/MCP/ReadFile): {permission, user_message,
        // agent_message} — NO `continue` field.
        _ => render_pre_tool(resp),
    }
}

/// `SessionStart`/`workspaceOpen` ⇒ `{additional_context}` — the ONLY Cursor hook
/// whose output reaches the model. Per <https://cursor.com/docs/hooks>,
/// `additional_context` is "added to the conversation's initial system context".
/// This is where the mistake ledger + global rules + kanban boot context land;
/// `beforeSubmitPrompt` CANNOT inject agent-readable context, so on Cursor the
/// harness context is injected ONCE per conversation here (not every turn — a
/// Cursor API limit, not a bug). Context is read from `system_message` (where
/// `exit_session_start_full` puts it) or `additional_context`.
fn render_session_start(resp: &HookResponse) -> String {
    let ctx = context_message(resp);
    let out = CursorSessionStartResponse {
        additional_context: (!ctx.is_empty()).then_some(ctx),
    };
    serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_owned())
}

/// `beforeSubmitPrompt` ⇒ `{continue, user_message}` ONLY. `user_message` is shown
/// to the USER when blocked (it does NOT reach the model), so it carries the block
/// reason and nothing else. A clean allow emits `{continue:true}` with no message.
fn render_submit(resp: &HookResponse) -> String {
    let blocked = is_blocked(resp);
    let msg = context_message(resp);
    let out = CursorSubmitResponse {
        r#continue: !blocked,
        // Surface a message only when blocking — an allow-path message here would
        // be a user-facing popup on every turn.
        user_message: (blocked && !msg.is_empty()).then_some(msg),
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_open())
}

/// `PreToolUse` (`beforeShellExecution`/`beforeMCPExecution`/`beforeReadFile`) ⇒
/// `{permission, user_message, agent_message}`. NO `continue` field (Cursor ignores
/// it on these events). `agent_message` IS honored here (unlike submit), so a deny
/// reason can reach both the user and the agent.
fn render_pre_tool(resp: &HookResponse) -> String {
    let blocked = is_blocked(resp);
    let msg = context_message(resp);
    let agent = (!msg.is_empty()).then_some(msg);
    // Allow-path relay is model-readable via `agent_message` only — do NOT mirror
    // it into `user_message` (Cursor shows user_message to the human).
    let user = if blocked { agent.clone() } else { None };
    let out = CursorPreToolResponse {
        permission: if blocked { "deny" } else { "allow" },
        user_message: user,
        agent_message: agent,
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_open())
}

/// Cursor's `stop` hook contract: `{followup_message}` ONLY (`snake_case`). Per
/// <https://cursor.com/docs/hooks>, continuation is driven SOLELY by a non-empty
/// `followup_message` — Cursor auto-submits it as the next user message. There is
/// NO `continue` field on the stop hook (it is ignored), and the field is
/// `snake_case`, NOT `camelCase`. The previous `{continue, followupMessage}` shape was
/// the loophole: Cursor never found `followup_message`, so the harness loop's
/// `[AUTO_CONTINUE]` text was silently dropped and the loop died.
///
/// We populate `followup_message` ONLY when the gate decided to CONTINUE
/// (`decision == "block"` — the gate blocks the stop to force the next dispatch
/// turn). On a clean stop (`[ALL_BLOCKED]` / drained board, `decision != block`)
/// we send an EMPTY object so Cursor stops — a non-empty message there would
/// wrongly resubmit and spin.
fn render_stop(resp: &HookResponse) -> String {
    let should_continue = resp.decision == "block";
    let msg = context_message(resp);
    let out = CursorStopResponse {
        followup_message: (should_continue && !msg.is_empty()).then_some(msg),
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_open())
}

/// Is this verdict a BLOCK? A canonical block can arrive two ways: the top-level
/// `decision == "block"` (stop/post-tool blocks) OR `hook_specific_output.
/// permission_decision == "deny"` (the `PreToolUse` deny path — `new_pre_tool_use_deny`
/// sets ONLY this field, leaving `decision` empty). Checking only `decision` made
/// Cursor render `allow` for every `PreToolUse` deny, so the destructive blocklist's
/// deny was silently downgraded to allow. Treat `ask` as block too (Cursor has no
/// distinct ask — surface it as a deny the user must override, never a silent allow).
fn is_blocked(resp: &HookResponse) -> bool {
    if resp.decision == "block" {
        return true;
    }
    resp.hook_specific_output.as_ref().is_some_and(|h| {
        h.permission_decision == "deny" || h.permission_decision == "ask"
    })
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
    if let Some(h) = resp.hook_specific_output.as_ref() {
        // Allow-path relay (turn shadow, advisories) lives in `additional_context`;
        // prefer it over the boilerplate `permission_decision_reason` ("allow").
        // Deny/ask reasons use `permission_decision_reason` with empty context.
        if !h.additional_context.is_empty() {
            return h.additional_context.clone();
        }
        if !h.permission_decision_reason.is_empty() {
            return h.permission_decision_reason.clone();
        }
    }
    String::new()
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

/// `PreToolUse` output: `{permission, user_message, agent_message}` — `snake_case`
/// on the wire per <https://cursor.com/docs/hooks> (a prior `camelCase` rename
/// emitted `userMessage`/`agentMessage`, which Cursor silently ignored). NO
/// `continue` field — Cursor does not honor it on permission-gating hooks.
#[derive(Debug, Serialize)]
struct CursorPreToolResponse {
    permission: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_message: Option<String>,
}

/// `beforeSubmitPrompt` output: `{continue, user_message}` ONLY. No `permission`,
/// no `agent_message` (Cursor honors neither here); `user_message` is user-facing.
#[derive(Debug, Serialize)]
struct CursorSubmitResponse {
    r#continue: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_message: Option<String>,
}

/// `sessionStart` output: `{additional_context}` — the only agent-readable
/// injection door. SOURCE: <https://cursor.com/docs/hooks>.
#[derive(Debug, Serialize)]
struct CursorSessionStartResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_context: Option<String>,
}

/// Cursor's `stop` hook output schema: `{followup_message}` ONLY, `snake_case`.
/// Per <https://cursor.com/docs/hooks> the stop hook has NO `continue` field —
/// continuation is driven solely by a non-empty `followup_message`. The previous
/// `{continue, followupMessage}` (`camelCase` + bogus field) was the loophole that
/// broke every Cursor harness loop: Cursor never found `followup_message`.
#[derive(Debug, Serialize)]
struct CursorStopResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    followup_message: Option<String>,
}

/// Map a Cursor camelCase event name to the canonical event the gates dispatch on.
/// Unknown/empty passes through unchanged (a forward-compatible default).
fn canonical_event(cursor_event: &str) -> String {
    match cursor_event {
        "preToolUse" | "beforeShellExecution" | "beforeMCPExecution" | "beforeReadFile" => {
            "PreToolUse"
        }
        "postToolUse" | "afterFileEdit" => "PostToolUse",
        "beforeSubmitPrompt" => "UserPromptSubmit",
        // Cursor's session lifecycle. `sessionStart` is the ONLY hook whose output
        // injects agent-readable context (`additional_context`) — the real door for
        // the mistake ledger + global rules + kanban, since `beforeSubmitPrompt`
        // CANNOT reach the model. SOURCE: <https://cursor.com/docs/hooks>.
        "sessionStart" | "workspaceOpen" => "SessionStart",
        "sessionEnd" => "SessionEnd",
        "preCompact" => "PreCompact",
        "subagentStart" => "SubagentStart",
        // subagentStop is NOT the harness stop hook — it must not route through
        // render_stop (which would emit followup_message and spin the IDE).
        "subagentStop" => "SubagentStop",
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

/// Read an unsigned-integer field, tolerating absent/null/wrong-type (→ 0).
/// Used for Cursor's `loop_count`; a malformed or missing value fails safe to 0
/// (treated as "not yet in a follow-up loop").
fn get_u64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
    obj.get(key).and_then(serde_json::Value::as_u64).unwrap_or(0)
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

/// Bridge Cursor's FLAT tool fields into the canonical `tool_input` map the
/// gates read. Cursor's `beforeShellExecution` carries the shell command at
/// top-level `command`, and `afterFileEdit` carries `file_path`/`edits`; Claude
/// Code nests these under `tool_input`. Without this mapping the destructive
/// blocklist (`pre_tool_bash/dispatch.rs` reads `tool_input["command"]`) sees an
/// empty command and silently allows `rm -rf /`. SOURCE: <https://cursor.com/docs/hooks>
fn cursor_tool_input(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Option<std::collections::HashMap<String, serde_json::Value>> {
    let mut map = std::collections::HashMap::new();
    // Cursor's preToolUse nests tool args under tool_input (CC-shaped);
    // without lifting it the pre-write gate sees an empty file_path and
    // fail-closes EVERY Write/Edit. Lift nested first so the flat
    // security-critical fields below win on key collision.
    if let Some(nested) = obj.get("tool_input").and_then(serde_json::Value::as_object) {
        for (k, v) in nested {
            if !v.is_null() {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    // Cursor Write/StrReplace dialect names: path -> file_path,
    // contents -> content. Canonicalize for the gates.
    for (native, canonical) in [("path", "file_path"), ("contents", "content")] {
        if !map.contains_key(canonical) {
            let v = map.get(native).cloned()
                .or_else(|| obj.get(native).filter(|x| !x.is_null()).cloned());
            if let Some(v) = v {
                map.insert(canonical.to_owned(), v);
            }
        }
    }
    // Shell command (beforeShellExecution) — the security-critical field.
    for key in ["command", "file_path", "content"] {
        if let Some(v) = obj.get(key).filter(|v| !v.is_null()) {
            map.insert(key.to_owned(), v.clone());
        }
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

/// Cursor nests the tool name under `metadata.tool_name` (falling back to a
/// top-level `tool_name` if a future payload promotes it). When ABSENT, derive it
/// from the event: `beforeShellExecution` IS the Bash tool, so the canonical
/// `tool_name` must be "Bash" or `pre_tool::run` won't route to the destructive
/// blocklist (it dispatches on `tool_name == "Bash"`). Cursor's shell payload
/// identifies the tool by event name, not a `tool_name` field, so without this the
/// blocklist never sees a shell command.
fn cursor_tool_name(
    obj: &serde_json::Map<String, serde_json::Value>,
    raw_event: &str,
) -> String {
    let explicit = obj
        .get("metadata")
        .and_then(|m| m.get("tool_name"))
        .or_else(|| obj.get("tool_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !explicit.is_empty() {
        return explicit.to_owned();
    }
    match raw_event {
        "beforeShellExecution" => "Bash",
        "beforeReadFile" => "Read",
        "afterFileEdit" => "Edit",
        _ => "",
    }
    .to_owned()
}
