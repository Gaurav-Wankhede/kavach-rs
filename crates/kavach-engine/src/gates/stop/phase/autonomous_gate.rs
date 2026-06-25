//! Guard (P0, user-authority): PLAN-MODE OVERRIDE — allow the stop when
//! `permission_mode == "plan"`. See decision.engine.stop-plan-mode-override.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

/// True iff the harness reports Plan Mode for this turn (pure → unit-testable).
#[must_use]
pub(crate) fn is_plan_mode(permission_mode: &str) -> bool {
    permission_mode == "plan"
}

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !is_plan_mode(&ctx.input.permission_mode) {
        return ControlFlow::Continue(());
    }
    crate::gates::event_log::log_gate_decision(
        &ctx.session.session_id,
        "stop:plan_mode_override",
        "allow_stop",
        &format!("permission_mode=plan on turn {}; not dispatching a card", ctx.session.turn_count),
        &ctx.session.project,
    );
    eprintln!(
        "[PLAN_MODE] Plan Mode is active — the user drives this turn by asking \
         questions. NOT dispatching a kanban card. The autonomous loop resumes in \
         Auto (default) or bypassPermissions mode."
    );
    drop(kavach_hook::exit_silent());
    ControlFlow::Break(())
}

#[cfg(test)]
#[path = "plan_mode/tests.rs"]
mod tests;
