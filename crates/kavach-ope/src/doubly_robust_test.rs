//! Doubly-Robust proofs — the defining property: unbiased if EITHER the reward
//! model OR the propensities are right. Two complementary tests pin both arms.
#![allow(
    clippy::float_cmp,
    reason = "exact arithmetic constants; deterministic, not measured"
)]
use super::estimate;
use crate::dm::RewardModel;
use crate::ips::FixedPolicy;
use crate::sample::{Action, LoggedSample};

/// Reward model returning a fixed value per action (context-free).
struct ConstByAction {
    allow: f64,
    ask: f64,
    block: f64,
}
impl RewardModel for ConstByAction {
    fn predict(&self, _ctx: &[f64], action: Action) -> f64 {
        match action {
            Action::Allow => self.allow,
            Action::Ask => self.ask,
            Action::Block => self.block,
        }
    }
}

#[test]
fn empty_is_non_gating() {
    let policy = FixedPolicy {
        allow: 1.0,
        ask: 0.0,
        block: 0.0,
    };
    let model = ConstByAction {
        allow: 0.0,
        ask: 0.0,
        block: 0.0,
    };
    assert!(estimate(&[], &policy, &model).std_error.is_infinite());
}

#[test]
fn exact_model_makes_the_correction_vanish() {
    // Model predicts each logged reward EXACTLY -> residual 0 -> DR == DM == the
    // model's value for the target action. Target always Allow, model Allow=1.
    let samples = vec![
        LoggedSample::with_context(Action::Allow, 0.5, 1.0, vec![]),
        LoggedSample::with_context(Action::Ask, 0.5, -1.0, vec![]),
    ];
    let policy = FixedPolicy {
        allow: 1.0,
        ask: 0.0,
        block: 0.0,
    };
    // Model is exact on both logged actions: Allow->1, Ask->-1.
    let model = ConstByAction {
        allow: 1.0,
        ask: -1.0,
        block: 0.0,
    };
    let est = estimate(&samples, &policy, &model);
    // Both per-sample values = baseline(1.0) + (pi/p)*(r - r_hat==0) = 1.0.
    assert_eq!(est.value, 1.0, "exact model -> DR is the DM baseline");
}

#[test]
fn correct_propensities_fix_a_biased_model() {
    // Model is WRONG (predicts 0 everywhere) but propensities are exact and the
    // target matches the logging policy (weight 1). DR's correction term becomes
    // (1)*(r - 0) = r, so DR recovers the IPS/mean reward DESPITE the bad model.
    let samples = vec![
        LoggedSample::with_context(Action::Allow, 0.5, 1.0, vec![]),
        LoggedSample::with_context(Action::Allow, 0.5, 0.0, vec![]),
        LoggedSample::with_context(Action::Allow, 0.5, 1.0, vec![]),
        LoggedSample::with_context(Action::Allow, 0.5, 0.0, vec![]),
    ];
    // Target assigns Allow 0.5 == logging propensity -> weight 1.
    let policy = FixedPolicy {
        allow: 0.5,
        ask: 0.25,
        block: 0.25,
    };
    let zero_model = ConstByAction {
        allow: 0.0,
        ask: 0.0,
        block: 0.0,
    };
    let est = estimate(&samples, &policy, &zero_model);
    // baseline 0 + 1*(r - 0) = r; mean(1,0,1,0) = 0.5.
    assert_eq!(
        est.value, 0.5,
        "correct propensities recover the value despite a zero model"
    );
}
