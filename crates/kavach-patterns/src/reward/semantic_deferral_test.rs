//! Tests for the semantic-deferral backstop. The card's VERIFY: a paraphrased
//! handoff that evades `deferral_pattern()` still scores as a deferral here.

use super::is_semantic_deferral;
use crate::reward::presets;

/// Build the literal-regex first pass so a test can prove a message EVADES it.
fn regex_first_pass(msg: &str) -> bool {
    regex::Regex::new(presets::deferral_pattern()).is_ok_and(|re| re.is_match(msg))
}

/// A paraphrased handoff the literal regex misses is still caught semantically.
#[test]
fn paraphrased_handoff_evades_regex_but_judge_catches_it() {
    let msg = "The card is scoped. I'll leave that to you — you can run the build when ready.";
    assert!(!regex_first_pass(msg), "must evade the literal regex");
    assert!(is_semantic_deferral(msg), "judge must catch the paraphrase");
}

/// "take it from here" + an actor cue is a deferral.
#[test]
fn take_it_from_here_is_a_deferral() {
    assert!(is_semantic_deferral(
        "Setup is done; feel free to take it from here, you could deploy next."
    ));
}

/// A genuine completion summary with neutral "you" prose is NOT a deferral.
#[test]
fn neutral_completion_summary_is_not_a_deferral() {
    let msg = "Shipped the audit-trail handler. This lets you read your own trail; \
               3-witness verified, all tests pass.";
    assert!(!is_semantic_deferral(msg));
}

/// A handoff verb with NO second-person actor cue does not trip (conservative AND).
#[test]
fn handoff_verb_without_actor_cue_is_not_a_deferral() {
    assert!(!is_semantic_deferral(
        "I will take it from here and finish the remaining sites myself."
    ));
}

/// Empty / whitespace message is never a deferral (boundary).
#[test]
fn empty_message_is_not_a_deferral() {
    assert!(!is_semantic_deferral("   "));
}
