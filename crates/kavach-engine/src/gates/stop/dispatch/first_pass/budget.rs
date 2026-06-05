//! All tiers empty — the loop may genuinely end. Handle the token budget +
//! loop-target stop conditions; always `Continue` (lets the stop proceed).
use core::ops::ControlFlow;

use crate::gates::event_log::log_gate_decision;
use crate::gates::stop::shared::StopCtx;

/// Apply loop budget/target stop conditions. Always returns `Continue` — by the
/// time this runs, every dispatch tier is empty (genuine drain).
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if ctx.session.loop_active {
        if ctx.session.budget_exceeded() {
            let used = ctx.session.token_budget_used;
            let total = ctx.session.token_budget_total;
            ctx.session.stop_loop();
            log_gate_decision(
                &ctx.session.session_id,
                "stop:budget_exceeded",
                "allow",
                &format!("token_budget_used={used} >= total={total}"),
                &ctx.session.project,
            );
            return ControlFlow::Continue(());
        }
        if ctx.session.loop_target_reached() {
            ctx.session.stop_loop();
        }
    }
    ControlFlow::Continue(())
}
