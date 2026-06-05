// split: Codex native edge. Codex mirrors the Claude Code hook contract (same
// event names, stdin JSON), so input lowering is the canonical null-tolerant
// parse; the ONE native difference is the exit-code-2 block signal on output.
//! `OpenAI` Codex CLI native edge.
//!
//! Codex deliberately mirrors Claude Code's hook contract.
//!
//! Identical event names (`PreToolUse`, `Stop`, `UserPromptSubmit`…), stdin JSON,
//! and it even exports `CLAUDE_PLUGIN_ROOT` for plugin compatibility. It adds a
//! turn-scope `turn_id` and a `permission_mode` enum, which the canonical
//! [`HookInput`] tolerates as extra fields. The ONE native divergence is the
//! OUTPUT: Codex treats hook **exit code 2** as a hard block (plus the JSON body).
//! SOURCE: <https://developers.openai.com/codex/hooks>

use kavach_types::HookInput;

/// Lower a raw Codex payload into the canonical [`HookInput`].
///
/// Codex's input is Claude-Code-compatible, so this is the same null-tolerant
/// parse — extra Codex fields (`turn_id`, `permission_mode`) are ignored by the
/// canonical struct without error.
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<HookInput, String> {
    crate::parse_hook_input(raw_payload)
}

/// Render a canonical verdict into Codex's native output.
///
/// The body is the same Claude-Code JSON Codex understands; the exit-code-2 block
/// signal is applied by the caller via [`super::Vendor::block_exit_code`].
#[must_use]
pub fn render(resp: &kavach_types::HookResponse) -> String {
    serde_json::to_string(resp)
        .unwrap_or_else(|_| r#"{"decision":"block","reason":"hook internal error"}"#.to_owned())
}
