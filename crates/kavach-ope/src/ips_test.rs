//! IPS estimator math proofs. The load-bearing correctness property: when the
//! target policy reproduces the logging propensities, IPS recovers the plain
//! mean reward (every weight is 1) — the unbiasedness anchor.
#![allow(
    clippy::float_cmp,
    reason = "exact arithmetic constants; these are deterministic, not measured"
)]
use super::{FixedPolicy, estimate, estimate_self_normalized};
use crate::sample::{Action, LoggedSample};

#[test]
fn empty_input_is_non_gating() {
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 };
    let est = estimate(&[], &policy);
    assert_eq!(est.n, 0);
    assert_eq!(est.value, 0.0);
    assert!(est.std_error.is_infinite());
    // A no-data estimate must never greenlight a deploy.
    assert_eq!(est.lower_confidence_bound(1.96), f64::NEG_INFINITY);
}

#[test]
fn matching_policy_recovers_the_mean_reward() {
    // Logging policy took Allow w.p. 0.5 each time; target also assigns 0.5 to
    // Allow. Every weight pi(a)/p = 0.5/0.5 = 1, so IPS == mean(reward).
    let samples = vec![
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, -1.0),
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, 1.0),
    ];
    let policy = FixedPolicy { allow: 0.5, ask: 0.25, block: 0.25 };
    let est = estimate(&samples, &policy);
    // mean reward = (1 - 1 + 1 + 1) / 4 = 0.5.
    assert_eq!(est.value, 0.5);
    assert_eq!(est.n, 4);
    assert!(est.std_error.is_finite());
}

#[test]
fn upweights_actions_the_target_favors_more() {
    // Target assigns Allow prob 1.0 vs logging propensity 0.5 -> weight 2.
    // One sample, reward 1 -> IPS value = 2 * 1 = 2.
    let samples = vec![LoggedSample::new(Action::Allow, 0.5, 1.0)];
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 };
    let est = estimate(&samples, &policy);
    assert_eq!(est.value, 2.0);
    // A single sample has no variance information -> infinite SE.
    assert!(est.std_error.is_infinite());
}

#[test]
fn a_target_that_never_takes_the_logged_action_values_it_at_zero() {
    // Target assigns Block prob 0 -> weight 0 -> that sample contributes 0,
    // exactly the IPS behavior for unsupported actions.
    let samples = vec![
        LoggedSample::new(Action::Block, 0.3, 1.0),
        LoggedSample::new(Action::Block, 0.3, 1.0),
    ];
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 };
    let est = estimate(&samples, &policy);
    assert_eq!(est.value, 0.0);
}

#[test]
fn snips_matching_policy_also_recovers_the_mean_reward() {
    // All weights = 1 -> sum(w*r)/sum(w) = mean(r) = 0.5, same as plain IPS.
    let samples = vec![
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, -1.0),
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, 1.0),
    ];
    let policy = FixedPolicy { allow: 0.5, ask: 0.25, block: 0.25 };
    let est = estimate_self_normalized(&samples, &policy);
    assert_eq!(est.value, 0.5);
    assert!(est.std_error.is_finite());
}

#[test]
fn snips_normalizes_away_a_uniform_weight_scale() {
    // SNIPS divides by the weight SUM, so a constant weight (here 2x via the
    // target favoring Allow) cancels — value stays the plain mean reward, unlike
    // plain IPS which would scale it by 2. This is the variance-reduction win.
    let samples = vec![
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, 0.0),
    ];
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 }; // weight = 2 each
    let snips = estimate_self_normalized(&samples, &policy);
    assert_eq!(snips.value, 0.5, "SNIPS cancels the uniform 2x scale");
    let plain = estimate(&samples, &policy);
    assert_eq!(plain.value, 1.0, "plain IPS keeps the 2x scale: mean(2*1, 2*0)=1");
}

#[test]
fn snips_empty_and_unsupported_are_non_gating() {
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 };
    assert!(estimate_self_normalized(&[], &policy).std_error.is_infinite());
    // Target never takes the logged Block action -> weight sum 0 -> undefined.
    let block = vec![LoggedSample::new(Action::Block, 0.4, 1.0)];
    let est = estimate_self_normalized(&block, &policy);
    assert!(est.std_error.is_infinite());
    assert_eq!(est.lower_confidence_bound(1.96), f64::NEG_INFINITY);
}
