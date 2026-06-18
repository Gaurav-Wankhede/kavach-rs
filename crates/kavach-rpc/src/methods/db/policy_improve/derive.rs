//! Candidate-policy derivation for `db.policy_improve`: estimate each action's
//! off-policy value, then pessimistically choose the best — the advisory-scope
//! recommendation the three promotion gates then audit.
//
//   deterministic target policies), then RSCB-MC pessimistic `choose`. CHOICE:
//   global (un-bucketed) estimate for V1 — coarse but high-coverage; finer
//   context buckets fragment coverage and suppress promotion (see roadmap).
//   TIME: O(A*N), A=3 actions, N=samples. SPACE: O(A). YEAR: 2026.
use kavach_ope::controller::{ActionValue, AdvisoryCandidates, GateScope, RiskConfig, choose};
use kavach_ope::ips::FixedPolicy;
use kavach_ope::{Action, Estimate, LoggedSample};

use super::super::ope_shared::{MeanRewardModel, mean_reward};

/// The learned advisory recommendation: the chosen action, its DR estimate, and
/// the deterministic action distribution put on it.
pub(super) struct Candidate {
    /// The chosen action's Doubly-Robust estimate (the promote gate's value).
    pub estimate: Estimate,
    /// Candidate P(Allow).
    pub allow: f64,
    /// Candidate P(Ask).
    pub ask: f64,
    /// Candidate P(Block).
    pub block: f64,
}

/// Estimate each action's value, pessimistically choose the best (advisory
/// scope), and return the deterministic candidate policy on it.
pub(super) fn derive_candidate(samples: &[LoggedSample], z: f64) -> Candidate {
    let model = MeanRewardModel {
        mean: mean_reward(samples),
    };
    let values: Vec<ActionValue> = [Action::Allow, Action::Ask, Action::Block]
        .iter()
        .map(|&a| {
            let est = kavach_ope::doubly_robust::estimate(samples, &fixed_on(a), &model);
            ActionValue::new(a, est)
        })
        .collect();
    // Advisory scope: policy_improve is the offline advisory-learning path; a P0
    // gate never reaches here (it could not satisfy AdvisoryCandidates::new).
    let cfg = RiskConfig::new(z, 0.0);
    let action = AdvisoryCandidates::new(&values, GateScope::Advisory)
        .map_or(Action::Ask, |adv| choose(adv, cfg));
    let estimate = values
        .iter()
        .find(|v| v.action == action)
        .map_or_else(Estimate::non_informative, |v| v.estimate);
    let (allow, ask, block) = distribution(action);
    Candidate {
        estimate,
        allow,
        ask,
        block,
    }
}

/// A deterministic fixed policy putting all mass on `a`.
const fn fixed_on(a: Action) -> FixedPolicy {
    let (allow, ask, block) = distribution(a);
    FixedPolicy::new(allow, ask, block)
}

/// The deterministic action distribution `(allow, ask, block)` for `a`.
const fn distribution(a: Action) -> (f64, f64, f64) {
    match a {
        Action::Allow => (1.0, 0.0, 0.0),
        Action::Block => (0.0, 0.0, 1.0),
        // Ask + any future variant -> defer-to-human distribution (safe default).
        Action::Ask | _ => (0.0, 1.0, 0.0),
    }
}
