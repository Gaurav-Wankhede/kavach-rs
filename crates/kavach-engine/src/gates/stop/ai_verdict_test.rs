//! Tests for the WITNESS-DERIVED completion verdict (operator directive 2026-06-18).
//!
//! The verdict is bound to the objective workspace witnesses, NEVER to prose.
//! These target the PURE [`verdict_from_witness`] map so they run instantly —
//! the impure [`extract_ai_verdict`] would spawn the minutes-long cargo witnesses
//! (and timed out under nextest), which is exactly why the pure map was split out.
use super::{WitnessRun, verdict_from_witness};

#[test]
fn passed_witness_is_net_advance() {
    assert_eq!(verdict_from_witness(WitnessRun::Passed), Some(true));
}

#[test]
fn failed_witness_is_regression() {
    assert_eq!(verdict_from_witness(WitnessRun::Failed), Some(false));
}

#[test]
fn spawn_error_is_regression_fail_closed() {
    // A Rust project whose cargo could not even spawn is a regression, never an
    // advance — fail-closed so a broken toolchain is never rewarded as success.
    assert_eq!(verdict_from_witness(WitnessRun::SpawnError), Some(false));
}

#[test]
fn unprovable_abstains() {
    // Non-Rust + no KAVACH_VERIFY_CMD: abstain (None) rather than fabricate a
    // reward — the labeler never invents evidence it does not have.
    assert_eq!(verdict_from_witness(WitnessRun::Unprovable), None);
}

#[test]
fn no_witness_outcome_maps_to_a_positive_reward_except_passed() {
    // The ONLY path to a +1 is a genuine `Passed`; every other outcome is <=0.
    // This is the anti-hallucination invariant: a reward requires a passing build.
    for run in [
        WitnessRun::Failed,
        WitnessRun::SpawnError,
        WitnessRun::Unprovable,
    ] {
        assert_ne!(
            verdict_from_witness(run),
            Some(true),
            "only Passed may yield +1: {run:?} must not"
        );
    }
}
