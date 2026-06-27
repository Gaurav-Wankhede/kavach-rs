use std::io::{self, BufRead, Write};
use kavach_types::{HookInput, HookResponse};
use serde::Serialize;
pub mod cc21;
pub mod context;
pub mod lifecycle;
pub mod severity;
pub mod toon;
pub mod vendor;
pub use severity::GateSeverity;
pub use vendor::{SchemaSource, Vendor};
#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
// Re-export context functions at crate root for backwards compatibility
pub use context::{
    CACHE_BOUNDARY_MARKER, context_block, current_month, current_week, current_year,
    exit_approve_ctx, exit_block_ctx, exit_modify_ctx, exit_modify_ctx_with_module,
    exit_session_end_ctx, exit_user_prompt_submit_ctx, now_full, today, today_full,
};
// Re-export CC 2.1 functions at crate root
pub use cc21::{
    exit_notification_context, exit_notification_with_sequence, exit_post_tool_block,
    exit_post_tool_context, exit_post_tool_failure_context, exit_post_tool_trimmed,
    exit_pre_tool_allow, exit_pre_tool_ask, exit_pre_tool_deny, exit_prompt_context,
    exit_prompt_submit_block, exit_session_start_context, exit_session_start_full, exit_stop_block,
    exit_stop_context,
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
/// Read a hook input payload through the NATIVE EDGE for a resolved harness.
///
/// Reads stdin once, resolves the vendor (hybrid: `explicit` `--vendor` wins,
/// else `$KAVACH_HARNESS`, else payload auto-detect, else Claude Code), and
/// lowers that vendor's native payload into the canonical [`HookInput`]. The
/// resolved [`Vendor`] is returned so the caller can render its native output.
///
/// # Errors
/// Returns `Err` with the resolved vendor attached when the payload is not a
/// JSON object at all, so the caller can still emit a vendor-native failure.
pub fn read_hook_input_native(
    explicit: Option<&str>,
) -> Result<(Vendor, HookInput), (Vendor, String)> {
    let stdin = io::stdin();
    let mut buf = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => buf.push(l),
            Err(e) => return Err((Vendor::ClaudeCode, format!("read error: {e}"))),
        }
    }
    let raw = buf.join("\n");
    let vendor = Vendor::resolve(explicit, &raw);
    // Arm the native output sink BEFORE the gate runs, so every self-emitted
    // verdict (allow/block/ask) is rendered in this harness's dialect. Without
    // this, a Cursor gate's allow emitted Claude Code's body and Cursor read its
    // absent `continue`/`permission` as null — the original wedge.
    match vendor.lower(&raw) {
        Ok(input) => {
            // Arm the sink with the resolved vendor AND the canonical event, so a
            // gate emitting a bare verdict still renders into the right native
            // contract (e.g. Cursor's Stop → `{continue, followupMessage}`).
            set_output_context(vendor, &input.hook_event_name);
            // Arm the session-id context so EVERY gate's session load keys the
            // durable row + stop-reblock counter per conversation — Cursor sets
            // no KAVACH_SESSION_ID env; it carries the id as conversation_id,
            // already lowered into input.session_id. No-op for the empty id.
            kavach_session::set_session_context(&input.session_id);
            Ok((vendor, input))
        }
        Err(e) => {
            set_output_context(vendor, "");
            Err((vendor, e))
        }
    }
}
/// Emit a canonical [`HookResponse`] in `vendor`'s NATIVE output contract and
/// return the process exit code that vendor expects (Codex blocks via exit 2).
#[must_use = "the returned exit code must be passed to process::exit for Codex's exit-2 block"]
pub fn output_native(vendor: Vendor, resp: &HookResponse) -> i32 {
    let json = vendor.render(resp);
    emit_or_fail_closed(&json);
    if resp.decision == "block" {
        vendor.block_exit_code()
    } else {
        0
    }
}
/// Write `json` to stdout, or fail CLOSED if the write errors.
///
/// stdout is the gate's ONLY verdict channel to the host. A swallowed write
/// loses the verdict; the host then reads absent output as "allow" — fail-OPEN
/// on an enforcement gate. So a non-pipe write error exits the process with
/// `EXIT_HOOK_ERROR` (2): the host treats a non-zero hook exit as "do not
/// proceed", never as allow. A broken pipe means the host already closed the
/// channel (it is gone), so there is nothing left to fail closed toward —
/// diagnose and return so the caller's exit code still flows.
#[expect(
    clippy::print_stderr,
    clippy::exit,
    reason = "fail-closed hook path: no tracing sink in the hook binary; process exit IS the verdict-undeliverable signal to the host"
)]
fn emit_or_fail_closed(json: &str) {
    let Err(e) = writeln!(io::stdout().lock(), "{json}") else {
        return;
    };
    if e.kind() == io::ErrorKind::BrokenPipe {
        // Host already closed the channel — it is gone; nothing to fail toward.
        eprintln!("kavach: stdout closed by host (broken pipe)");
        return;
    }
    eprintln!("kavach: stdout write failed ({e}) — fail-closed, host must not proceed");
    std::process::exit(EXIT_HOOK_ERROR);
}
/// Hook-error exit code. A non-zero hook exit tells every supported host "the
/// gate did not produce a verdict — do not proceed", which is the fail-closed
/// outcome when the verdict cannot be delivered on stdout.
const EXIT_HOOK_ERROR: i32 = 2;
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
std::thread_local! {
    /// The harness whose NATIVE dialect this thread's gate output must speak.
    /// The edge (`read_hook_input_native`) sets it once per invocation; every
    /// gate that self-emits via [`output`] is then rendered in that dialect
    /// WITHOUT the engine or any gate knowing a non-Claude-Code harness exists.
    /// Defaults to [`Vendor::ClaudeCode`] — the canonical, unset behavior.
    static OUTPUT_VENDOR: std::cell::Cell<Vendor> = const { std::cell::Cell::new(Vendor::ClaudeCode) };
    /// The canonical event this thread's gate is answering (e.g. "Stop",
    /// "UserPromptSubmit"). The edge sets it from the lowered input so a native
    /// renderer can pick the right output contract (Cursor's `Stop` =
    /// `{continue, followupMessage}`) even when a gate emits a bare verdict that
    /// doesn't stamp the event itself.
    static OUTPUT_EVENT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}
/// Set the native dialect + answered event for all subsequent gate output on
/// THIS thread.
///
/// Called by the native edge after vendor resolution so a gate's self-emitted
/// verdict (allow/block/ask) is rendered in the caller's contract — Cursor's
/// `{continue,permission}`, Codex's CC-compatible body — instead of always
/// Claude Code's. Keeps the engine/gates vendor-blind: they call [`output`]
/// exactly as before.
pub fn set_output_context(vendor: Vendor, event: &str) {
    OUTPUT_VENDOR.with(|v| v.set(vendor));
    OUTPUT_EVENT.with(|e| e.replace(event.to_owned()));
}
/// The native dialect currently selected for this thread's gate output.
#[must_use]
pub fn output_vendor() -> Vendor {
    OUTPUT_VENDOR.with(std::cell::Cell::get)
}
/// The canonical event the current thread's gate is answering ("" if unset).
#[must_use]
pub fn output_event() -> String {
    OUTPUT_EVENT.with(|e| e.borrow().clone())
}
#[expect(
    clippy::print_stderr,
    reason = "hook last-ditch diagnostic path; no tracing subscriber in hook binary, broken-pipe surfaces via SIGPIPE (RFC 1869)"
)]
pub fn output(resp: &HookResponse) {
    // Render the verdict in the thread's resolved native dialect, scoped to the
    // event being answered. For Claude Code (the default) this is byte-identical
    // to `write_json(resp)`; for Cursor/Codex it translates the canonical body
    // into their contract — the single chokepoint that makes EVERY gate's happy
    // path natively correct.
    let json = output_vendor().render_for(resp, &output_event());
    if writeln!(io::stdout().lock(), "{json}").is_err() {
        eprintln!("kavach: stdout write failed");
    }
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
