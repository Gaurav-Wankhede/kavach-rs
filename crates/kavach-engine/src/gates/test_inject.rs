//! Test-enforcement injection: escalate a `[TEST_ENFORCEMENT]` nudge into
//! intent-gate output when the session carries pending (untested) edits.
mod action;
mod context;
mod path;

#[cfg(test)]
mod tests;

pub(crate) use context::build_test_context;
