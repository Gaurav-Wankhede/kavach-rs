//! Tests for the W4 local auto-commit gate. Git and RPC side effects need a repo
//! and daemon (absent in unit context), and this crate forbids `unsafe` so the
//! disable switch cannot be toggled via process env here. The deterministic
//! contract is pinned instead: `run` never unwinds on an empty or populated card
//! key (the fail-open path), and the disable-env constant name stays fixed.

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
