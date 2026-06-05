//! Terminal-group guard: the clean-exit emitter — the ONLY terminal stop. The
//! review-isolation and shallow-verdict HALT guards were removed under the
//! "kill blocking, keep auto-continue" policy; a Stop now either re-dispatches
//! the next kanban card or exits clean, never halts.

mod clean_exit;

pub(crate) use clean_exit::check as clean_exit;
