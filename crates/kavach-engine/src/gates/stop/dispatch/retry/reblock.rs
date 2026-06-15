//! Three-tier re-block (mirrors first-attempt) under the same breaker budget:
//! dispatch the next card, witness-gate auto-verify of `done` cards, then branch
//! on the auto-verify outcome — `KEYSTONE_REPAIR` (AI fixes a witness-failing
//! card) vs `AUTO_CONTINUE` into the next un-built `[PLAN]` phase when the board
//! is drained. The loop halts only when the plan is fully built and the board is
//! empty.
use core::ops::ControlFlow;

mod outcome;

use outcome::{continue_next_phase, keystone_repair};
use super::probe::next_dispatch;
use crate::gates::loop_frame;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{
    AutoVerify, SOURCE_DOWN_KEY, auto_verify_done_cards, claim_card,
};

/// Run the three-tier re-block while under the breaker ceiling. `Continue` only
/// when the ceiling is spent or no terminal branch fired (falls through).
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if ctx.session.stop_reblock_count >= kavach_session::SessionState::max_stop_reblocks() {
        return ControlFlow::Continue(());
    }
    match next_dispatch(&ctx.session.project) {
        Some((_, priority, _)) if priority == SOURCE_DOWN_KEY => {
            drop(kavach_hook::exit_stop_block(
                "[AUTO_CONTINUE] kanban source UNREACHABLE — cannot verify the \
                 backlog is empty; clean stop REFUSED (fail-closed; this outage \
                 silently disables the loop).\nRECOVER: `kavach rpc` \
                 (background), then `kavach db kanban --project <slug>`. Fix the \
                 daemon before stopping.",
            ));
            ControlFlow::Break(())
        }
        Some((tier, priority, title)) => {
            let _claimed = claim_card(&ctx.session.project, &priority);
            ctx.session.increment_stop_reblock();
            let attempt = ctx.session.stop_reblock_count;
            let max = kavach_session::SessionState::max_stop_reblocks();
            // L3: if the claimed card carries a dynamic-workflow harness, append
            // the `[AUTO_CONTINUE] run Workflow <path>` directive so the AI runs
            // the compiled workflow rather than hand-executing the card.
            let harness =
                super::super::harness_suffix(&ctx.session.project, &priority).unwrap_or_default();
            let loop_prefix = loop_frame::build_loop_stop(ctx.session, Some(&title));
            let reward_prefix = loop_frame::build_reward_stop_last(ctx.session);
            drop(kavach_hook::exit_stop_block(&format!(
                "{loop_prefix}{reward_prefix}STOP BLOCKED ({attempt}/{max}): kanban has runnable work. \
                 NEXT {tier} [{priority}]: {title}. CLAIMED — now in_progress in \
                 the Kavach DB; resume this work NOW. Stop is forced after {max} \
                 re-blocks; the card stays open and you resume work THIS turn per \
                 §FOCUS NEVER-PROPOSE-SESSION-BREAK. CONTRACT: claim -> implement \
                 -> 3-witness verify (artifact exists -> diff landed -> build \
                 passes) -> close, all this turn; loophole-check before any done \
                 claim.{harness}"
            )));
            ControlFlow::Break(())
        }
        None if !ctx.session.has_recent_failure()
            && ctx.session.active_subagents == 0
            && !(ctx.session.has_task() && ctx.session.task_status == "in_progress") =>
        {
            all_blocked_or_autoverify(ctx)
        }
        None => ControlFlow::Continue(()),
    }
}

/// A `done` card (awaiting verify) is non-runnable and blocks dependents needing
/// a `verified` prereq. Branch on the three-state auto-verify outcome:
/// - `Promoted` + a card now dispatchable → resume that card.
/// - `WitnessFailed` → an AI-fixable keystone exists → command `KEYSTONE_REPAIR`.
/// - `NothingDone` / `Promoted` with nothing dispatchable → the queue is empty or
///   every remainder is blocked by unmet dependencies → a genuine clean stop. This
///   last branch is what stops the loop from running forever on a blocked backlog.
fn all_blocked_or_autoverify(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let outcome = auto_verify_done_cards(&ctx.session.project);
    if matches!(outcome, AutoVerify::Promoted(n) if n > 0)
        && let Some((tier, key, title)) = next_dispatch(&ctx.session.project)
    {
        let _claimed = claim_card(&ctx.session.project, &key);
        ctx.session.increment_stop_reblock();
        let attempt = ctx.session.stop_reblock_count;
        let max = kavach_session::SessionState::max_stop_reblocks();
        drop(kavach_hook::exit_stop_block(&format!(
            "[AUTO_CONTINUE] ({attempt}/{max}) auto-verified done card(s) \
             (workspace witnesses passed: cargo check + nextest) → loop unblocked. \
             NEXT {tier} [{key}]: {title}. CLAIMED — resume NOW."
        )));
        return ControlFlow::Break(());
    }
    ctx.session.clear_stop_reblock();
    if outcome == AutoVerify::WitnessFailed {
        return keystone_repair();
    }
    continue_next_phase(&ctx.session.project)
}
