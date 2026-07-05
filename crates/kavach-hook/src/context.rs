use crate::{HookAction, modify, output};
use kavach_types::{HookResponse, HookSpecificOutput};

/// Return today's date as YYYY-MM-DD using local time (bare ISO form).
///
/// Use for stored timestamps (e.g. RAG `built_at`) where a machine-parseable
/// date is needed. For agent-visible CONTEXT, prefer [`today_full`] (weekday-aware).
#[must_use]
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Return today's date WITH the weekday name, e.g. `"Tuesday, 2026-06-16"`.
///
/// This is the agent-visible temporal anchor injected into the Tabula Rasa
/// (session-start) context and EVERY pre-task gate. The weekday sharpens "as of
/// today" awareness so web research is scoped to the precise current day, not a
/// stale training-weight assumption. `%A` is the full local weekday name.
#[must_use]
pub fn today_full() -> String {
    chrono::Local::now().format("%A, %Y-%m-%d").to_string()
}

/// Return the exact current instant: weekday, date, time WITH the IANA offset,
/// e.g. `"Tuesday, 2026-06-16 14:07:52 +05:30"`.
///
/// This is the precise temporal anchor (Time + Date + Day) the research advisory
/// injects so the agent scopes web research to the EXACT current moment, never a
/// stale training-weight assumption. The offset (`%z`) makes "now" unambiguous
/// across hosts. All fields are read live from the system clock — nothing here is
/// hardcoded, so the anchor is always correct whenever the gate fires.
#[must_use]
pub fn now_full() -> String {
    chrono::Local::now()
        .format("%A, %Y-%m-%d %H:%M:%S %z")
        .to_string()
}

/// Return the current year (e.g., 2026).
/// Uses `unsigned_abs()` — year is always a non-negative i32, no fallible cast needed.
#[must_use]
pub fn current_year() -> u32 {
    use chrono::Datelike;
    chrono::Local::now().year().unsigned_abs()
}

/// Return the current month (1-12).
#[must_use]
pub fn current_month() -> u32 {
    use chrono::Datelike;
    chrono::Local::now().month()
}

/// Return the current ISO week number (1-53).
#[must_use]
pub fn current_week() -> u32 {
    use chrono::Datelike;
    chrono::Local::now().iso_week().week()
}

/// Prompt caching hint: marks the boundary between static and dynamic content.
///
/// Content BEFORE this marker is cacheable (skills, CLAUDE.md, static rules).
/// Content AFTER this marker is dynamic (dates, session IDs, turn counts).
/// Claude Code uses this to place `cache_control` breakpoints optimally.
/// ALGO: Prompt caching separation | SEARCHED: 2026-04
/// Source: platform.claude.com/docs/en/build-with-claude/prompt-caching
pub const CACHE_BOUNDARY_MARKER: &str = "\n<!-- DYNAMIC_CONTENT_BELOW -->\n";

/// Build a context block string. Name is kept as-is (NO case change).
/// ```text
/// [name]
/// key1: value1
/// key2: value2
/// ```
#[must_use]
pub fn context_block(name: &str, kvs: &[(&str, &str)]) -> String {
    let mut out = format!("[{name}]\n");
    for (k, v) in kvs {
        // Direct push_str chain: no throwaway format! alloc, no Result to handle.
        out.push_str(k);
        out.push_str(": ");
        out.push_str(v);
        out.push('\n');
    }
    out
}

/// `PreToolUse` allow with context GATE block, weekday-aware date auto-injected.
#[must_use]
pub fn exit_approve_ctx(gate: &str) -> HookAction {
    let d = today_full();
    let context = context_block(gate, &[("status", "allow"), ("date", &d)]);
    // SOURCE: crates/kavach-toon/src/caveman.rs (public compress() API, lossless-preserving)
    let context = kavach_toon::caveman::compress(&context, kavach_toon::caveman::Level::Full);
    let resp = HookResponse::new_pre_tool_use_with_context(gate, &context);
    output(&resp);
    HookAction::Done
}

/// `PreToolUse` deny with context BLOCK block, date auto-injected.
/// Sends plain reason as permissionDecisionReason, context block as additionalContext.
#[must_use]
pub fn exit_block_ctx(gate: &str, reason: &str) -> HookAction {
    let d = today_full();
    let context = context_block(
        gate,
        &[("status", "block"), ("reason", reason), ("date", &d)],
    );
    let resp = HookResponse {
        hook_specific_output: Some(HookSpecificOutput {
            hook_event_name: "PreToolUse".into(),
            permission_decision: "deny".into(),
            permission_decision_reason: reason.into(),
            additional_context: context,
            ..Default::default()
        }),
        ..Default::default()
    };
    output(&resp);
    HookAction::Done
}

/// Legacy modify with context injection, date auto-injected via kvs.
#[must_use]
pub fn exit_modify_ctx(gate: &str, kvs: &[(&str, &str)]) -> HookAction {
    let context = context_block(gate, kvs);
    modify(gate, &context);
    HookAction::Done
}

/// Modify with context injection + lazy-loaded module content.
#[must_use]
pub fn exit_modify_ctx_with_module(
    gate: &str,
    kvs: &[(&str, &str)],
    module_content: &str,
) -> HookAction {
    let mut context = context_block(gate, kvs);
    context.push_str("\n[MODULE:LAZY_LOADED]\n");
    context.push_str(module_content);
    modify(gate, &context);
    HookAction::Done
}

/// `UserPromptSubmit` with context block, date auto-injected.
#[must_use]
pub fn exit_user_prompt_submit_ctx(gate: &str, kvs: &[(&str, &str)]) -> HookAction {
    let d = today_full();
    let mut all_kvs: Vec<(&str, &str)> = kvs.to_vec();
    all_kvs.push(("date", &d));
    let context = context_block(gate, &all_kvs);
    crate::exit_user_prompt_submit(&context)
}

/// `SessionEnd` with context block, date auto-injected.
#[must_use]
pub fn exit_session_end_ctx(kvs: &[(&str, &str)]) -> HookAction {
    let d = today_full();
    let mut all_kvs: Vec<(&str, &str)> = kvs.to_vec();
    all_kvs.push(("date", &d));
    let context = context_block("SESSION_END", &all_kvs);
    crate::exit_session_end(&context)
}
