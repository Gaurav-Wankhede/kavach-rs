//! PRIORITY 1c: promote + dispatch the next roadmap backlog card.
use core::ops::ControlFlow;

use super::source_down;
use crate::gates::event_log::log_gate_decision;
use crate::gates::loop_frame;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{SOURCE_DOWN_KEY, claim_card, get_next_backlog_info};

/// `Break` with a backlog `[AUTO_CONTINUE]` envelope if a backlog card is
/// runnable; `Continue` when the backlog tier is empty (genuine drain).
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let Some((backlog_key, backlog_title)) = get_next_backlog_info(&ctx.session.project) else {
        return ControlFlow::Continue(());
    };
    if backlog_key == SOURCE_DOWN_KEY {
        return source_down::block("roadmap backlog");
    }
    // Honor the atomic claim: a lost CAS means another session already took this
    // card — fall through instead of announcing a false claim (multi-session
    // work-steal guard; see task.rs).
    if !claim_card(&ctx.session.project, &backlog_key) {
        log_gate_decision(
            &ctx.session.session_id,
            "stop:claim_lost",
            "continue",
            &format!("backlog={backlog_key} taken by another session; falling through"),
            &ctx.session.project,
        );
        return ControlFlow::Continue(());
    }
    if ctx.session.current_kanban_card != backlog_key {
        ctx.session.current_kanban_card.clone_from(&backlog_key);
        ctx.session.save_or_log();
    }
    if ctx.session.loop_active && !ctx.session.loop_exceeded_max() {
        ctx.session.increment_loop();
    }
    log_gate_decision(
        &ctx.session.session_id,
        "stop:backlog_promoted",
        "block",
        &format!("promoted={backlog_key}: {backlog_title}"),
        &ctx.session.project,
    );
    let loop_prefix = loop_frame::build_loop_stop(ctx.session, Some(&backlog_title));
    let proj = &ctx.session.project;
    drop(kavach_hook::exit_stop_block(&format!(
        "{loop_prefix}[AUTO_CONTINUE] Do NOT stop. Build this backlog card THIS turn.\n\
         NEXT BACKLOG [{backlog_key}]: {backlog_title}\n\
         This card is CLAIMED and in_progress in the Kavach DB. Start it now.\n\n\
         Do this now, in order:\n\
         1. Read the card:\n\
           kavach db get --project {proj} --category roadmap --key {backlog_key} --full\n\
         2. Open a phase iteration on the first file you'll edit:\n\
           kavach phase iteration-start <path>\n\
         3. Implement it. Then close:\n\
           kavach db status-update --project {proj} --category roadmap --key {backlog_key} --status done\n\
           kavach phase iteration-done\n\
         Your VERY NEXT action must be step 1 — do not end this turn describing the card."
    )));
    ControlFlow::Break(())
}
