//! Direct-Method proofs. DM ignores logged rewards and averages the reward
//! model's prediction for the target policy's action mix — so a constant model
//! yields that constant, and the value tracks the model, not the logged data.
#![allow(
    clippy::float_cmp,
    reason = "exact arithmetic constants; deterministic, not measured"
)]
use super::{RewardModel, estimate};
use crate::ips::FixedPolicy;
use crate::sample::{Action, LoggedSample};

/// A reward model that returns a fixed value per action, ignoring context.
struct ConstByAction {
    allow: f64,
    ask: f64,
    block: f64,
}
impl RewardModel for ConstByAction {
    fn predict(&self, _context: &[f64], action: Action) -> f64 {
        match action {
            Action::Allow => self.allow,
            Action::Ask => self.ask,
            Action::Block => self.block,
        }
    }
}

fn sample(action: Action) -> LoggedSample {
    // Logged reward is deliberately wild (99) to prove DM IGNORES it.
    LoggedSample::with_context(action, 0.5, 99.0, vec![1.0, 2.0])
}

#[test]
fn empty_input_is_non_gating() {
    let policy = FixedPolicy {
        allow: 1.0,
        ask: 0.0,
        block: 0.0,
    };
    let model = ConstByAction {
        allow: 1.0,
        ask: 0.0,
        block: 0.0,
    };
    let est = estimate(&[], &policy, &model);
    assert_eq!(est.n, 0);
    assert!(est.std_error.is_infinite());
}

#[test]
fn dm_value_is_the_model_prediction_not_the_logged_reward() {
    // Target always Allow; model says Allow -> reward 1. DM value = 1 for every
    // context, regardless of the logged reward (99). Proves DM uses r-hat only.
    let samples = vec![sample(Action::Block), sample(Action::Ask)];
    let policy = FixedPolicy {
        allow: 1.0,
        ask: 0.0,
        block: 0.0,
    };
    let model = ConstByAction {
        allow: 1.0,
        ask: -1.0,
        block: -1.0,
    };
    let est = estimate(&samples, &policy, &model);
    assert_eq!(est.value, 1.0, "DM tracks the model, not the logged reward");
}

#[test]
fn dm_mixes_actions_by_the_target_policy_probabilities() {
    // Target: 0.5 Allow + 0.5 Ask. Model: Allow=2, Ask=0, Block=-10.
    // Per-context value = 0.5*2 + 0.5*0 + 0.0*(-10) = 1.0.
    let samples = vec![
        sample(Action::Allow),
        sample(Action::Allow),
        sample(Action::Allow),
    ];
    let policy = FixedPolicy {
        allow: 0.5,
        ask: 0.5,
        block: 0.0,
    };
    let model = ConstByAction {
        allow: 2.0,
        ask: 0.0,
        block: -10.0,
    };
    let est = estimate(&samples, &policy, &model);
    assert_eq!(est.value, 1.0);
    // Constant per-context values -> zero variance -> finite (zero) SE.
    assert_eq!(est.std_error, 0.0);
}
