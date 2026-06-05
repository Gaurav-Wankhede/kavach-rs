//! Guard: yield the Stop when background tasks or session crons are in flight.
//! Blocking here recreates GitHub issue #55754 (~50min loop) while the bg
//! subagent's async completion never fires. SOURCE:
//! github.com/anthropics/claude-code/issues/55754 + changelog v2.1.152.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

#[expect(
    clippy::needless_pass_by_ref_mut,
    reason = "uniform fn(&mut StopCtx) pipeline signature is required by the guard dispatch table even when this guard only reads ctx"
)]
pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !ctx.input.background_tasks.is_empty() || !ctx.input.session_crons.is_empty() {
        eprintln!(
            "[KAVACH_BG_YIELD] {} background task(s) + {} cron(s) in flight; stop-gate yields",
            ctx.input.background_tasks.len(),
            ctx.input.session_crons.len()
        );
        drop(kavach_hook::exit_silent());
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}
