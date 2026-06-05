//! `DataCOPE` trust-check proofs. The guard property: equal importance weights
//! give full coverage (ESS == n); a single dominant weight collapses ESS toward
//! 1, so the estimate is flagged untrustworthy before any deploy decision.
#![allow(
    clippy::float_cmp,
    reason = "exact arithmetic constants; deterministic, not measured"
)]
use super::assess;
use crate::ips::FixedPolicy;
use crate::sample::{Action, LoggedSample};

#[test]
fn equal_weights_give_full_coverage() {
    // Target == logging propensity -> every weight 1 -> ESS == n, coverage 1.0.
    let samples = vec![
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, 0.0),
        LoggedSample::new(Action::Allow, 0.5, 1.0),
        LoggedSample::new(Action::Allow, 0.5, 0.0),
    ];
    let policy = FixedPolicy { allow: 0.5, ask: 0.25, block: 0.25 };
    let t = assess(&samples, &policy);
    assert_eq!(t.effective_sample_size, 4.0);
    assert_eq!(t.coverage_ratio, 1.0);
    assert!(t.is_trustworthy(0.5));
}

#[test]
fn one_dominant_weight_collapses_the_effective_sample_size() {
    // One sample has a tiny logging propensity -> a huge weight that dominates
    // Σw². ESS collapses far below n, so coverage is low and the estimate is
    // flagged untrustworthy — the data barely covers the target policy.
    let samples = vec![
        LoggedSample::new(Action::Allow, 0.001, 1.0), // weight ~1000
        LoggedSample::new(Action::Allow, 1.0, 0.0),
        LoggedSample::new(Action::Allow, 1.0, 0.0),
        LoggedSample::new(Action::Allow, 1.0, 0.0),
    ];
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 };
    let t = assess(&samples, &policy);
    assert!(t.effective_sample_size < 1.1, "ESS collapses to ~1, got {}", t.effective_sample_size);
    assert!(t.coverage_ratio < 0.3, "coverage low, got {}", t.coverage_ratio);
    assert!(!t.is_trustworthy(0.5), "must be flagged untrustworthy");
}

#[test]
fn empty_and_unsupported_are_untrustworthy() {
    let policy = FixedPolicy { allow: 1.0, ask: 0.0, block: 0.0 };
    let empty = assess(&[], &policy);
    assert_eq!(empty.coverage_ratio, 0.0);
    assert!(!empty.is_trustworthy(0.01));

    // Target never takes the logged Block action -> all weights 0 -> zero ESS.
    let unsupported = vec![LoggedSample::new(Action::Block, 0.5, 1.0)];
    let t = assess(&unsupported, &policy);
    assert_eq!(t.effective_sample_size, 0.0);
    assert!(!t.is_trustworthy(0.01));
}
