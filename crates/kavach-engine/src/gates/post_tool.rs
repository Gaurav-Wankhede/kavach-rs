//! Post-tool umbrella gate: context injection + research/skill tracking.
//!
//! `trim` handles the budget-trim short-circuit + its inline state tracking;
//! `skill` records Skill invocations. This hub runs the ordered early-return
//! stages, then dispatches to the per-tool handler.
mod skill;
mod trim;

use kavach_types::HookInput;

use crate::error::EngineError;
use crate::gates::{post_tool_bash, post_tool_read, post_tool_research, post_tool_task};

/// Post-tool umbrella gate: context injection + research tracking + skill tracking.
/// Single session instance passed to all sub-handlers to prevent double-load race.
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let mut session = kavach_session::get_or_create_session();
    session.increment_turn();
    session.clear_failure();

    // Refresh session lease heartbeat on every post-tool to prevent mid-op reclaim.
    // See decision.engine.session_lease_heartbeat.
    let _renewed = crate::gates::stop_dispatch::renew_my_leases();

    // Scan assistant message for [RCA] block and persist to session.
    // Wires Gap 1 multi-turn RCA tracking — flag carries forward within intent window.
    // SOURCE: code-reviewer (Gap 1) — detection→persist must be deterministic, every turn.
    if !session.rca_satisfied()
        && super::pre_write_rca_guard::has_rca_block(&input.last_assistant_message)
    {
        session.mark_rca_present();
    }

    // Check trimming FIRST — if needed, skip handlers (they write to stdout)
    // and do state tracking inline. Only ONE JSON output per hook invocation.
    if let Some(trimmed_output) = trim::check_trim(input) {
        trim::track_state_only(input, &mut session);
        let context = kavach_hook::context_block("POST_TOOL:TRIMMED", &[]);
        drop(kavach_hook::exit_post_tool_trimmed(
            &trimmed_output,
            &context,
        ));
        return Ok(());
    }

    // §LSP-FIRST producer — record any LSP tool call into session.lsp_diag_seen
    // BEFORE dispatching to per-tool handlers. The check is name-prefix based so
    // it covers native + cclsp + Anthropic-official LSP plugins uniformly.
    // SOURCE: ~/.claude/CLAUDE.md §LSP-FIRST + code.claude.com/docs/en/mcp.
    super::post_tool_lsp::handle(input, &mut session);

    if let Some(done) = emit_advisory_recovery(&mut session) {
        return done;
    }

    dispatch_tool(input, &mut session)
}

/// One-shot advisory-recovery feedback: if a `PreToolUse` `P2Advise` gate stashed a
/// mechanical fix, surface it as additionalContext so the next turn auto-heals
/// instead of permission-seeking. Cleared after emission to prevent rumination.
/// SOURCE: roadmap.unit.agent-feedback-loop. Returns `Some(_)` if it emitted.
fn emit_advisory_recovery(
    session: &mut kavach_session::SessionState,
) -> Option<Result<(), EngineError>> {
    if session.last_advisory_gate.is_empty() {
        return None;
    }
    let ctx = format!(
        "[ADVISORY_RECOVERY:{gate}] {fix}",
        gate = session.last_advisory_gate,
        fix = session.last_advisory_fix,
    );
    session.last_advisory_gate.clear();
    session.last_advisory_fix.clear();
    session.save().ok();
    if super::turn_relay::should_relay() {
        super::turn_relay::queue_advisory(session, &ctx);
        drop(kavach_hook::exit_silent());
        return Some(Ok(()));
    }
    drop(kavach_hook::exit_post_tool_context(&ctx));
    Some(Ok(()))
}

/// Dispatch to the per-tool post handler by tool name.
fn dispatch_tool(
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Result<(), EngineError> {
    match input.tool_name.as_str() {
        "WebSearch" | "WebFetch" => post_tool_research::handle(input, session),
        "Skill" => skill::handle_skill_done(input, session),
        "Task" => post_tool_task::handle(input, session),
        "Bash" => post_tool_bash::handle(input, session),
        "Read" | "Glob" | "Grep" => post_tool_read::handle(input, session),
        _ => {
            drop(kavach_hook::exit_silent());
            Ok(())
        }
    }
}
