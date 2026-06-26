//! RSCB-MC controller proofs. The load-bearing ones are the fail-closed
//! invariants: the controller abstains to `Ask` under uncertainty; a candidate
//! is promoted ONLY when its pessimistic value strictly beats the incumbent's;
//! and a hard-block gate cannot construct the advisory input `choose` requires.
#![allow(
    clippy::float_cmp,
    reason = "exact, deterministic estimates constructed in-test, not measured"
)]

use super::{ActionValue, AdvisoryCandidates, GateScope, RiskConfig, choose, promote};
use crate::estimate::Estimate;
use crate::sample::Action;

fn est(value: f64, std_error: f64) -> Estimate {
    Estimate {
        value,
        std_error,
        n: 100,
    }
}

/// Wrap advisory-scope candidates for `choose` (the only scope it accepts).
fn adv(c: &[ActionValue]) -> AdvisoryCandidates<'_> {
    AdvisoryCandidates::new(c, GateScope::Advisory).expect("advisory scope always wraps")
}

#[test]
fn picks_the_action_with_the_highest_lower_bound_not_highest_mean() {
    // Allow has a higher MEAN (0.9) but a huge CI; Block has a lower mean (0.5)
    // but is tight. Pessimism must prefer Block — the lucky mean does not win.
    let cands = [
        ActionValue::new(Action::Allow, est(0.9, 0.5)), // lcb @1.96 ≈ -0.08
        ActionValue::new(Action::Block, est(0.5, 0.05)), // lcb @1.96 ≈ 0.40
    ];
    assert_eq!(
        choose(adv(&cands), RiskConfig::conservative()),
        Action::Block
    );
}

#[test]
fn abstains_to_ask_when_no_action_clears_the_floor() {
    // Both actions have a NEGATIVE pessimistic value -> none clears floor 0.0 ->
    // the controller defers to the human (Ask), even though Ask was not a candidate.
    let cands = [
        ActionValue::new(Action::Allow, est(0.1, 0.5)),
        ActionValue::new(Action::Block, est(0.05, 0.5)),
    ];
    assert_eq!(choose(adv(&cands), RiskConfig::conservative()), Action::Ask);
}

#[test]
fn empty_candidates_abstain_to_ask() {
    assert_eq!(choose(adv(&[]), RiskConfig::conservative()), Action::Ask);
}

#[test]
fn a_non_informative_estimate_is_ignored() {
    // Infinite SE (n<2) carries no information -> skipped; the only informative
    // action that clears the floor wins.
    let cands = [
        ActionValue::new(Action::Allow, est(5.0, f64::INFINITY)),
        ActionValue::new(Action::Block, est(0.5, 0.05)),
    ];
    assert_eq!(
        choose(adv(&cands), RiskConfig::conservative()),
        Action::Block
    );
}

#[test]
fn an_exact_tie_breaks_toward_the_more_conservative_action() {
    // Identical estimates for Allow and Block -> the tie must resolve to Block,
    // never the riskier Allow.
    let cands = [
        ActionValue::new(Action::Allow, est(0.5, 0.05)),
        ActionValue::new(Action::Block, est(0.5, 0.05)),
    ];
    assert_eq!(
        choose(adv(&cands), RiskConfig::conservative()),
        Action::Block
    );
}

#[test]
fn a_hard_block_scope_cannot_construct_advisory_candidates() {
    // GATE-BYPASS MITIGATION: a P0/hard-block gate must NOT be able to feed the
    // learned controller. The scope guard refuses construction, fail-closed.
    let cands = [ActionValue::new(Action::Block, est(0.5, 0.05))];
    assert!(
        AdvisoryCandidates::new(&cands, GateScope::HardBlock).is_none(),
        "hard-block scope must never reach policy selection"
    );
}

#[test]
fn promote_requires_strictly_beating_the_incumbent_lower_bound() {
    let z = 1.96;
    // Candidate clearly better: higher mean, same CI.
    assert!(promote(&est(0.8, 0.05), &est(0.4, 0.05), z));
    // Candidate has a higher MEAN but a wide CI -> its LCB is below the
    // incumbent's tight LCB -> NOT promoted (no optimism wins).
    assert!(!promote(&est(0.9, 0.5), &est(0.4, 0.05), z));
}

#[test]
fn an_exact_value_tie_does_not_promote() {
    // Equal estimates -> keep the incumbent (fail-closed: only a strict win ships).
    assert!(!promote(&est(0.5, 0.05), &est(0.5, 0.05), 1.96));
}

#[test]
fn a_non_informative_candidate_never_promotes() {
    // Infinite-SE candidate -> LCB is -inf -> never beats a finite incumbent.
    assert!(!promote(&est(9.0, f64::INFINITY), &est(0.1, 0.05), 1.96));
}
