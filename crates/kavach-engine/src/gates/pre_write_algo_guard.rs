//! Algorithm Hunter pre-write guard.
//!
//! Block Rust writes that introduce non-trivial algorithmic logic without a
//! prior `/arch` invocation this turn, or auto-inject a prior algorithm
//! decision from kavach-db. The skill invocation is the only satisfier — never
//! an inline comment.
//!
//! Outcomes: `Allow` (no trigger / hunter invoked), `AutoInject` (prior DB
//! decision), `Block` (trigger, no decision, hunter not invoked).
mod check;
mod decision;
mod outcome;
mod strip;
mod triggers;
#[cfg(test)]
#[path = "pre_write_algo_guard_test.rs"]
#[path = "pre_write_algo_guard_test.rs"]
mod tests;
pub(crate) use check::check;
pub(crate) use outcome::AlgoGuardOutcome;
