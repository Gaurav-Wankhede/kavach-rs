//! Forced terminal: collect in-flight bypass state, run the authoritative
//! empty-queue probe (refuse the stop if runnable work remains unless a single
//! card is live-lock saturated), then emit the clean STOP.
use core::ops::ControlFlow;

use super::probe::next_dispatch;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{SOURCE_DOWN_KEY, claim_card, is_backlog_saturated};

/// Collect what is being bypassed by a forced stop, for the terminal advisory.
fn bypass_info(ctx: &StopCtx<'_>) -> String {
    let mut bypassed = Vec::new();
    if ctx.session.has_recent_failure() {
        bypassed.push(format!(
            "unresolved failure: {} (turn {}, blocked {} times)",
            ctx.session.last_failure_tool,
            ctx.session.last_failure_turn,
            ctx.session.failure_block_count,
        ));
    }
    if ctx.session.active_subagents > 0 {
        bypassed.push(format!(
            "{} active subagent(s)",
            ctx.session.active_subagents
        ));
    }
    if ctx.session.has_task() && ctx.session.task_status == "in_progress" {
        bypassed.push(format!("task in progress: {}", ctx.session.current_task));
    }
    if bypassed.is_empty() {
        "none".into()
    } else {
        bypassed.join(", ")
    }
}

/// The forced-terminal tail. Always returns `Break` — it is the last branch.
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let info = bypass_info(ctx);
    if info != "none" {
        ctx.session
            .add_case_fact(&format!("forced stop with bypassed: {info}"));
    }
    ctx.session.clear_failure();

    // AUTHORITATIVE EMPTY-QUEUE PROBE (Ralph-loop law): re-run the dependency-aware
    // chain to find the next task. A non-empty queue means work remains — dispatch
    // it — UNLESS a single card is saturated past the live-lock ceiling with zero
    // progress (the one anti-spin guard; the loop otherwise never self-terminates).
    let saturated = is_backlog_saturated(
        ctx.session.stop_reblock_count,
        ctx.session.has_progress_since_last_stop(),
    );
    if !saturated
        && let Some((tier, key, title)) = next_dispatch(&ctx.session.project)
        && key != SOURCE_DOWN_KEY
    {
        let _claimed = claim_card(&ctx.session.project, &key);
        ctx.session.increment_stop_reblock();
        drop(kavach_hook::exit_stop_block(&format!(
            "[AUTO_CONTINUE] backlog NOT empty — a completion left runnable work \
             queued. NEXT {tier} [{key}]: {title}. CLAIMED — resume NOW. (The \
             re-block breaker bounds SPINNING, not a non-empty queue; the loop \
             continues while work remains.)"
        )));
        return ControlFlow::Break(());
    }

    // Terminal for THIS turn: backlog provably empty, OR one card saturated past
    // the live-lock ceiling. Either way the loop does NOT self-terminate — it
    // re-scans the DB across statuses for the NEXT task (a different one when the
    // current card is live-locked) and yields only to the user's `Esc`.
    ctx.session.clear_stop_reblock();
    let context = kavach_hook::context_block(
        "AUTO_CONTINUE",
        &[
            (
                "why",
                "the dispatch tiers found nothing runnable, or one card is \
                 live-locked past the anti-spin ceiling. Neither ends the loop: \
                 the DB still holds roadmap units and decisions to execute.",
            ),
            (
                "action",
                "Do NOT stop. Re-scan the DB for the next task across statuses — \
                 `kavach db query --category roadmap` and `--category decision` for \
                 any todo/unexecuted row (skip the live-locked card; pick a \
                 DIFFERENT one). RESEARCH MODE FIRST: WebSearch the current \
                 authoritative source to scope it — TABULA RASA = TRUTH, never trust \
                 training weights — then claim and START it THIS turn. When the DB \
                 holds nothing actionable, keep the loop open and yield to the user's \
                 `Esc`.",
            ),
            ("bypassed", &info),
        ],
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}
