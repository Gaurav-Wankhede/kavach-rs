//! TDD: ope evaluate/audit CLI param builders. Pure constructors are the
//! unit-tested core; the RPC round-trip runs against the live daemon.
use super::*;

#[test]
fn evaluate_params_carry_candidate_and_bounds() {
    let p = build_evaluate_params(0.6, 0.3, 0.1, 500, 1.96, 0.2);
    assert!((p.allow - 0.6).abs() < f64::EPSILON);
    assert!((p.ask - 0.3).abs() < f64::EPSILON);
    assert!((p.block - 0.1).abs() < f64::EPSILON);
    assert_eq!(p.limit, 500);
    assert!((p.z - 1.96).abs() < f64::EPSILON);
}

#[test]
fn audit_params_carry_limit_and_tolerance() {
    let p = build_audit_params(500, 0.05);
    assert_eq!(p.limit, 500);
    assert!((p.drift_tolerance - 0.05).abs() < f64::EPSILON);
}
