//! `PostToolUse:Bash` gate: memory sync + failure-signal context injection.
//!
//! `handle` (the orchestrator) lives in `handle`; pure classifiers in `detect`;
//! test-run bookkeeping in `tests_track`.
mod detect;
mod handle;
mod progress;
mod tests_track;
#[cfg(test)]
#[path = "post_tool_bash_test.rs"]
mod tests;
pub(crate) use handle::handle;
/// Public accessor for the trimming path (`track_state_only`).
#[must_use]
pub(crate) fn is_test_command_pub(cmd: &str) -> bool {
    detect::is_test_command(cmd)
}
