//! Test-enforcement injection: escalate a `[TEST_ENFORCEMENT]` nudge into
//! intent-gate output when the session carries pending (untested) edits.
mod action;
mod context;
mod path;
#[cfg(test)]
#[path = "test_inject_test.rs"]
mod tests;
pub(crate) use context::build_test_context;
