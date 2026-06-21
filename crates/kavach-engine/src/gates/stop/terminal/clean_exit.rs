//! Guard: the dispatch tiers found no runnable card. Reset the pending-work
//! re-block breaker and emit the census-aware DB-rescan verdict (never a
//! hardcoded self-stop — the loop yields only to the user's `Esc`), plus any
//! ride-along advisories. Always Breaks (terminal for THIS turn, not the loop).

use core::ops::ControlFlow;

use super::super::shared::StopCtx;
use crate::gates::bandit::{emit, explore_emit};
use kavach_patterns::bandit_log::{BanditContext, GateAction};

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // Clean exit = GREEDY Allow, logged via epsilon-greedy (explore only when
    // KAVACH_RL_EXPLORE armed; never converts to Block). Reward back-filled at
    // verify. Fire-and-forget. See decision.engine.clean-exit-bandit-log.
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
    // Also sample into the SOFT held-out channel (KAVACH_RL_HELDOUT_RATE, default
    // 0=off) for the reward-hacking audit. See decision.engine.clean-exit-held-out.
    let roll = explore_emit::held_out_roll(&ctx.session.session_id, emit::now_ms());
    emit::maybe_emit_held_out(&ctx.session.session_id, ctx_for_emit, action, propensity, roll);
    // Work is genuinely done at the dispatch level — reset the pending-work
    // re-block breaker before composing the terminal verdict.
    ctx.session.clear_stop_reblock();

    // Drained board != finished plan: emit the census-aware verdict ([ALL_BLOCKED]
    // or board-drained [PLAN]), never a silent stop hiding un-built phases.
    // Loop-safe (ALLOWS the stop). See decision.engine.clean-exit-drained-plan.
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
    // Compaction-seam ride-along (shared predicate with SessionStart): an
    // auto-compact may have fired a Stop with an in_progress card done-but-
    // UNRECORDED — surface [RECONCILE] so the next turn resumes at VERIFY, not a
    // re-edit. Fail-soft (None on clean tree / no hint / RPC miss). Advisory only.
    if let Some(reconcile) = super::super::super::session_start::reconcile_context(&ctx.session.project) {
        full.push('\n');
        full.push_str(&reconcile);
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

    // Loophole surface: RESOLVE, never block. Sites already recorded + carried
    // forward; attach awareness as a ride-along and let the stop proceed.
    // SOURCE: decision.loophole.resolve-not-handback.
    if let Some(advisory) = ctx.loophole_advisory.as_deref() {
        full.push('\n');
        full.push_str(advisory);
    }

    // REFUSE-STOP when census proves unblocked roadmap todos the probe hid: command
    // a direct query+claim+start. Breaker-bounded (force-allows after N).
    // See decision.engine.refuse-stop-roadmap-todos.
    if refuse_stop_on_roadmap_todos(ctx) {
        let blocked = super::drained::roadmap_todos_remain_context(&ctx.session.project);
        drop(kavach_hook::exit_stop_block(&format!("{blocked}\n{full}")));
        return ControlFlow::Break(());
    }

    // REFUSE-STOP on an unsourced current-knowledge claim (internet-first teeth):
    // command WebSearch + cite-or-drop. Breaker-bounded (force-allows after N).
    // See decision.engine.refuse-stop-unsourced-research.
    if refuse_stop_on_unsourced_research(ctx) {
        let blocked = format!(
            "[RESEARCH_FIRST] Do NOT stop. This turn asserted a current-knowledge \
             fact (latest/version/API/pricing/supports) from memory with NO source \
             URL. Tabula rasa: the weights are stale, the web is truth (global \
             CLAUDE.md §internet-first). WebSearch a real source THIS turn and cite \
             its URL, persist the finding (`kavach db write --category research`), or \
             DROP the claim. No source -> no claim.\n{full}"
        );
        drop(kavach_hook::exit_stop_block(&blocked));
        return ControlFlow::Break(());
    }

    drop(kavach_hook::exit_stop_context(&full));
    ControlFlow::Break(())
}

/// Decide whether a drained-board clean-stop must be REFUSED because the turn
/// shipped risk-bearing work with an un-closed loophole.
/// Decide whether the drained-board clean-stop must be REFUSED because the census
/// proves dispatchable roadmap todos the probe missed (census/dispatch divergence).
/// Breaker-bounded (`roadmap_todos_remain`) so a board the model genuinely cannot
/// act on force-allows after N refusals. Calling it mutates the breaker count, so
/// invoke exactly once per stop. Order: checked AFTER the loophole refuse-stop, so
/// an open loophole still takes precedence.
fn refuse_stop_on_roadmap_todos(ctx: &mut StopCtx<'_>) -> bool {
    super::drained::roadmap_todos_remain(&ctx.session.project)
        && super::super::shared::should_block_behavioral(ctx.session, "roadmap_todos_remain")
}

/// Decide whether the clean-stop must be REFUSED because this turn made an
/// unsourced current-knowledge claim (`detect_claim_without_research` fired).
/// Breaker-bounded (`research_unsourced`) so a turn that genuinely cannot source a
/// claim force-allows after N refusals. Calling it mutates the breaker count, so
/// invoke exactly once per stop. Checked LAST (after loophole + roadmap-todos), so
/// those higher-priority refuse-stops still take precedence.
fn refuse_stop_on_unsourced_research(ctx: &mut StopCtx<'_>) -> bool {
    ctx.research_unsourced
        && super::super::shared::should_block_behavioral(ctx.session, "research_unsourced")
}

#[cfg(test)]
#[path = "clean_exit_test.rs"]
mod tests;
