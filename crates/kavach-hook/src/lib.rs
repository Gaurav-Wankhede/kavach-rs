use std::io::{self, BufRead, Write};

use kavach_types::{HookInput, HookResponse};
use serde::Serialize;

pub mod cc21;
pub mod context;
pub mod lifecycle;
pub mod severity;
pub mod toon;

pub use severity::GateSeverity;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// Re-export context functions at crate root for backwards compatibility
pub use context::{
    CACHE_BOUNDARY_MARKER, context_block, current_month, current_week, current_year,
    exit_approve_ctx, exit_block_ctx, exit_modify_ctx, exit_modify_ctx_with_module,
    exit_session_end_ctx, exit_user_prompt_submit_ctx, today,
};

// Re-export CC 2.1 functions at crate root
pub use cc21::{
    exit_notification_context, exit_notification_with_sequence, exit_post_tool_block,
    exit_post_tool_context, exit_post_tool_failure_context, exit_post_tool_trimmed,
    exit_pre_tool_allow, exit_pre_tool_ask, exit_pre_tool_deny, exit_prompt_context,
    exit_session_start_context, exit_session_start_full, exit_stop_block, exit_stop_context,
};

// Re-export lifecycle functions at crate root
pub use lifecycle::{
    exit_elicitation_decline, exit_permission_allow, exit_permission_deny,
    exit_permission_request_allow, exit_permission_request_deny, exit_session_end,
    exit_subagent_start, exit_subagent_stop, exit_user_prompt_submit,
    exit_user_prompt_submit_silent,
};

/// What the hook wants to do after outputting JSON.
#[derive(Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-cli cmd/gates.rs; non_exhaustive => E0004"
)]
pub enum HookAction {
    Done,
    Error,
}

// --- Input ---

/// Read a hook input payload from stdin.
///
/// # Errors
/// Returns `Err` with a human-readable message on stdin read failure or JSON parse failure.
pub fn read_hook_input() -> Result<HookInput, String> {
    let stdin = io::stdin();
    read_hook_input_from(stdin.lock())
}

/// Read a hook input payload from an arbitrary reader.
///
/// # Errors
/// Returns `Err` with a human-readable message on read failure or JSON parse failure.
pub fn read_hook_input_from<R: BufRead>(reader: R) -> Result<HookInput, String> {
    let mut buf = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| format!("read error: {e}"))?;
        buf.push(line);
    }
    let raw = buf.join("\n");
    parse_hook_input(&raw)
}

/// Parse a raw hook-input payload into [`HookInput`], tolerating explicit JSON
/// `null` on any field.
///
/// A present `null` (not an absent key) otherwise hits serde's typed field and
/// fails with `invalid type: null, expected a string` — the bug that blocked
/// Cursor, whose payloads carry `null` for fields a Claude-Code field expects as
/// a string. We pre-scrub every top-level `null` to "absent" so `#[serde(default)]`
/// fills it, covering all fields at once instead of one `null_string` attr each.
///
/// # Errors
/// Returns `Err` only when the payload is not a JSON object at all (truly
/// unparseable) — a shape no amount of field-defaulting can recover.
pub fn parse_hook_input(raw: &str) -> Result<HookInput, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))?;
    if let Some(obj) = value.as_object_mut() {
        // Drop every top-level explicit `null`; an absent key triggers
        // `#[serde(default)]`, a present `null` does not. Nested nulls (inside
        // tool_input/attachments, typed Option/Value) are already tolerated.
        obj.retain(|_, v| !v.is_null());
    }
    serde_json::from_value(value).map_err(|e| format!("JSON parse error: {e}"))
}

/// Read a hook input payload, emitting an error response on failure.
///
/// # Errors
/// Returns `Err(HookAction::Error)` after writing an error response when the
/// input cannot be read or parsed.
pub fn must_read_hook_input() -> Result<HookInput, HookAction> {
    match read_hook_input() {
        Ok(input) => Ok(input),
        Err(e) => {
            output_error(&e);
            Err(HookAction::Error)
        }
    }
}

// --- Output helpers ---

#[expect(
    clippy::print_stderr,
    reason = "hook last-ditch diagnostic path; no tracing subscriber in hook binary, broken-pipe surfaces via SIGPIPE (RFC 1869)"
)]
pub(crate) fn write_json<T: Serialize>(val: &T) {
    let json = match serde_json::to_string(val) {
        Ok(j) => j,
        Err(e) => {
            // eprintln! is the canonical Rust stderr macro (RFC 1869).
            // No Result to handle; broken-pipe surfaces via SIGPIPE.
            eprintln!("kavach: serialization error: {e}");
            r#"{"decision":"block","reason":"hook internal error: serialization failed"}"#.into()
        }
    };
    if writeln!(io::stdout().lock(), "{json}").is_err() {
        eprintln!("kavach: stdout write failed");
    }
}

pub fn output(resp: &HookResponse) {
    write_json(resp);
}
pub fn output_error(msg: &str) {
    output(&HookResponse::new_block(&format!("error: {msg}")));
}
pub fn approve(reason: &str) {
    output(&HookResponse::new_approve(reason));
}
pub fn block(reason: &str) {
    output(&HookResponse::new_block(reason));
}
pub fn modify(reason: &str, ctx: &str) {
    output(&HookResponse::new_modify(reason, ctx));
}

// --- Exit helpers ---

#[must_use]
pub fn exit_silent() -> HookAction {
    approve("ok");
    HookAction::Done
}

#[must_use]
pub fn exit_approve(reason: &str) -> HookAction {
    approve(reason);
    HookAction::Done
}

#[must_use]
pub fn exit_block(reason: &str) -> HookAction {
    block(reason);
    HookAction::Done
}

#[must_use]
pub fn exit_modify(reason: &str, ctx: &str) -> HookAction {
    modify(reason, ctx);
    HookAction::Done
}
