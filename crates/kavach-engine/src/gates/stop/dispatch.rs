//! Dispatch-group guards: the autonomous-loop heart. `retry` owns the
//! stop-retry path (failure breaker, user-focus surfacing, three-tier re-block,
//! forced terminal). `first_pass` owns the first-attempt task→hunt→backlog
//! dispatch + loop budget. Each is its own single-responsibility child. These
//! are the `[AUTO_CONTINUE]` re-dispatch guards kept under the "kill blocking,
//! keep auto-continue" policy.

mod first_pass;
mod harness;
mod retry;

pub(crate) use first_pass::check as first_pass;
pub(crate) use harness::harness_suffix;
pub(crate) use retry::check as retry;
