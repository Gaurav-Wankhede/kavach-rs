// split: Google Antigravity (agy) native edge — successor to the retired Gemini
// CLI (gemini CLI retired 2026-06-18; agy is the migration target). Antigravity
// hooks receive JSON on stdin and read a JSON object on stdout carrying a
// `decision` key set to "allow" or "deny" — the block signal is the JSON body
// (NOT exit code 2). Its `PreToolUse` event maps directly from Gemini's
// `BeforeTool`. Config lives at ~/.gemini/config/hooks.json (CHANGELOG v1.0.8).
//! Google Antigravity CLI (`agy`) native edge.
//!
//! Antigravity's hook contract is JSON-stdin → JSON-stdout with a top-level
//! `{"decision":"allow"|"deny","reason":...}` object; continuation/blocking is
//! driven by `decision`, not an exit code. Event names are `PascalCase` and largely
//! CC-compatible (`PreToolUse`, `PostToolUse`, `SessionStart`, `Stop`), so input
//! lowering is the canonical null-tolerant parse; only the OUTPUT shape differs.
//! SOURCES: <https://antigravity.google/docs/hooks> ·
//! github.com/google-antigravity/antigravity-cli CHANGELOG (hooks.json path).

use kavach_types::HookResponse;
use serde::Serialize;

/// Lower a raw Antigravity payload into the canonical [`HookInput`].
///
/// agy's input is CC-compatible (`PascalCase` events, stdin JSON); extra agy fields
/// are ignored by the canonical struct without error.
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<kavach_types::HookInput, String> {
    crate::parse_hook_input(raw_payload)
}

/// agy's `decision` output object. A canonical block/ask becomes `deny`; anything
/// else is `allow`. `reason` carries the gate's message (and `AUTO_CONTINUE` text
/// on a Stop block — agy reads `reason` as the agent-visible continuation).
#[derive(Debug, Serialize)]
struct AntigravityDecision {
    decision: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// True when this verdict denies/asks. Mirrors the cross-vendor block test: a
/// top-level `decision == "block"` (stop/post-tool) OR a `PreToolUse` deny carried
/// in `hook_specific_output.permission_decision`. `ask` is surfaced as `deny`
/// (agy has no distinct ask — never silently allow).
fn is_denied(resp: &HookResponse) -> bool {
    if resp.decision == "block" {
        return true;
    }
    resp.hook_specific_output.as_ref().is_some_and(|h| {
        h.permission_decision == "deny" || h.permission_decision == "ask"
    })
}

/// The agent-visible message: the verdict `reason`, else injected context
/// (`system_message` / `additional_context` / nested `hook_specific_output`).
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

/// Render a canonical verdict into agy's native `{decision, reason}` output.
#[must_use]
pub fn render(resp: &HookResponse) -> String {
    let denied = is_denied(resp);
    let msg = message(resp);
    let out = AntigravityDecision {
        decision: if denied { "deny" } else { "allow" },
        reason: (!msg.is_empty()).then_some(msg),
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_closed())
}

/// agy's failure default: fail CLOSED (deny) — symmetric with Codex/Claude Code.
/// An internal serialization error must not silently allow a gated action.
#[must_use]
pub fn fail_closed() -> String {
    r#"{"decision":"deny","reason":"kavach: hook internal error"}"#.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_renders_deny_with_reason() {
        let resp = HookResponse::new_block("[AUTO_CONTINUE] keep working");
        let json = render(&resp);
        assert!(json.contains(r#""decision":"deny""#), "{json}");
        assert!(json.contains("AUTO_CONTINUE"), "{json}");
    }

    #[test]
    fn approve_renders_allow() {
        let resp = HookResponse::new_approve("");
        let json = render(&resp);
        assert!(json.contains(r#""decision":"allow""#), "{json}");
    }

    #[test]
    fn lower_tolerates_agy_extra_fields() {
        let raw = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","agy_session":"x"}"#;
        let input = lower(raw).unwrap();
        assert_eq!(input.hook_event_name, "PreToolUse");
        assert_eq!(input.tool_name, "Bash");
    }
}
