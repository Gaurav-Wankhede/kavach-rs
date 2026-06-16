//! PRIORITY 1b: dispatch the next bug-hunt card (proven, unfixed defect).
use core::ops::ControlFlow;

use super::source_down;
use crate::gates::event_log::log_gate_decision;
use crate::gates::loop_frame;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{SOURCE_DOWN_KEY, claim_card, get_next_hunt_info};

/// `Break` with a hunt `[AUTO_CONTINUE]` envelope if a hunt card is open;
/// `Continue` when the hunt tier is empty.
pub(super) fn check(ctx: &StopCtx<'_>) -> ControlFlow<()> {
    let Some((hunt_key, hunt_title)) = get_next_hunt_info(&ctx.session.project) else {
        return ControlFlow::Continue(());
    };
    log_gate_decision(
        &ctx.session.session_id,
        "stop:hunt_pending",
        "block",
        &format!("next_hunt={hunt_key}: {hunt_title}"),
        &ctx.session.project,
    );
    if hunt_key == SOURCE_DOWN_KEY {
        return source_down::block("backlog");
    }
    let proj = ctx.session.project.clone();
    // Honor the atomic claim: lost CAS -> another session took this hunt card;
    // fall through rather than announce a false claim (work-steal guard).
    if !claim_card(&proj, &hunt_key) {
        log_gate_decision(
            &ctx.session.session_id,
            "stop:claim_lost",
            "continue",
            &format!("hunt={hunt_key} taken by another session; falling through"),
            &proj,
        );
        return ControlFlow::Continue(());
    }
    let loop_prefix = loop_frame::build_loop_stop(ctx.session, Some(&hunt_title));
    drop(kavach_hook::exit_stop_block(&format!(
        "{loop_prefix}[AUTO_CONTINUE] Do NOT stop. Fix this proven defect THIS turn.\n\
         NEXT HUNT [{hunt_key}]: {hunt_title}\n\
         This card is CLAIMED and in_progress in the Kavach DB. Work it now.\n\n\
         Do this now, in order:\n\
         1. Read the proven defect + repro:\n\
           kavach db get --project {proj} --category roadmap --key {hunt_key} --full\n\
         2. Confirm it is still live on HEAD (skip if already fixed), then RCA + root-fix.\n\
         3. Verify 3-witness: failing-test -> passing + cargo check exit 0.\n\
         4. Gate: severity high/critical -> [HUMAN_GATE] halt; else close:\n\
           kavach db status-update --project {proj} --category roadmap --key {hunt_key} --status verified\n\
         Your VERY NEXT action must be step 1 — do not end this turn describing the defect."
    )));
    ControlFlow::Break(())
}
