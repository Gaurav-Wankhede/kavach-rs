//! Guard: kanban card pointer hygiene only.
//!
//! POLICY ("kill blocking, keep auto-continue"): an in-progress current card no
//! longer HALTS the stop. Halting here was a nag that blocked the user mid-work;
//! the autonomous loop is preserved by the `dispatch::retry` reblock tier, which
//! re-dispatches the next runnable card (a still-open current card included).
//! This guard now only clears a STALE pointer when its card is already terminal
//! (verified/deferred) in the DB, then always falls through.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;
use crate::gates::stop_dispatch::card_is_still_open;

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !ctx.input.stop_hook_active && !ctx.session.current_kanban_card.is_empty() {
        let card = ctx.session.current_kanban_card.clone();
        // Only a terminal (verified/deferred) card is a stale pointer to clear.
        // An open card is left in place for the reblock dispatch; never halt.
        if !card_is_still_open(&ctx.session.project, &card) {
            ctx.session.current_kanban_card.clear();
            ctx.session.save_or_log();
        }
    }
    ControlFlow::Continue(())
}
