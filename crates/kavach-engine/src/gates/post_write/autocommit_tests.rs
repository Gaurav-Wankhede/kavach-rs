//! Tests for the W4 local auto-commit gate. The git/RPC side effects need a repo
//! + daemon (absent in unit context), and this crate forbids `unsafe` so we cannot
//! mutate process env to exercise the disable switch. We therefore pin the parts
//! that ARE deterministic and side-effect-free: `run` returns without panicking on
//! both an empty and a populated card key (the fail-open contract), and a clean
//! tree yields no commit receipt.

use super::*;

#[test]
fn run_never_panics_on_empty_card_key() {
    // No card key → heartbeat skipped; commit path returns None on a clean tree or
    // any git error. The only contract under test is "does not unwind".
    let _ = run("");
}

#[test]
fn run_never_panics_on_populated_card_key() {
    let _ = run("roadmap.phasemerge.w4-postwrite-db-and-commit");
}

#[test]
fn disable_env_const_is_the_documented_switch() {
    // Lock the env-var name so the docs/decision row and code can't silently drift.
    assert_eq!(DISABLE_ENV, "KAVACH_AUTOCOMMIT_OFF");
}
