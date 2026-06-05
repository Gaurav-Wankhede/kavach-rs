//! PRIORITY 1: dispatch the next kanban task.
use core::ops::ControlFlow;

use super::source_down;
use crate::gates::event_log::log_gate_decision;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{SOURCE_DOWN_KEY, claim_card, get_next_task_info};

/// `Break` with an `[AUTO_CONTINUE]` envelope if a task is pending; `Continue`
/// (fall through to the next tier) when the task tier is empty.
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let Some((priority, title)) = get_next_task_info(&ctx.session.project) else {
        return ControlFlow::Continue(());
    };
    if priority == SOURCE_DOWN_KEY {
        return source_down::block("backlog");
    }
    let _claimed = claim_card(&ctx.session.project, &priority);
    if ctx.session.current_kanban_card != priority {
        ctx.session.current_kanban_card.clone_from(&priority);
        ctx.session.save_or_log();
    }
    if ctx.session.loop_active && !ctx.session.loop_exceeded_max() {
        ctx.session.increment_loop();
    }
    log_gate_decision(
        &ctx.session.session_id,
        "stop:kanban_pending",
        "block",
        &format!("next_task={priority}: {title}"),
        &ctx.session.project,
    );
    let proj = &ctx.session.project;
    drop(kavach_hook::exit_stop_block(&format!(
        "[AUTO_CONTINUE] Kanban has pending work — do not stop.\n\
         NEXT TASK [{priority}]: {title}\n\
         (CLAIMED — this card is now in_progress in the Kavach DB; execute it immediately.)\n\n\
         Step 1 — read the card:\n\
           kavach db get --project {proj} --category roadmap --key {priority} --full\n\
         Step 2 — open phase iteration on the first file you'll edit:\n\
           kavach phase iteration-start <path>\n\
         Step 3 — execute. Close with:\n\
           kavach db status-update --project {proj} --category roadmap --key {priority} --status done\n\
           kavach phase iteration-done"
    )));
    ControlFlow::Break(())
}
