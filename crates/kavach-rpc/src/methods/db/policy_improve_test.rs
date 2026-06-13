//! Proofs for the policy-improve gates. The load-bearing ones (the adversarial
//! mitigations): a candidate is REFUSED promotion on ANY of low coverage, a C2
//! floor violation, or a non-promotable (Hacking/Inconclusive) audit — EVEN when
//! its LCB beats the incumbent. That fail-closed conjunction is the reward-
//! hacking + ope-validity guard. Plus `derive_candidate`'s pessimistic choice.
#![allow(
    clippy::float_cmp,
    reason = "deterministic in-test estimates, not measured"
)]

use super::derive::derive_candidate;
use super::result::blocked_reason;
use kavach_ope::{Action, LoggedSample};

#[test]
fn a_floor_violation_refuses_promotion_even_with_a_winning_lcb() {
    // beats_incumbent = true, but a C2 safety-floor violation blocks first.
    assert_eq!(blocked_reason(true, 1, false, true), Some("safety_floor"));
}

#[test]
fn a_non_promotable_audit_refuses_promotion_even_with_a_winning_lcb() {
    // Floor clean, but audit not promotable (Hacking/Inconclusive drift) -> block,
    // even though the candidate's LCB beats the incumbent.
    assert_eq!(blocked_reason(true, 0, false, true), Some("audit_drift"));
}

#[test]
fn low_coverage_refuses_promotion_even_with_a_winning_lcb() {
    // Untrustworthy coverage (the propensity=1.0 / no-exploration regime) refuses
    // before anything else — the ope-validity mitigation.
    assert_eq!(blocked_reason(false, 0, true, true), Some("trust_coverage"));
}

#[test]
fn an_lcb_loser_is_refused_even_when_every_other_gate_clears() {
    assert_eq!(
        blocked_reason(true, 0, true, false),
        Some("incumbent_not_beaten")
    );
}

#[test]
fn a_clean_trustworthy_audited_lcb_winner_is_allowed() {
    assert_eq!(blocked_reason(true, 0, true, true), None);
}

#[test]
fn derive_prefers_the_action_with_the_best_pessimistic_value() {
    // 10 Allow decisions rewarded +1, 10 Block decisions rewarded -1 (propensity
    // 1.0). The "always Allow" target picks up the positive residuals, "always
    // Block" the negative ones -> Allow has the best pessimistic value.
    let mut samples: Vec<LoggedSample> = (0..10)
        .map(|_| LoggedSample::new(Action::Allow, 1.0, 1.0))
        .collect();
    samples.extend((0..10).map(|_| LoggedSample::new(Action::Block, 1.0, -1.0)));

    let cand = derive_candidate(&samples, 1.96);
    // Deterministic on Allow ⇒ Allow was the pessimistically-best action.
    assert!(cand.allow > 0.99, "Allow (best LCB) chosen");
    assert!(cand.block < 0.01, "Block (worst value) not chosen");
}
