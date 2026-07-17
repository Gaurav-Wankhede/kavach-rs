// split: Kimi Code native edge. Kimi mirrors the Claude Code hook contract (same
// PascalCase event names, snake_case stdin JSON), so input lowering is the
// canonical null-tolerant parse; the ONE native difference is the exit-code-2
// block signal on output (config format — TOML [[hooks]] — is install-side only).
//! Kimi Code CLI native edge.
//!
//! Kimi deliberately mirrors Claude Code's hook contract.
//!
//! Identical event names (`PreToolUse`, `Stop`, `UserPromptSubmit`…), snake_case
//! stdin JSON (`hook_event_name`, `session_id`, `tool_input`…), and the same
//! `hookSpecificOutput.permissionDecision` blocking JSON. The ONE native
//! divergence is the OUTPUT failure signal: Kimi treats hook **exit code 2** as a
//! hard block, with stderr as the reason (other non-zero exits fail OPEN).
//! SOURCE: <https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html>

use kavach_types::HookInput;

/// Lower a raw Kimi payload into the canonical [`HookInput`].
///
/// Kimi's input is Claude-Code-compatible, so this is the same null-tolerant
/// parse — any extra Kimi fields are ignored by the canonical struct without
/// error. (A Kimi payload is also shape-indistinguishable from Claude Code, so
/// payload sniffing cannot detect it; the installed hook pins `--vendor kimi`.)
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all.
pub fn lower(raw_payload: &str) -> Result<HookInput, String> {
    crate::parse_hook_input(raw_payload)
}

/// Render a canonical verdict into Kimi's native output.
///
/// The body is the same Claude-Code JSON Kimi understands; the exit-code-2 block
/// signal is applied by the caller via [`super::Vendor::block_exit_code`].
#[must_use]
pub fn render(resp: &kavach_types::HookResponse) -> String {
    serde_json::to_string(resp)
        .unwrap_or_else(|_| r#"{"decision":"block","reason":"hook internal error"}"#.to_owned())
}
