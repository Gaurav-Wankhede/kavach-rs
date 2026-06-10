//! Terminal-group guard: the clean-exit emitter — the ONLY terminal stop. The
//! review-isolation and shallow-verdict HALT guards were removed under the
//! "kill blocking, keep auto-continue" policy; a Stop now either re-dispatches
//! the next kanban card or exits clean, never halts.

mod clean_exit;
/// Shared drained-board terminal verdict (ALL_BLOCKED vs PLAN nudge), emitted by
/// BOTH this group's `clean_exit` and the retry tail so the terminals never
/// diverge. `pub(in crate::gates::stop)` so the retry path can reach it too.
pub(in crate::gates::stop) mod drained;

pub(crate) use clean_exit::check as clean_exit;
