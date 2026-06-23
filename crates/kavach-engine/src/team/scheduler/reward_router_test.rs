//! TDD: outcome-rewarded routing — the Conductor RL lesson. Dispatch outcomes
//! accumulate per (vendor, role); the router prefers the highest-scoring vendor.
//! SOURCE: decision.fugu-orchestration-layer · https://sakana.ai/fugu/
use super::*;
use crate::team::AgentRole;
use kavach_patterns::bandit_log::Reward;

#[test]
fn fresh_router_has_no_preference() {
    let r = RewardRouter::default();
    assert_eq!(r.preferred_vendor(AgentRole::Worker), None);
}

#[test]
fn higher_reward_sum_wins_for_a_role() {
    let mut r = RewardRouter::default();
    r.record("codex", AgentRole::Worker, Reward::VerifiedClean);
    r.record("codex", AgentRole::Worker, Reward::VerifiedClean);
    r.record("cc", AgentRole::Worker, Reward::FalseDecision);
    assert_eq!(r.preferred_vendor(AgentRole::Worker), Some("codex".to_owned()));
}

#[test]
fn routing_is_per_role_not_global() {
    let mut r = RewardRouter::default();
    r.record("cc", AgentRole::Thinker, Reward::VerifiedClean);
    r.record("codex", AgentRole::Worker, Reward::VerifiedClean);
    assert_eq!(r.preferred_vendor(AgentRole::Thinker), Some("cc".to_owned()));
    assert_eq!(r.preferred_vendor(AgentRole::Worker), Some("codex".to_owned()));
}

#[test]
fn negative_outcomes_demote_a_vendor() {
    let mut r = RewardRouter::default();
    r.record("cc", AgentRole::Worker, Reward::VerifiedClean);
    r.record("cc", AgentRole::Worker, Reward::FalseDecision);
    r.record("cc", AgentRole::Worker, Reward::FalseDecision);
    // net -1 < codex's single +1
    r.record("codex", AgentRole::Worker, Reward::VerifiedClean);
    assert_eq!(r.preferred_vendor(AgentRole::Worker), Some("codex".to_owned()));
}

#[test]
fn neutral_ask_does_not_change_ranking() {
    let mut r = RewardRouter::default();
    r.record("cc", AgentRole::Worker, Reward::NeededAsk);
    assert_eq!(r.score("cc", AgentRole::Worker), 0);
}
