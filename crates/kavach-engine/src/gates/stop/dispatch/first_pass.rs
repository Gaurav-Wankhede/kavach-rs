//! Guard: first-attempt loop dispatch (first stop only).
//!
//! Checks the three tiers in priority order — `task` → `hunt` → `backlog` — and
//! HARD-BLOCKs with an `[AUTO_CONTINUE]` dispatch envelope while runnable work
//! exists. Fail-closed on the `SOURCE_DOWN` sentinel. Then handles loop budget +
//! target. Returns `Continue` only when all tiers are empty (genuine drain).
//!
//! Each tier is a `check` returning `ControlFlow`, run in priority order; the
//! first to `Break` wins. `source_down` is the shared fail-closed block.
mod backlog;
mod budget;
mod hunt;
mod source_down;
mod task;

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if ctx.input.stop_hook_active {
        return ControlFlow::Continue(());
    }
    task::check(ctx)?;
    hunt::check(ctx)?;
    backlog::check(ctx)?;
    budget::check(ctx)
}
