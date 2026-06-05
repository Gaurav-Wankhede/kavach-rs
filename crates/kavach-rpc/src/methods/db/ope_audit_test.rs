//! Pure-function proofs for the P5 audit's row projection + reward channelling.
//! The async `ope_audit` itself needs a live store; here we prove the row-level
//! logic that decides what the audit counts, since that is where a silent
//! mis-parse would let a hacking row slip past.

use super::{channel_reward, mean_estimate, relaxes_block, rule_shadow_pair};
use kavach_ope::Action;
use kavach_ope::label::action_from_tag;

#[test]
fn a_canary_row_yields_its_rule_and_shadow_actions() {
    let json = r#"{"action":"block","shadow_action":"allow","reward":"verified_clean"}"#;
    assert_eq!(rule_shadow_pair(json), Some((Action::Block, Action::Allow)));
}

#[test]
fn a_row_without_a_shadow_action_is_not_a_pair() {
    // Ordinary (non-canary) rows carry only the rule action — they don't enter
    // the floor audit.
    let json = r#"{"action":"block","reward":"verified_clean"}"#;
    assert_eq!(rule_shadow_pair(json), None);
}

#[test]
fn relaxes_block_flags_only_a_softened_hard_block() {
    assert!(relaxes_block(Action::Block, Action::Allow));
    assert!(relaxes_block(Action::Block, Action::Ask));
    assert!(!relaxes_block(Action::Block, Action::Block));
    // Advisory rule verdicts are the tuning surface — never a violation.
    assert!(!relaxes_block(Action::Allow, Action::Block));
    assert!(!relaxes_block(Action::Ask, Action::Allow));
}

#[test]
fn hard_channel_excludes_held_out_rows_and_vice_versa() {
    let witness = r#"{"reward":"verified_clean"}"#;
    let held_out = r#"{"reward":"false_decision","held_out":true}"#;
    // hard channel (want_held_out=false)
    assert_eq!(channel_reward(witness, false), Some(1.0));
    assert_eq!(channel_reward(held_out, false), None);
    // soft channel (want_held_out=true)
    assert_eq!(channel_reward(witness, true), None);
    assert_eq!(channel_reward(held_out, true), Some(-1.0));
}

#[test]
fn an_unrewarded_row_contributes_to_no_channel() {
    let json = r#"{"reward":null}"#;
    assert_eq!(channel_reward(json, false), None);
    assert_eq!(channel_reward(json, true), None);
}

#[test]
fn action_from_tag_maps_the_snake_case_vocabulary() {
    assert_eq!(action_from_tag("allow"), Some(Action::Allow));
    assert_eq!(action_from_tag("ask"), Some(Action::Ask));
    assert_eq!(action_from_tag("block"), Some(Action::Block));
    assert_eq!(action_from_tag("garbage"), None);
}

#[test]
fn an_empty_channel_is_non_informative_not_a_confident_zero() {
    let e = mean_estimate(&[]);
    assert_eq!(e.n, 0);
    assert!(
        !e.std_error.is_finite(),
        "empty channel ⇒ infinite SE ⇒ Inconclusive"
    );
}

#[test]
fn a_populated_channel_has_a_finite_mean_and_se() {
    let e = mean_estimate(&[1.0, 1.0, -1.0]);
    assert_eq!(e.n, 3);
    assert!(e.std_error.is_finite());
    assert!((e.value - (1.0 / 3.0)).abs() < 1e-9, "mean of [1,1,-1]");
}
