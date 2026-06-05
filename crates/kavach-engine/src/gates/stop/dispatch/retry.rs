//! Guard: the stop-retry path (only when `stop_hook_active`). Owns the bounded
//! failure-block breaker, the `USER_FOCUS` one-shot surfacing, the three-tier
//! re-block dispatch (with witness-gated auto-verify of `done` cards), and the
//! forced clean terminal — guarded by the authoritative empty-queue probe so a
//! completion never abandons runnable work, and the live-lock saturation guard
//! so a single stuck card cannot wedge the session forever.
//!
//! The branches form one cohesive state machine over the bounded
//! `stop_reblock_count`; each is a `check` returning `ControlFlow`, run in this
//! exact order so the no-infinite-loop + no-abandoned-work invariants hold.
mod failure;
mod focus;
mod probe;
mod reblock;
mod terminal;

use core::ops::ControlFlow;

use crate::gates::stop::shared::StopCtx;

/// Stop-retry terminal. Runs the ordered branch chain; the first branch to
/// `Break` decides the stop. Only active under `stop_hook_active`.
pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !ctx.input.stop_hook_active {
        return ControlFlow::Continue(());
    }
    failure::check(ctx)?;
    focus::check(ctx)?;
    reblock::check(ctx)?;
    terminal::check(ctx)
}
