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
    // Progress-gated reblock breaker: trip after N consecutive no-progress stops.
    // See decision.engine.progress-gated-reblock.
    if ctx.session.stop_reblock_count >= kavach_session::SessionState::max_stop_reblocks() {
        return ControlFlow::Continue(());
    }
    match next_dispatch(&ctx.session.project) {
        Some((_, priority, _)) if priority == SOURCE_DOWN_KEY => {
            drop(kavach_hook::exit_stop_block(
                "[AUTO_CONTINUE] kanban source UNREACHABLE — cannot read the \
                 backlog to find the next task; fail-closed so the outage cannot \
                 silently disable the loop.\nRECOVER: `kavach rpc` (background), \
                 then `kavach db kanban --project <slug>` and resume dispatch. The \
                 loop yields only to the user's `Esc`, never to a DB outage.",
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
                "{loop_prefix}{reward_prefix}STOP BLOCKED ({attempt}/{max}): you have runnable work — \
                 resume it NOW, do not stop. NEXT {tier} [{priority}]: {title}. This card is CLAIMED \
                 and in_progress in the Kavach DB. Start it this turn. CONTRACT: claim -> implement \
                 -> 3-witness verify (artifact exists -> diff landed -> build \
                 passes) -> close, all this turn; run the loophole lenses before \
                 you claim done. Do NOT propose a session break.{harness}"
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
/// a `verified` prereq. Branch on the four-state auto-verify outcome:
/// - `Promoted(n > 0)` + a card now dispatchable → resume that card.
/// - `WitnessFailed` → an AI-fixable keystone exists → command `KEYSTONE_REPAIR`.
/// - `Unprovable` → non-Rust project with no `KAVACH_VERIFY_CMD` → cannot prove work →
///   block and surface reason.
/// - `NothingDone` / `Promoted` with nothing dispatchable → the queue is empty or
///   every remainder is dependency-blocked → emit the census-aware DB-rescan
///   verdict (`all_blocked` / `board_drained`), never a self-stop. The loop yields
///   only to the user's `Esc`.
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
             (workspace witnesses passed: cargo check + clippy + nextest + git diff) → loop unblocked. \
             NEXT {tier} [{key}]: {title}. CLAIMED — resume NOW."
        )));
        return ControlFlow::Break(());
    }
    ctx.session.clear_stop_reblock();
    match outcome {
        AutoVerify::VerifyRpcDown => {
            drop(kavach_hook::exit_stop_block(
                "[VERIFY_RPC_DOWN] Work is PROVEN (workspace witnesses passed) but the \
                 DB write to promote done -> verified FAILED — the kavach daemon/RPC is \
                 unreachable. The card is NOT incomplete; the status update did not land. \
                 Do NOT re-implement the card.\nRECOVER: start `kavach rpc` (background), \
                 then `kavach db status-update <key> --status verified --project <slug>` to \
                 land the promotion, then resume dispatch. The loop yields only to `Esc`, \
                 never silently re-dispatches a finished card on an outage.",
            ));
            ControlFlow::Break(())
        }
        AutoVerify::WitnessFailed => keystone_repair(),
        AutoVerify::Unprovable => {
            drop(kavach_hook::exit_stop_block(
                "BLOCKED: `done` cards exist but work CANNOT BE PROVEN. The project is \
                 not a Rust workspace (no Cargo.toml) and KAVACH_VERIFY_CMD is not set. \
                 Either:\n\
                 1. Set env var KAVACH_VERIFY_CMD to a shell command that verifies the work, then resume.\n\
                 2. Manually promote the cards (kavach db roadmap update <key> --status verified) if work is proven by external audit.\n\
                 The loop yields only to the user's `Esc`."
            ));
            ControlFlow::Break(())
        }
        AutoVerify::Promoted(_) | AutoVerify::NothingDone => continue_next_phase(&ctx.session.project),
    }
}
