//! Guard: the genuine clean exit. No pending tasks, no review needed — reset
//! the pending-work re-block breaker and emit either a STOP context (when a
//! reason or semver advisory exists) or a silent exit. Always Breaks (terminal).

use core::ops::ControlFlow;

use super::super::shared::StopCtx;
use crate::gates::bandit::emit;
use kavach_patterns::bandit_log::{BanditContext, GateAction};

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // Layer-A bandit log: a clean exit is the Stop gate's `Allow` action. Reward
    // is None here — it is back-filled when the 3-witness verify resolves. Pure
    // logging, fire-and-forget; never affects whether the stop proceeds.
    emit::emit_decision(
        &ctx.session.session_id,
        BanditContext::new(
            "stop",
            "Stop",
            "",
            0,
            "",
            u32::try_from(ctx.session.turn_count).unwrap_or(0),
        ),
        GateAction::Allow,
        1.0,
        None,
    );
    // Work is genuinely done at the dispatch level — reset the pending-work
    // re-block breaker before composing the terminal verdict.
    ctx.session.clear_stop_reblock();

    // BUG FIX (drained-board → plan-check): the dispatch tiers found nothing
    // runnable, but a drained board is NOT a finished plan. Emit the SAME
    // census-aware verdict the retry terminal uses (`[ALL_BLOCKED]` when every
    // remainder is owner-gated, else the board-drained `[PLAN]` nudge) — never a
    // silent stop that hides un-built plan phases. This terminal used to
    // `exit_silent()` on an empty board, so an empty kanban stopped the loop
    // immediately without ever checking the active `[PLAN]`. Loop-safe:
    // `exit_stop_context` ALLOWS the stop, so the advisory can never spin.
    let mut full = super::drained::drained_terminal_context(&ctx.session.project);

    // Ride-alongs (all advisory; the stop still proceeds): an explicit user stop
    // reason, the semver advisory, and the U3 capture-finding nudge (a decision
    // settled in prose but not persisted this turn) — appended only when present
    // so a settled-but-unpersisted finding is never lost.
    if !ctx.input.reason.is_empty() {
        full = format!("[STOP] why: {}\n{full}", ctx.input.reason);
    }
    let semver_ctx = ctx.semver_advisory.as_deref().unwrap_or("");
    if !semver_ctx.is_empty() {
        full.push('\n');
        full.push_str(semver_ctx);
    }
    let capture_ctx = ctx.capture_advisory.as_deref().unwrap_or("");
    if !capture_ctx.is_empty() {
        full.push('\n');
        full.push_str(capture_ctx);
    }
    let loophole_ctx = ctx.loophole_advisory.as_deref().unwrap_or("");
    if !loophole_ctx.is_empty() {
        full.push('\n');
        full.push_str(loophole_ctx);
    }
    let shallow_ctx = ctx.shallow_advisory.as_deref().unwrap_or("");
    if !shallow_ctx.is_empty() {
        full.push('\n');
        full.push_str(shallow_ctx);
    }
    super::super::pattern_extract::trigger_on_verify(ctx.session);
    drop(kavach_hook::exit_stop_context(&full));
    ControlFlow::Break(())
}
