//! Crate-level proofs for the deploy-gate primitive: the lower confidence bound
//! is what a candidate gate policy is judged on (D4: ship iff LCB > incumbent).
#![allow(
    clippy::float_cmp,
    reason = "exact arithmetic constants; deterministic, not measured"
)]
use super::Estimate;

#[test]
fn lcb_is_value_minus_z_times_se() {
    let est = Estimate {
        value: 0.8,
        std_error: 0.1,
        n: 100,
    };
    // 95% LCB = 0.8 - 1.96 * 0.1 = 0.604.
    let lcb = est.lower_confidence_bound(1.96);
    assert!((lcb - 0.604).abs() < 1e-9, "got {lcb}");
}

#[test]
fn infinite_se_yields_neg_infinity_lcb() {
    // No spread information -> the floor is -inf, so the policy can never clear
    // an incumbent on a lucky point estimate alone.
    let est = Estimate {
        value: 5.0,
        std_error: f64::INFINITY,
        n: 1,
    };
    assert_eq!(est.lower_confidence_bound(1.96), f64::NEG_INFINITY);
}

#[test]
fn a_higher_lcb_policy_beats_a_lower_one() {
    // The actual deploy comparison: candidate ships iff its LCB > incumbent's.
    let incumbent = Estimate {
        value: 0.5,
        std_error: 0.05,
        n: 200,
    };
    let candidate = Estimate {
        value: 0.7,
        std_error: 0.04,
        n: 200,
    };
    let z = 1.96;
    assert!(
        candidate.lower_confidence_bound(z) > incumbent.lower_confidence_bound(z),
        "candidate with higher pessimistic floor must win"
    );
}
