//! RSCB-MC — the risk-sensitive contextual-bandit controller (Layer C).
//!
//! A lightweight hot-path policy over the fixed action set `{Allow, Ask, Block}`.
//!
//! Not an LLM, no rollouts, no hosted inference — just a pessimistic value
//! comparison over per-action estimates the OPE layer already produces.
//!
//! Two design invariants, both fail-closed (design C2 / the P5 reward-hacking
//! guard):
//! - PESSIMISTIC: each action is scored by its LOWER confidence bound
//!   (`value − z·std_error`), never its point estimate, so a high-variance lucky
//!   mean cannot win. Over-conservative by construction.
//! - ABSTENTION: `Ask` (defer to the human) is the safe action. When no action's
//!   pessimistic score clears a confidence floor, the controller abstains to
//!   `Ask` rather than guess.
//!
//! Scope (design D2): this tunes only ADVISORY gates. Hard P0/forbid gates bypass
//! the controller entirely and stay static — they are NOT in this action set's
//! decision surface.

use crate::estimate::Estimate;
use crate::sample::Action;

#[cfg(test)]
#[path = "controller_test.rs"]
mod tests;

/// One action's offline-estimated value, paired with the action it scores.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ActionValue {
    /// The action this estimate is for.
    pub action: Action,
    /// Its off-policy value estimate (point value + CI).
    pub estimate: Estimate,
}

impl ActionValue {
    /// Pair an action with its estimate.
    #[must_use]
    pub const fn new(action: Action, estimate: Estimate) -> Self {
        Self { action, estimate }
    }
}

/// The risk-sensitive controller's configuration.
///
/// `z` sets how pessimistic the per-action score is (higher = more
/// conservative); `confidence_floor` is the minimum pessimistic score an action
/// must clear to be chosen over abstention.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RiskConfig {
    /// z-score for the lower confidence bound (e.g. 1.96 ≈ 95%).
    pub z: f64,
    /// An action's LCB must exceed this to be selected; otherwise the controller
    /// abstains to `Ask`. A floor of `0.0` means "only act when the pessimistic
    /// estimate is net-positive".
    pub confidence_floor: f64,
}

impl RiskConfig {
    /// The default conservative configuration: ~95% pessimism, act only on a
    /// net-positive pessimistic estimate.
    #[must_use]
    pub const fn conservative() -> Self {
        Self { z: 1.96, confidence_floor: 0.0 }
    }
}

/// Choose an action by the risk-sensitive pessimistic rule.
///
/// Scores each candidate by its lower confidence bound at `cfg.z`, then:
/// - if the best LCB clears `cfg.confidence_floor`, take that action;
/// - otherwise ABSTAIN to [`Action::Ask`] (the safe default), even if `Ask` was
///   not among the candidates — uncertainty defers to the human.
///
/// Empty input abstains to `Ask`. Ties are broken toward the more conservative
/// action (`Block` > `Ask` > `Allow`) so a coin-flip never lands on the riskier
/// `Allow`.
#[must_use]
pub fn choose(candidates: &[ActionValue], cfg: RiskConfig) -> Action {
    let mut best: Option<(f64, Action)> = None;
    for c in candidates {
        let lcb = c.estimate.lower_confidence_bound(cfg.z);
        if !lcb.is_finite() {
            continue; // an infinite-SE estimate carries no information.
        }
        match best {
            Some((best_lcb, best_action)) if !beats(lcb, c.action, best_lcb, best_action) => {}
            _ => best = Some((lcb, c.action)),
        }
    }
    match best {
        Some((lcb, action)) if lcb > cfg.confidence_floor => action,
        // No candidate cleared the floor (or all were non-informative) → abstain.
        _ => Action::Ask,
    }
}

/// Whether candidate `(lcb, action)` should replace the current best
/// `(best_lcb, best_action)`: a strictly higher LCB wins; an exact tie breaks
/// toward the MORE conservative action so uncertainty never favors `Allow`.
fn beats(lcb: f64, action: Action, best_lcb: f64, best_action: Action) -> bool {
    if lcb > best_lcb {
        return true;
    }
    if lcb < best_lcb {
        return false;
    }
    conservatism(action) > conservatism(best_action)
}

/// Conservatism rank: `Block` (2) is safest, then `Ask` (1), then `Allow` (0).
const fn conservatism(action: Action) -> u8 {
    match action {
        Action::Block => 2,
        Action::Ask => 1,
        Action::Allow => 0,
    }
}

/// The canary promotion gate (design D4): a CANDIDATE policy ships only if its
/// pessimistic value strictly beats the INCUMBENT's pessimistic value.
///
/// Both are compared at the SAME `z`, so a candidate with a higher point estimate
/// but wider CI does not win on optimism alone. Returns false on a tie or when
/// either estimate is non-informative (infinite SE) — fail-closed: keep the
/// incumbent unless the challenger is provably better.
#[must_use]
pub fn promote(candidate: &Estimate, incumbent: &Estimate, z: f64) -> bool {
    let cand_lcb = candidate.lower_confidence_bound(z);
    let inc_lcb = incumbent.lower_confidence_bound(z);
    cand_lcb.is_finite() && inc_lcb.is_finite() && cand_lcb > inc_lcb
}
