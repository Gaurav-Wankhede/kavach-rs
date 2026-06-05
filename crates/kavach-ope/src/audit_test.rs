//! Reward-hacking audit proofs. Two invariants, both fail-closed:
//! 1. the C2 safety floor — a learned action can NEVER relax a hard rule block;
//! 2. the two-tier drift monitor — soft (real) value below hard (witness) value
//!    by more than tolerance is reward hacking and freezes promotion.

use super::{
    AuditVerdict, detect_reward_hacking, first_floor_violation, safety_floor_held,
};
use crate::estimate::Estimate;
use crate::sample::Action;

// ---- C2 safety floor ---------------------------------------------------------

#[test]
fn a_hard_block_may_never_be_relaxed_by_the_learned_policy() {
    // The single condition that must never reach production: rule said Block,
    // the controller wants to soften it.
    assert!(!safety_floor_held(Action::Block, Action::Allow), "Block→Allow is a relaxation");
    assert!(!safety_floor_held(Action::Block, Action::Ask), "Block→Ask is a relaxation");
}

#[test]
fn matching_or_tightening_a_block_holds_the_floor() {
    assert!(safety_floor_held(Action::Block, Action::Block), "Block→Block holds");
}

#[test]
fn advisory_rule_verdicts_are_free_to_tune_either_direction() {
    // Allow/Ask are the controller's tuning surface — any learned action is
    // permitted there (the OPE-CI + canary gate it separately).
    for rule in [Action::Allow, Action::Ask] {
        for shadow in [Action::Allow, Action::Ask, Action::Block] {
            assert!(safety_floor_held(rule, shadow), "{rule:?}→{shadow:?} is advisory tuning");
        }
    }
}

#[test]
fn batch_audit_returns_the_first_floor_violation() {
    let pairs = [
        (Action::Allow, Action::Block), // fine — tightening an advisory
        (Action::Ask, Action::Allow),   // fine — advisory tuning
        (Action::Block, Action::Allow), // VIOLATION — relaxing a hard block
        (Action::Block, Action::Ask),   // also a violation, but later
    ];
    assert_eq!(first_floor_violation(&pairs), Some((Action::Block, Action::Allow)));
}

#[test]
fn batch_audit_is_clean_when_no_block_is_relaxed() {
    let pairs = [
        (Action::Allow, Action::Block),
        (Action::Block, Action::Block),
        (Action::Ask, Action::Allow),
    ];
    assert_eq!(first_floor_violation(&pairs), None);
}

// ---- two-tier drift monitor --------------------------------------------------

fn est(value: f64, n: usize) -> Estimate {
    Estimate { value, std_error: 0.1, n }
}

#[test]
fn soft_tracking_hard_within_tolerance_is_healthy() {
    let hard = est(0.80, 100);
    let soft = est(0.78, 40);
    assert_eq!(detect_reward_hacking(&hard, &soft, 0.05), AuditVerdict::Healthy);
}

#[test]
fn soft_far_below_hard_is_reward_hacking_with_the_gap() {
    // The policy passes the cheap 3-witness (0.9) but the real held-out
    // re-verification only earns 0.4 — it learned to game the witness.
    let hard = est(0.90, 100);
    let soft = est(0.40, 40);
    let verdict = detect_reward_hacking(&hard, &soft, 0.05);
    let gap = match verdict {
        AuditVerdict::Hacking { gap } => gap,
        AuditVerdict::Healthy | AuditVerdict::Inconclusive => f64::NAN,
    };
    assert!(gap.is_finite(), "expected Hacking, got {verdict:?}");
    assert!((gap - 0.50).abs() < 1e-9, "gap is hard−soft");
}

#[test]
fn a_soft_signal_above_hard_is_never_hacking() {
    // If real verification is BETTER than the witness, there is nothing to game.
    let hard = est(0.50, 100);
    let soft = est(0.70, 40);
    assert_eq!(detect_reward_hacking(&hard, &soft, 0.05), AuditVerdict::Healthy);
}

#[test]
fn no_soft_samples_is_inconclusive_not_healthy() {
    // Fail-closed: we cannot clear a policy with no real held-out signal yet.
    let hard = est(0.80, 100);
    let soft = est(0.0, 0);
    assert_eq!(detect_reward_hacking(&hard, &soft, 0.05), AuditVerdict::Inconclusive);
}

#[test]
fn an_infinite_se_estimate_is_inconclusive() {
    let hard = Estimate { value: 0.8, std_error: f64::INFINITY, n: 100 };
    let soft = est(0.4, 40);
    assert_eq!(detect_reward_hacking(&hard, &soft, 0.05), AuditVerdict::Inconclusive);
}
