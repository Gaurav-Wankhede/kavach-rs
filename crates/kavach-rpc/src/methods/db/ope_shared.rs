//! Shared OPE projection helpers — ONE definition of how a stored `bandit_log`
//! row becomes a `kavach_ope::LoggedSample`, reused by `db.ope_evaluate` and
//! `db.policy_improve` so the closure never forks the projection logic.
use kavach_ope::dm::RewardModel;
use kavach_ope::label::{action_from_tag, reward_scalar};
use kavach_ope::{Action, LoggedSample};

/// A constant reward model `r̂(x, a) = mean(reward)` — the low-variance DM anchor
/// for V1. It ignores context (every prediction is the global mean reward), so
/// Doubly-Robust reduces to IPS plus a mean baseline: unbiased if the logged
/// propensities are right, with DM's variance reduction. A context-aware model
/// (ridge over the features) is the Layer-C upgrade.
pub(super) struct MeanRewardModel {
    /// The single predicted reward (global mean over the usable samples).
    pub mean: f64,
}

impl RewardModel for MeanRewardModel {
    fn predict(&self, _context: &[f64], _action: Action) -> f64 {
        self.mean
    }
}

/// Mean realized reward over the usable samples (0.0 when empty) — the constant
/// DM model's single prediction.
#[expect(
    clippy::float_arithmetic,
    reason = "averaging rewards is the DM anchor's single statistic; the estimator math itself lives in kavach-ope"
)]
pub(super) fn mean_reward(samples: &[LoggedSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n = f64::from(u32::try_from(samples.len()).unwrap_or(u32::MAX));
    samples.iter().map(|s| s.reward).sum::<f64>() / n
}

/// Project one stored `BanditRow` JSON into a `LoggedSample`, or `None` if the
/// row is unparseable OR its reward is not yet back-filled (un-rewarded rows are
/// not usable for OPE — only graded decisions carry signal).
pub(super) fn sample_from_row(json: &str) -> Option<LoggedSample> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let action = action_from_tag(value.get("action")?.as_str()?)?;
    let propensity = value.get("propensity")?.as_f64()?;
    // None-reward rows are excluded — only a back-filled reward is usable signal.
    let reward = reward_scalar(value.get("reward")?.as_str()?)?;
    let context = context_features(value.get("context"));
    Some(LoggedSample::with_context(
        action, propensity, reward, context,
    ))
}

/// Project the `BanditContext` object into the numeric feature vector the Direct
/// Method consumes: `[diff_bytes, prior_fire_count, risk_level]`, where the risk
/// label is ordinal-encoded (low=0, medium=1, high=2, unknown=0).
fn context_features(ctx: Option<&serde_json::Value>) -> Vec<f64> {
    let Some(c) = ctx else { return Vec::new() };
    let diff_bytes = c
        .get("diff_bytes")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let prior = c
        .get("prior_fire_count")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let risk = c
        .get("intent_risk")
        .and_then(serde_json::Value::as_str)
        .map_or(0.0, |r| match r {
            "medium" => 1.0,
            "high" => 2.0,
            _ => 0.0,
        });
    vec![diff_bytes, prior, risk]
}

#[cfg(test)]
#[path = "ope_test.rs"]
mod tests;
