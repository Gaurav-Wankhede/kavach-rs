//! Proofs for the `ope.evaluate` payload→sample projection and the report
//! shape. The DB-bound `ope_evaluate` itself is covered by the kavach-surreal
//! integration roundtrip; here we pin the pure mapping logic that turns a stored
//! `BanditRow` JSON into a usable `LoggedSample`, which is where the silent bugs
//! (un-rewarded rows leaking in, a tagged-enum mis-parse) would hide.
#![allow(
    clippy::float_cmp,
    reason = "exact scalar rewards/probabilities; deterministic, not measured"
)]

use super::{context_features, sample_from_row};
use kavach_ope::label::reward_scalar;
use kavach_ope::Action;

#[test]
fn a_rewarded_row_projects_to_a_usable_sample() {
    let json = r#"{"session_id":"s1","timestamp_ms":1,
        "context":{"gate":"micro_file","tool":"Write","file_ext":"rs",
                   "diff_bytes":120,"intent_risk":"high","prior_fire_count":2},
        "action":"allow","propensity":1.0,"reward":"verified_clean"}"#;
    let s = sample_from_row(json).expect("rewarded row is usable");
    assert_eq!(s.action, Action::Allow);
    assert_eq!(s.reward, 1.0, "verified_clean maps to +1");
    assert_eq!(s.propensity, 1.0);
    // context = [diff_bytes, prior_fire_count, risk(high=2)]
    assert_eq!(s.context, vec![120.0, 2.0, 2.0]);
}

#[test]
fn an_unrewarded_row_is_dropped() {
    // reward = null -> not yet 3-witness-verified -> NOT usable for OPE.
    let json = r#"{"session_id":"s2","timestamp_ms":2,
        "context":{"diff_bytes":0,"intent_risk":"","prior_fire_count":0},
        "action":"block","propensity":1.0,"reward":null}"#;
    assert!(sample_from_row(json).is_none(), "a None-reward row must be excluded");
}

#[test]
fn a_malformed_row_is_dropped_not_panicked() {
    assert!(sample_from_row("not json").is_none());
    assert!(sample_from_row(r#"{"action":"allow"}"#).is_none(), "missing fields -> drop");
}

#[test]
fn the_false_decision_reward_is_negative_one() {
    let json = r#"{"context":{},"action":"block","propensity":1.0,"reward":"false_decision"}"#;
    let s = sample_from_row(json).expect("usable");
    assert_eq!(s.reward, -1.0, "a false decision is the costly -1");
}

#[test]
fn reward_scalar_maps_every_variant_and_rejects_unknown() {
    assert_eq!(reward_scalar("verified_clean"), Some(1.0));
    assert_eq!(reward_scalar("needed_ask"), Some(0.0));
    assert_eq!(reward_scalar("false_decision"), Some(-1.0));
    assert_eq!(reward_scalar("garbage"), None);
}

#[test]
fn context_features_default_to_zero_when_absent() {
    assert_eq!(context_features(None), Vec::<f64>::new());
    let empty = serde_json::json!({});
    assert_eq!(context_features(Some(&empty)), vec![0.0, 0.0, 0.0]);
}
