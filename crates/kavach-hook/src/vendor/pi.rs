// split: Pi (earendil-works/pi coding-agent) native edge. Pi extensions subscribe
// to lifecycle events via pi.on(event, cb); a tool_call handler BLOCKS by returning
// `{ block: true, reason? }` and ALLOWS by returning nothing. agent_end is Pi's
// Stop-equivalent (fires once per prompt after all turns) so the autonomous loop's
// AUTO_CONTINUE reaches Pi too. Extensions auto-discover at ~/.pi/agent/extensions/.
//! Pi coding-agent (`earendil-works/pi`) native edge.
//!
//! Pi's hook contract differs from the CC family on OUTPUT: a `tool_call` handler
//! returns `{"block":true,"reason":…}` to deny and returns NOTHING to allow —
//! there is no exit-code signal. Input is CC-compatible (`PascalCase`-ish events,
//! stdin JSON), so lowering is the canonical null-tolerant parse. The TS extension
//! shim ([`crate`] installs it) maps Pi's `pi.on` events onto canonical gate names
//! (`tool_call`→`PreToolUse`, `agent_end`→`Stop`, …) before shelling to kavach.
//! SOURCE: research.pi-extension-hook-api ·
//! github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md.

use kavach_types::HookResponse;
use serde::Serialize;

/// Lower a raw Pi payload into the canonical [`HookInput`](kavach_types::HookInput).
///
/// The TS shim forwards a CC-shaped JSON object (it has already mapped Pi's event
/// name onto the canonical one), so this is the canonical null-tolerant parse;
/// extra Pi fields are ignored without error.
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<kavach_types::HookInput, String> {
    crate::parse_hook_input(raw_payload)
}

/// Pi's `tool_call` block object. A canonical block/ask becomes `{block:true}`;
/// an allow renders as the empty object `{}` which the shim returns as `undefined`
/// (Pi reads "no block" as "proceed"). `reason` carries the gate message — and the
/// `AUTO_CONTINUE` text on an `agent_end`/Stop block (Pi surfaces it to the agent).
#[derive(Debug, Serialize)]
struct PiDecision {
    /// Present + `true` ONLY on a deny; omitted entirely on allow so the rendered
    /// object is `{}` (→ `undefined` in the shim).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    block: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// True when this verdict denies/asks. Mirrors the cross-vendor block test: a
/// top-level `decision == "block"` (stop / post-tool) OR a `PreToolUse` deny in
/// `hook_specific_output.permission_decision`. `ask` is surfaced as a block (Pi
/// has no distinct ask — never silently allow).
fn is_denied(resp: &HookResponse) -> bool {
    if resp.decision == "block" {
        return true;
    }
    resp.hook_specific_output
        .as_ref()
        .is_some_and(|h| h.permission_decision == "deny" || h.permission_decision == "ask")
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

/// Render a canonical verdict into Pi's native `tool_call` return contract:
/// `{block:true,reason}` to deny, or `{}` (→ `undefined`) to allow.
#[must_use]
pub fn render(resp: &HookResponse) -> String {
    let denied = is_denied(resp);
    let msg = message(resp);
    let out = PiDecision {
        block: denied,
        // Carry the reason on a block (the agent-visible deny / AUTO_CONTINUE text);
        // drop it on allow so the object is exactly `{}`.
        reason: (denied && !msg.is_empty()).then_some(msg),
    };
    serde_json::to_string(&out).unwrap_or_else(|_| fail_closed())
}

/// Pi's failure default: fail CLOSED (block) — symmetric with Codex/Claude Code.
/// An internal serialization error must not silently allow a gated action.
#[must_use]
pub fn fail_closed() -> String {
    r#"{"block":true,"reason":"kavach: hook internal error"}"#.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_renders_block_true_with_reason() {
        let resp = HookResponse::new_block("[AUTO_CONTINUE] keep working");
        let json = render(&resp);
        assert!(json.contains(r#""block":true"#), "{json}");
        assert!(json.contains("AUTO_CONTINUE"), "{json}");
    }

    #[test]
    fn approve_renders_empty_object_no_block() {
        // Allow → `{}` so the shim returns `undefined` (Pi proceeds). The `block`
        // key must be ABSENT, not `false`, so Pi never sees a falsey-but-present flag.
        let resp = HookResponse::new_approve("");
        let json = render(&resp);
        assert_eq!(json, "{}", "allow must render as the empty object: {json}");
        assert!(!json.contains("block"), "allow must omit block: {json}");
    }

    #[test]
    fn lower_tolerates_pi_extra_fields() {
        let raw = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","pi_turn":7}"#;
        let input = lower(raw).unwrap();
        assert_eq!(input.hook_event_name, "PreToolUse");
        assert_eq!(input.tool_name, "Bash");
    }
}
