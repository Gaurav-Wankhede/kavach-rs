//! Guard: the dispatch tiers found no runnable card. Reset the pending-work
//! re-block breaker and emit the census-aware DB-rescan verdict (never a
//! hardcoded self-stop — the loop yields only to the user's `Esc`), plus any
//! ride-along advisories. Always Breaks (terminal for THIS turn, not the loop).

use core::ops::ControlFlow;

use super::super::shared::StopCtx;
use crate::gates::bandit::{emit, explore_emit};
use kavach_patterns::bandit_log::{BanditContext, GateAction};

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // Layer-A bandit log: a clean exit is the Stop gate's GREEDY `Allow` action.
    // P7: pass it through epsilon-greedy so, when `KAVACH_RL_EXPLORE` is armed, the
    // emit MAY log a non-argmax advisory action (`Ask`) with its TRUE propensity
    // < 1.0 — giving the off-policy estimators non-degenerate overlap. Disarmed
    // (default) this is `(Allow, 1.0)`, the exact prior behavior. C2: the advisory
    // set bars `Block`, so exploration NEVER converts this allow into a block.
    // Reward is None here — back-filled when the 3-witness verify resolves. Pure
    // logging, fire-and-forget; never affects whether the stop proceeds.
    let (action, propensity) = explore_emit::explore_action(
        GateAction::Allow,
        &ctx.session.session_id,
        emit::now_ms(),
    );
    let ctx_for_emit = BanditContext::new(
        "stop",
        "Stop",
        "",
        0,
        "",
        u32::try_from(ctx.session.turn_count).unwrap_or(0),
    );
    emit::emit_decision(
        &ctx.session.session_id,
        ctx_for_emit.clone(),
        action,
        propensity,
        None,
    );
    // P8: sample this same decision into the SOFT held-out channel at rate
    // `KAVACH_RL_HELDOUT_RATE` (default 0 ⇒ off). The held-out row carries the same
    // (action, propensity) but is tagged `held_out: true`; its reward is back-filled
    // by an INDEPENDENT real re-verification, giving the reward-hacking audit
    // (`db.ope_audit`) a soft channel to compare against the hard 3-witness. Without
    // it the soft channel is always empty ⇒ the audit stays `Inconclusive` ⇒ a
    // candidate policy can never be cleared. Fire-and-forget; never affects the stop.
    let roll = explore_emit::held_out_roll(&ctx.session.session_id, emit::now_ms());
    emit::maybe_emit_held_out(&ctx.session.session_id, ctx_for_emit, action, propensity, roll);
    // Work is genuinely done at the dispatch level — reset the pending-work
    // re-block breaker before composing the terminal verdict.
    ctx.session.clear_stop_reblock();

    // BUG FIX (drained-board → plan-check): the dispatch tiers found nothing
    // runnable, but a drained board is NOT a finished plan. Emit the SAME
    // census-aware verdict the retry terminal uses (`[ALL_BLOCKED]` when every
    // remainder is dependency-blocked, else the board-drained `[PLAN]` nudge) —
    // never a silent stop that hides un-built plan phases. This terminal used to
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
    // DB-C dynamic injection: the operator-editable `gate.injection.clean_exit` DB row,
    // if present, rides along here — proving the binary carries NO advisory prose
    // for this gate; the text is data-driven + hot-editable (no rebuild). Absent →
    // nothing appended (fail-open). Any gate adopts this with one `gate_injection` call.
    if let Some(inj) =
        crate::gates::stop_dispatch::query::gate_injection(&ctx.session.project, "clean_exit")
    {
        full.push('\n');
        full.push_str(&inj);
    }
    let shallow_ctx = ctx.shallow_advisory.as_deref().unwrap_or("");
    if !shallow_ctx.is_empty() {
        full.push('\n');
        full.push_str(shallow_ctx);
    }
    // Continuation-menu ride-along: the final message asked "continue or pause?"
    // while THIS verdict already commands continuation. Append the imperative
    // nudge so the stop context itself contradicts the permission-seeking question
    // (advisory — the stop still proceeds; the next turn must continue, not re-ask).
    let continuation_ctx = ctx.continuation_advisory.as_deref().unwrap_or("");
    if !continuation_ctx.is_empty() {
        full.push('\n');
        full.push_str(continuation_ctx);
    }
    super::super::pattern_extract::trigger_on_verify(ctx.session);

    // REFUSE-STOP on an un-fixed loophole (parity with [CYCLE_DEADLOCK]): the
    // board is drained, but this turn shipped risk-bearing work WITHOUT a
    // `Loopholes closed:` line — a loophole may be LIVE. A clean stop here would
    // terminate with the defect unfixed, so DO NOT allow the stop: emit
    // exit_stop_block so the loop is forced to close (or file) the loophole this
    // turn. Bounded by the behavioral breaker (category "loophole_open"): after N
    // refusals it force-allows (loop-safety) while recording the surrender, so a
    // model that genuinely cannot answer can never be trapped in an infinite spin.
    if refuse_stop_on_open_loophole(ctx) {
        let blocked = format!(
            "[LOOPHOLE_OPEN] Do NOT stop. This turn shipped risk-bearing work \
             without a `Loopholes closed:` line — a loophole may be LIVE and unfixed \
             right now. FIX it at its root THIS turn (run the 6 attack lenses; close \
             each at file:line or file a card), then emit `Loopholes closed:`. \
             Fixing beats documenting.\n{full}"
        );
        drop(kavach_hook::exit_stop_block(&blocked));
        return ControlFlow::Break(());
    }

    drop(kavach_hook::exit_stop_context(&full));
    ControlFlow::Break(())
}

/// Decide whether a drained-board clean-stop must be REFUSED because the turn
/// shipped risk-bearing work with an un-closed loophole.
///
/// True iff an un-answered loophole advisory is present AND the behavioral
/// breaker for `loophole_open` has not yet tripped. The breaker bound is what
/// makes this loop-safe: after N consecutive refusals it returns `false`
/// (force-allow) while recording the surrender, so a turn that genuinely cannot
/// answer is never trapped in an infinite refuse-stop. Calling it mutates the
/// breaker count, so invoke exactly once per stop.
fn refuse_stop_on_open_loophole(ctx: &mut StopCtx<'_>) -> bool {
    ctx.loophole_advisory.is_some()
        && super::super::shared::should_block_behavioral(ctx.session, "loophole_open")
}

#[cfg(test)]
#[path = "clean_exit_test.rs"]
mod tests;
