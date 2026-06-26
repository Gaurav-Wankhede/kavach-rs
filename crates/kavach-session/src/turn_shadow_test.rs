use super::{RewardOutcome, SessionState};

#[test]
fn store_turn_shadow_caps_and_marks_pending() {
    let mut s = SessionState::default();
    let big = "x".repeat(900);
    s.store_turn_shadow(&big);
    assert!(s.turn_shadow.len() <= 800);
    assert!(s.turn_shadow_pending());
}

#[test]
fn pending_advisories_fifo_cap_three() {
    let mut s = SessionState::default();
    s.queue_pending_advisory("one");
    s.queue_pending_advisory("two");
    s.queue_pending_advisory("three");
    s.queue_pending_advisory("four");
    assert_eq!(s.pending_advisories.len(), 3);
    assert_eq!(s.pending_advisories[0], "two");
}

#[test]
fn drain_pending_advisories_returns_then_empties() {
    // The carry-forward contract: a stop-gate advisory (e.g. an un-interrogated
    // loophole) queued on turn N must be drained by turn N+1's intent injector
    // EXACTLY ONCE — present on the first drain, gone on the second, so the next
    // turn's context is not nagged forever (replay/idempotency lens).
    let mut s = SessionState::default();
    s.queue_pending_advisory("[LOOPHOLE] interrogate the lease path");
    s.queue_pending_advisory("[LOOPHOLE] interrogate the auth path");
    let drained = s.drain_pending_advisories().expect("first drain non-empty");
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0], "[LOOPHOLE] interrogate the lease path");
    assert!(s.pending_advisories.is_empty(), "queue cleared after drain");
    assert!(
        s.drain_pending_advisories().is_none(),
        "second drain is None — advisory surfaced once, not re-nagged"
    );
}

#[test]
fn take_relay_payload_merges_and_clears() {
    let mut s = SessionState::default();
    s.store_turn_shadow("[INTENT] type:fix");
    s.queue_pending_advisory("verify failed on card X");
    let payload = s
        .take_relay_payload(super::RelayFlush::Full)
        .expect("non-empty relay");
    assert!(payload.contains("[INTENT]"));
    assert!(payload.contains("[POST_TOOL_RELAY]"));
    assert!(!s.turn_shadow_pending());
    assert!(s.pending_advisories.is_empty());
    assert!(s.take_relay_payload(super::RelayFlush::Full).is_none());
}

#[test]
fn advisories_only_preserves_turn_shadow_for_pre_write() {
    let mut s = SessionState::default();
    s.store_turn_shadow("[INTENT] type:fix");
    s.queue_pending_advisory("post-tool: verify failed");
    let adv = s
        .take_relay_payload(super::RelayFlush::AdvisoriesOnly)
        .expect("advisories");
    assert!(adv.contains("[POST_TOOL_RELAY]"));
    assert!(!adv.contains("[INTENT]"));
    assert!(s.turn_shadow_pending());
    assert!(!s.turn_shadow.is_empty());
    let full = s
        .take_relay_payload(super::RelayFlush::Full)
        .expect("shadow on write");
    assert!(full.contains("[INTENT]"));
    assert!(!s.turn_shadow_pending());
}

#[test]
fn queue_lifecycle_relay_merges_into_shadow() {
    let mut s = SessionState::default();
    s.store_turn_shadow("[INTENT] type:fix");
    s.queue_lifecycle_relay("[PRE_COMPACT] keep facts");
    assert!(s.turn_shadow.contains("[INTENT]"));
    assert!(s.turn_shadow.contains("[PRE_COMPACT]"));
    assert!(s.turn_shadow_pending());
}

#[test]
fn record_reward_outcome_tracks_pass_rate() {
    let mut s = SessionState::default();
    s.record_reward_outcome("unit.a", RewardOutcome::Passed);
    s.record_reward_outcome("unit.b", RewardOutcome::Failed);
    assert_eq!(s.reward_session_pass, 1);
    assert_eq!(s.reward_session_total, 2);
    assert!(s.last_reward_summary.contains("unit.b"));
}

#[test]
fn ai_judged_outcomes_are_graded_samples() {
    // RLAIF: an AI-judged verdict is a first-class ±1 sample (NOT an abstention),
    // so the bandit learns where the mechanical 3-witness oracle would be blind.
    let mut s = SessionState::default();
    s.record_reward_outcome("unit.ai-good", RewardOutcome::AiJudged(true));
    s.record_reward_outcome("unit.ai-bad", RewardOutcome::AiJudged(false));
    assert_eq!(s.reward_session_pass, 1, "AI-good is a pass");
    assert_eq!(s.reward_session_total, 2, "both AI verdicts count");
    assert!(s.last_reward_summary.contains("RLAIF"));
}

#[test]
fn abstention_is_neutral_not_a_failure() {
    // L2 regression: a card with NO verification signal (None) must not be
    // scored a failure, and must NOT count toward the graded total/pass rate.
    let mut s = SessionState::default();
    s.record_reward_outcome("unit.a", RewardOutcome::Passed);
    s.record_reward_outcome("unit.no-receipt", RewardOutcome::Abstain);
    assert_eq!(s.reward_session_pass, 1, "abstention adds no pass");
    assert_eq!(
        s.reward_session_total, 1,
        "abstention is not a graded sample"
    );
    assert!(
        s.last_reward_summary.contains("ABSTAINED"),
        "no-signal reads as abstained, never FAILED (-1.0)"
    );
    assert!(
        !s.last_reward_summary.contains("FAILED"),
        "absence of a receipt must never be tagged a -1.0 failure"
    );
}
