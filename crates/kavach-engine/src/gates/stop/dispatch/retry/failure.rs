//! Failure tier of the retry chain.
//!
//! POLICY ("kill blocking, keep auto-continue"): a recent tool failure must NOT
//! HALT the stop. Halting here also pre-empted the `reblock` tier — the `?`
//! short-circuit in `retry::check` would never reach the kanban auto-continue
//! dispatch — so a single tool error wedged the autonomous loop. This tier now
//! always falls through to `reblock`, which re-dispatches the next runnable
//! card. The agent still sees the failure in its own transcript; the gate no
//! longer blocks the stop on it.
use core::ops::ControlFlow;

use crate::gates::stop::shared::StopCtx;

/// No-op tier under the kill-blocking policy: always `Continue` so control flows
/// to the kanban re-dispatch (`reblock`) instead of halting on a tool failure.
pub(super) const fn check(_ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    ControlFlow::Continue(())
}
