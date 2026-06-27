//! Cargo-test command tracking: quote-aware command-position detection of
//! `cargo test`/`cargo nextest run`, plus the unscoped/duplicate-run gates.
//!
//! Parsing (`extract`) is separated from the gate predicates (`guards`) so the
//! CWE-184 quote-aware scanner can be tested in isolation from session state.
mod extract;
mod guards;
#[cfg(test)]
#[path = "test_tracker_test.rs"]
mod tests;
pub(in crate::gates::pre_tool_bash) use guards::{
    check_duplicate_test_run, check_unscoped_test_run, register_test_run,
};
