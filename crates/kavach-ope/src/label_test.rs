//! Reward-labeler proofs. The load-bearing one is the reward-hacking guard: a
//! FALSE BLOCK (dev overrode, then it verified clean) scores -1, the same as a
//! false allow — so an over-firing gate is never free to the optimizer.
#![allow(
    clippy::float_cmp,
    reason = "exact reward scalars; deterministic, not measured"
)]
use super::{VerifyOutcome, label, reward_tag};
use crate::sample::Action;

#[test]
fn reward_tag_maps_the_scalar_to_the_wire_enum() {
    assert_eq!(
        reward_tag(Action::Allow, VerifyOutcome::VerifiedClean),
        "verified_clean"
    );
    assert_eq!(
        reward_tag(Action::Ask, VerifyOutcome::VerifiedClean),
        "verified_clean"
    );
    assert_eq!(
        reward_tag(Action::Allow, VerifyOutcome::VerifyFailed),
        "false_decision"
    );
    assert_eq!(
        reward_tag(Action::Block, VerifyOutcome::BlockedAndAccepted),
        "needed_ask"
    );
}

#[test]
fn allowing_good_work_is_rewarded() {
    assert_eq!(label(Action::Allow, VerifyOutcome::VerifiedClean), 1.0);
    assert_eq!(label(Action::Ask, VerifyOutcome::VerifiedClean), 1.0);
}

#[test]
fn a_false_allow_is_penalized() {
    // Let a breaking change through -> verify failed -> costly.
    assert_eq!(label(Action::Allow, VerifyOutcome::VerifyFailed), -1.0);
}

#[test]
fn a_false_block_is_penalized_exactly_like_a_false_allow() {
    // THE reward-hacking guard: a block the dev overrode that then verified
    // clean is a false positive, scored -1 — same cost as a false allow. If this
    // were 0, the optimizer would learn that blocking everything is free.
    let false_block = label(Action::Block, VerifyOutcome::BlockedThenOverriddenClean);
    let false_allow = label(Action::Allow, VerifyOutcome::VerifyFailed);
    assert_eq!(false_block, -1.0);
    assert_eq!(
        false_block, false_allow,
        "over-firing must cost as much as under-firing"
    );
}

#[test]
fn an_accepted_block_is_a_neutral_abstention() {
    // The block stood, no counterfactual observed -> neither reward nor penalty.
    assert_eq!(label(Action::Block, VerifyOutcome::BlockedAndAccepted), 0.0);
    assert_eq!(label(Action::Ask, VerifyOutcome::BlockedAndAccepted), 0.0);
}

#[test]
fn an_inconsistent_log_fails_closed_to_neutral() {
    // Allow paired with a block outcome is an impossible/garbled log -> 0, never
    // an invented reward.
    assert_eq!(label(Action::Allow, VerifyOutcome::BlockedAndAccepted), 0.0);
}
