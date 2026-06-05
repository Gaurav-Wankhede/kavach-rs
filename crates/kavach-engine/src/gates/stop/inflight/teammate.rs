//! Guard: yield the Stop while peer teammates are still working (v2.1.154 Agent
//! Teams). Blocking here recreates issue #55754 on the Team channel; the parent
//! Stop's duty is to YIELD so TeammateIdle/TaskCompleted gates drive the loop.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "uniform fn(&mut StopCtx) pipeline signature is required by the guard dispatch table even when this guard only reads ctx"
)]
pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if ctx.session.active_teammates > 0 {
        eprintln!(
            "[KAVACH_TEAM_YIELD] {} active teammate(s) in {}; stop-gate yields",
            ctx.session.active_teammates,
            if ctx.session.team_name.is_empty() {
                "team"
            } else {
                &ctx.session.team_name
            }
        );
        drop(kavach_hook::exit_silent());
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}
