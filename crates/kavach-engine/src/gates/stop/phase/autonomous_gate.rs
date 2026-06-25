//! Guard (P0, user-authority): AUTONOMOUS-MODE GATE — the Stop dispatch fires
//! ONLY in Auto (`permission_mode=="auto"`) or `bypassPermissions`; every other
//! mode (plan/default/acceptEdits/dontAsk) allows a clean stop. See
//! decision.engine.stop-autonomous-mode-only.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

/// True iff the harness reports an autonomous mode where the Stop gate's
/// auto-continue dispatch should run: dedicated Auto or `bypassPermissions`
/// (pure → unit-testable). SOURCE: code.claude.com/docs/en/permissions.
#[must_use]
pub(crate) fn stop_gate_fires(permission_mode: &str) -> bool {
    matches!(permission_mode, "auto" | "bypassPermissions")
}

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // A live user message THIS turn is the highest authority — honor it even in
    // auto/bypass (the user just typed; never re-dispatch a card over their words).
    let user_spoke = ctx.session.user_is_steering_this_turn();
    if stop_gate_fires(&ctx.input.permission_mode) && !user_spoke {
        return ControlFlow::Continue(());
    }
    crate::gates::event_log::log_gate_decision(
        &ctx.session.session_id,
        "stop:autonomous_gate_override",
        "allow_stop",
        &format!(
            "permission_mode={} (not auto/bypass) on turn {}; allowing stop",
            ctx.input.permission_mode, ctx.session.turn_count
        ),
        &ctx.session.project,
    );
    eprintln!(
        "[STOP_GATE] Auto-continue dispatch fires ONLY in Auto or bypassPermissions \
         mode. This turn is attended — the user drives it. NOT dispatching a kanban \
         card; allowing the stop."
    );
    drop(kavach_hook::exit_silent());
    ControlFlow::Break(())
}

#[cfg(test)]
#[path = "autonomous_gate/tests.rs"]
mod tests;
