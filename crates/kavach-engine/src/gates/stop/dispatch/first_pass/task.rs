//! PRIORITY 1: dispatch the next kanban task.
use core::ops::ControlFlow;

use super::source_down;
use crate::gates::event_log::log_gate_decision;
use crate::gates::loop_frame;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{SOURCE_DOWN_KEY, card_entry_status, claim_card, get_next_task_info};

/// `Break` with an `[AUTO_CONTINUE]` envelope if a task is pending; `Continue`
/// (fall through to the next tier) when the task tier is empty.
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let Some((priority, title)) = get_next_task_info(&ctx.session.project) else {
        return ControlFlow::Continue(());
    };
    if priority == SOURCE_DOWN_KEY {
        return source_down::block("backlog");
    }
    let claimed = claim_card(&ctx.session.project, &priority);
    if !claimed {
        // CAS miss on an already-`in_progress` card is expected (idempotent re-stop)
        // — reblock so Cursor's initial `loop_count:0` stop still dispatches runnable
        // work instead of a silent `{}` clean exit. Only fall through on a lost
        // `todo` race (another session claimed it first).
        let resume = card_entry_status(&ctx.session.project, &priority)
            .is_some_and(|s| s == "in_progress");
        if !resume {
            log_gate_decision(
                &ctx.session.session_id,
                "stop:claim_lost",
                "continue",
                &format!("card={priority} taken by another session; falling through"),
                &ctx.session.project,
            );
            return ControlFlow::Continue(());
        }
        log_gate_decision(
            &ctx.session.session_id,
            "stop:resume_in_progress",
            "block",
            &format!("resume={priority}: {title}"),
            &ctx.session.project,
        );
    }
    if ctx.session.current_kanban_card != priority {
        ctx.session.current_kanban_card.clone_from(&priority);
        ctx.session.save_or_log();
    }
    if ctx.session.loop_active && !ctx.session.loop_exceeded_max() {
        ctx.session.increment_loop();
    }
    if claimed {
        log_gate_decision(
            &ctx.session.session_id,
            "stop:kanban_pending",
            "block",
            &format!("next_task={priority}: {title}"),
            &ctx.session.project,
        );
    }
    let proj = &ctx.session.project;
    let loop_prefix = loop_frame::build_loop_stop(ctx.session, Some(&title));
    let reward_prefix = loop_frame::build_reward_stop_last(ctx.session);
    let claim_line = if claimed {
        "(CLAIMED — this card is now in_progress in the Kavach DB; execute it immediately.)"
    } else {
        "(IN_PROGRESS — resume this card immediately; do not stop mid-work.)"
    };
    drop(kavach_hook::exit_stop_block(&format!(
        "{loop_prefix}{reward_prefix}[AUTO_CONTINUE] Kanban has pending work — do not stop.\n\
         NEXT TASK [{priority}]: {title}\n\
         {claim_line}\n\n\
         Step 1 — read the card:\n\
           kavach db get --project {proj} --category roadmap --key {priority} --full\n\
         Step 2 — open phase iteration on the first file you'll edit:\n\
           kavach phase iteration-start <path>\n\
         Step 3 — execute. Close with:\n\
           kavach db status-update --project {proj} --category roadmap --key {priority} --status done\n\
           kavach phase iteration-done\n\
         CONTRACT: claim -> implement -> 3-witness verify (artifact exists -> diff landed -> build passes) \
         -> close, ALL this turn. Loophole-check before any done claim; do not stop mid-card."
    )));
    ControlFlow::Break(())
}
