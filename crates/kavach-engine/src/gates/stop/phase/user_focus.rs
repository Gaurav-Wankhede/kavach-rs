//! Guard (P0, user-authority): the USER-FOCUS OVERRIDE.
//!
//! ROOT CAUSE THIS FIXES (operator directive 2026-06-18): the stop gate's job is to
//! drain the kanban autonomously — good for unattended loop engineering. But when
//! the USER gives a specific instruction THIS turn, the gate would still fire
//! `STOP BLOCKED -> NEXT TASK [Y]` and drag the session onto a DIFFERENT queued
//! card than what the user just asked for. The loop directive ("terminate ONLY on
//! 3-witness") was written for unattended draining and applied identically to an
//! attended, user-steered turn.
//!
//! THE OVERRIDE: when the user issued a directive on the CURRENT turn
//! (`session.user_is_steering_this_turn()`) AND this session is NOT mid-work on a
//! claimed card, ALLOW the stop (`exit_silent`) instead of dispatching a foreign
//! card. The user's live instruction outranks the queue for THIS turn. The
//! autonomous loop is untouched on turns the user did NOT just steer — the gate
//! still dispatches the next card as before (that is the loop-engineering value).
//!
//! WHY "not mid-work on a claimed card" matters: if the user's instruction already
//! caused a card to be claimed in-progress this turn, the close-before-advance
//! invariant (`kanban_status`) must still run — the override only suppresses
//! dispatching a NEW, unrelated card over the user's focus, never the integrity of
//! a card the user's own work touched.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // Only fires on a fresh stop (not a re-entrant loop tick) where the user
    // steered THIS turn and no card is mid-work by this session.
    if ctx.input.stop_hook_active
        || !ctx.session.user_is_steering_this_turn()
        || !ctx.session.current_kanban_card.is_empty()
    {
        return ControlFlow::Continue(());
    }
    crate::gates::event_log::log_gate_decision(
        &ctx.session.session_id,
        "stop:user_focus_override",
        "allow_stop",
        &format!(
            "user steered turn {}; not dispatching a queued card",
            ctx.session.turn_count
        ),
        &ctx.session.project,
    );
    eprintln!(
        "[USER_FOCUS] the user steered this turn — honoring their instruction, NOT \
         dispatching a different kanban card. The autonomous loop resumes on the next \
         stop where the user did not just speak."
    );
    drop(kavach_hook::exit_silent());
    ControlFlow::Break(())
}
// NOTE: the guard itself calls `exit_silent()` (process exit) so it cannot be
// unit-tested in-process; its load-bearing PREDICATE is
// `SessionState::user_is_steering_this_turn()`, unit-tested in kavach-session
// (markers_tests.rs) where it is a pure function.
