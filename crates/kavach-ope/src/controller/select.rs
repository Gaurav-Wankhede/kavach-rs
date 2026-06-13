//! Pessimistic action selection over an advisory candidate set.
use super::scope::AdvisoryCandidates;
use super::value::RiskConfig;
use crate::sample::Action;

/// Choose an action by the risk-sensitive pessimistic rule.
///
/// Scores each candidate by its lower confidence bound at `cfg.z`, then:
/// - if the best LCB clears `cfg.confidence_floor`, take that action;
/// - otherwise ABSTAIN to [`Action::Ask`] (the safe default), even if `Ask` was
///   not among the candidates — uncertainty defers to the human.
///
/// Input is an [`AdvisoryCandidates`] (advisory-scope only — a P0 gate cannot
/// construct one). Empty input abstains to `Ask`. Ties break toward the more
/// conservative action (`Block` > `Ask` > `Allow`) so a coin-flip never lands on
/// the riskier `Allow`.
#[must_use]
pub fn choose(candidates: AdvisoryCandidates<'_>, cfg: RiskConfig) -> Action {
    let mut best: Option<(f64, Action)> = None;
    for c in candidates.as_slice() {
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
