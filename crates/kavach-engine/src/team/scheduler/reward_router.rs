//! Outcome-rewarded routing — the Conductor RL lesson applied to the vendor pool.
//!
//! Each dispatch outcome (a [`Reward`] back-filled when the 3-witness lands) is
//! accumulated per `(vendor, role)`. [`RewardRouter::preferred_vendor`] then
//! returns the highest-scoring vendor for a role, so routing *evolves* toward the
//! backend that actually verifies clean — instead of a frozen static map.
//!
//! SOURCE: decision.fugu-orchestration-layer · https://sakana.ai/fugu/
use std::collections::HashMap;

use kavach_patterns::bandit_log::Reward;

use crate::team::AgentRole;

/// Per-`(vendor, role)` reward accumulator driving adaptive vendor selection.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RewardRouter {
    /// (role, vendor) → summed reward scalar. i32 holds long histories without
    /// the i8 per-reward range overflowing.
    scores: HashMap<(AgentRole, String), i32>,
}

impl RewardRouter {
    /// Fold one dispatch outcome into the running score for `(vendor, role)`.
    pub fn record(&mut self, vendor: &str, role: AgentRole, reward: Reward) {
        let slot = self.scores.entry((role, vendor.to_owned())).or_insert(0);
        *slot = slot.saturating_add(i32::from(reward.value()));
    }

    /// Current accumulated score for `(vendor, role)` (0 if never recorded).
    #[must_use]
    pub fn score(&self, vendor: &str, role: AgentRole) -> i32 {
        self.scores
            .get(&(role, vendor.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    /// The highest-scoring vendor seen for `role`, or `None` if no outcome has
    /// been recorded for it yet (caller falls back to the static `RolePool`).
    /// Ties break deterministically by vendor id so the result is stable.
    #[must_use]
    pub fn preferred_vendor(&self, role: AgentRole) -> Option<String> {
        self.scores
            .iter()
            .filter(|((r, _), _)| *r == role)
            .max_by(|((_, va), sa), ((_, vb), sb)| sa.cmp(sb).then_with(|| vb.cmp(va)))
            .map(|((_, vendor), _)| vendor.clone())
    }
}

#[cfg(test)]
#[path = "reward_router_test.rs"]
mod reward_router_test;
