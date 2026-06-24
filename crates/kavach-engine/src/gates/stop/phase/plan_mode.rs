//! Guard (P0, user-authority): the PLAN-MODE OVERRIDE.
//!
//! ROOT CAUSE THIS FIXES (operator directive 2026-06-25): the stop gate's
//! auto-continue dispatch was authored for UNATTENDED draining (Auto mode /
//! bypassPermissions). In Plan Mode the USER drives the turn by asking questions
//! and reviewing the plan — the gate must NOT refuse the stop and dispatch a
//! queued card. Claude Code's Stop hook carries `permission_mode`; the value
//! `"plan"` IS Plan Mode (others: default, acceptEdits, dontAsk,
//! bypassPermissions). SOURCE: code.claude.com/docs/en/agent-sdk/permissions.
//!
//! THE OVERRIDE: when `permission_mode == "plan"`, ALLOW the stop
//! (`exit_silent`) BEFORE any dispatch/auto-continue guard runs. The Stop gate
//! resumes its autonomous-loop role on `default` (Auto) and `bypassPermissions`
//! turns, where the loop directive belongs. Wired before `disobedience` so a
//! Plan-Mode turn is never dragged onto a card.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

/// True iff the harness reports Plan Mode for this turn. Pure on the
/// `permission_mode` string so it is unit-testable (the guard itself
/// process-exits and cannot be tested in-process — see `user_focus`).
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
