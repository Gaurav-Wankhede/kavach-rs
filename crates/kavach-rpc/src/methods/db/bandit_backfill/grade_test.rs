//! Proofs for the pure P3a grading map: action + session outcome → reward tag.

use super::reward_tag_for_row;

const ALLOW: &str = r#"{"action":"allow"}"#;
const ASK: &str = r#"{"action":"ask"}"#;
const BLOCK: &str = r#"{"action":"block"}"#;

#[test]
fn a_passing_session_rewards_the_let_through_decisions_as_verified_clean() {
    // Only Allow/Ask "let the work through" — a passing session proves those
    // were right (+1 ⇒ verified_clean).
    assert_eq!(reward_tag_for_row(ALLOW, true), Some("verified_clean"));
    assert_eq!(reward_tag_for_row(ASK, true), Some("verified_clean"));
}

#[test]
fn a_block_in_a_passing_session_stays_neutral_the_work_never_went_through() {
    // A Block stopped the work; the session passing on OTHER work proves nothing
    // about that block (no counterfactual) ⇒ neutral (0 ⇒ needed_ask), never a
    // reward it didn't earn. This is the reward-hacking guard at the join.
    assert_eq!(reward_tag_for_row(BLOCK, true), Some("needed_ask"));
}

#[test]
fn a_failing_session_penalizes_only_the_allow_as_a_false_decision() {
    // The allowed change broke the build — a false allow (−1 ⇒ false_decision).
    assert_eq!(reward_tag_for_row(ALLOW, false), Some("false_decision"));
}

#[test]
fn a_block_or_ask_in_a_failing_session_is_neutral_no_counterfactual() {
    // No counterfactual for a block/ask that stood ⇒ neutral (0 ⇒ needed_ask).
    assert_eq!(reward_tag_for_row(BLOCK, false), Some("needed_ask"));
    assert_eq!(reward_tag_for_row(ASK, false), Some("needed_ask"));
}

#[test]
fn a_malformed_or_actionless_row_is_a_surfaced_skip() {
    assert_eq!(reward_tag_for_row("not json", true), None);
    assert_eq!(
        reward_tag_for_row(r#"{"propensity":1.0}"#, true),
        None,
        "no action ⇒ skip"
    );
    assert_eq!(
        reward_tag_for_row(r#"{"action":"garbage"}"#, true),
        None,
        "unknown action ⇒ skip"
    );
}
