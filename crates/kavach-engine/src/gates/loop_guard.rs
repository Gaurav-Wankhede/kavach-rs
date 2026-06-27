//! Detect repeated/looping tool calls that waste tokens.
//!
//! Tracks recent bash commands in session state and blocks if the same command
//! (or a near-duplicate) is executed 3+ times within a sliding window of
//! `WINDOW_TURNS` turns. The sliding-window scan is `O(HISTORY_SIZE)` = O(20),
//! a bounded constant.
//!
//! `detect` owns the window scan + block message; `inspection` exempts pure
//! read-only commands re-run to verify a mutated file; `history` is the bounded
//! command ring + entry parsing.
mod detect;
mod history;
mod inspection;
#[cfg(test)]
#[path = "loop_guard_test.rs"]
mod tests;
pub(crate) use detect::check_bash_loop;
pub(crate) use history::record_command;
