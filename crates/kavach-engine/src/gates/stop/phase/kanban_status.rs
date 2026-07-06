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
        // Close-before-advance is a DB-INTEGRITY invariant, non-surrenderable.
        // See decision.engine.kanban-status-close-before-advance.
        if !updated {
            let n = ctx.session.files_modified_this_turn.len();
            let project = ctx.session.project.clone();
            drop(kavach_hook::exit_stop_block(&format!(
                "[KANBAN_STATUS_PENDING] (required: close-before-advance invariant)\n\
                 You modified {n} file(s) this turn but card '{card}' is still open and \
                 was not updated. The loop advances once the DB reflects reality.\n\
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
