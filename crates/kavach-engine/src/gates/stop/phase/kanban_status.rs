//! Guard (P1, circuit-breaker): a turn that modified in-project files must end
//! with a `kavach db status-update` on the active card. Card mis-attribution
//! fix: relaxes when no edit could plausibly belong to the card; auto-clears a
//! stale closed-card pointer.

use core::ops::ControlFlow;

use super::super::shared::{StopCtx, card_owns_any_turn_file};
use crate::gates::stop_dispatch::card_is_still_open;

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !(!ctx.input.stop_hook_active
        && !ctx.session.files_modified_this_turn.is_empty()
        && !ctx.session.current_kanban_card.is_empty())
    {
        return ControlFlow::Continue(());
    }
    let card = ctx.session.current_kanban_card.clone();
    let any_in_project = card_owns_any_turn_file(ctx.session);
    if any_in_project && card_is_still_open(&ctx.session.project, &card) {
        let updated = ctx
            .session
            .recent_commands
            .iter()
            .any(|line| line.contains("kavach db status-update") && line.contains(&card));
        // NON-SURRENDERABLE: close-before-advance is a DB-INTEGRITY invariant
        // (global CLAUDE.md §autonomous_loop.3_close_before_advance), NOT a
        // cosmetic behavioral nag. It does NOT route through the 3-strike
        // behavioral breaker — a card left in_progress while work proceeds is a
        // lie to the work-ledger, and an invariant that can be waited out in N
        // turns is not enforced (CWE-840). PARKING ABOLISHED (operator directive
        // 2026-06-16, reaffirmed 2026-06-17): there is no honest-park escape — the
        // block lifts on EXACTLY ONE of:
        //   (a) a real `kavach db status-update` for THIS card this turn, or
        //   (b) DELETE the card (`kavach db delete --category roadmap --key ...`)
        //       when it is genuinely un-buildable — runnable or DELETED, never
        //       marker-parked (§delete_not_park). No timeout/marker escape.
        if !updated {
            let n = ctx.session.files_modified_this_turn.len();
            let project = ctx.session.project.clone();
            drop(kavach_hook::exit_stop_block(&format!(
                "[KANBAN_STATUS_PENDING] (non-surrenderable: close-before-advance invariant)\n\
                 You modified {n} file(s) this turn but card '{card}' is still open and \
                 was NOT updated. The loop will NOT advance until the DB reflects reality.\n\
                 CLOSE it:  kavach db status-update --project {project} --category roadmap \\\n\
                            --key {card} --status <done|verified>  (or in_progress if mid-work)\n\
                 OR, if it is genuinely un-buildable (missing credential / external \
                 prerequisite — FILE that gap as its own card first),\n\
                 DELETE it (parking is abolished — runnable or deleted):\n\
                   kavach db delete --project {project} --category roadmap --key {card}\n\
                 Then re-stop. There is no timeout escape and no marker escape — \
                 close or delete, then continue."
            )));
            return ControlFlow::Break(());
        }
    } else {
        ctx.session.current_kanban_card.clear();
        ctx.session.save_or_log();
        crate::gates::event_log::log_gate_decision(
            &ctx.session.session_id,
            "stop:kanban_auto_close",
            "recover",
            &format!("stale=closed card={card}"),
            &ctx.session.project,
        );
        eprintln!("[KANBAN_AUTO_CLOSE] stale=closed card={card}");
    }
    ControlFlow::Continue(())
}
