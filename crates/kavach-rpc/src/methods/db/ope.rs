//! `db.ope_evaluate` RPC — the Layer-B offline policy-evaluation report.
//!
//! Loads the logged `bandit_log` tuples, keeps only the reward-back-filled ones,
//! projects each into a `kavach_ope::LoggedSample`, and scores a CANDIDATE fixed
//! policy by Doubly-Robust OPE plus a `DataCOPE` trust check. The result is the
//! deploy-gate verdict (D4): the candidate's pessimistic lower-confidence bound
//! and whether the logged data even covers it. No policy is promoted here — this
//! RPC only REPORTS; the controller (Layer C) decides.

use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_ope::dm::RewardModel;
use kavach_ope::ips::FixedPolicy;
use kavach_ope::{Action, LoggedSample};
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "ope_test.rs"]
mod tests;

/// The candidate policy to evaluate, as a fixed action distribution, plus the
/// scan budget. The three probabilities should sum to ~1; they are used verbatim
/// (the estimator does not renormalize).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct OpeEvaluateParams {
    /// Candidate P(Allow).
    pub allow: f64,
    /// Candidate P(Ask).
    pub ask: f64,
    /// Candidate P(Block).
    pub block: f64,
    /// Max bandit rows to load (newest first). 0 is treated as "no rows".
    pub limit: u32,
    /// z-score for the lower confidence bound (e.g. 1.96 ≈ 95%).
    pub z: f64,
    /// Coverage floor in [0, 1]; below it the estimate is flagged untrustworthy.
    pub min_coverage_ratio: f64,
}

/// The offline-evaluation report for the candidate policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct OpeEvaluateResult {
    /// Whether the evaluation ran (false only on a load/transport error).
    pub success: bool,
    /// Doubly-Robust point estimate of the candidate's mean reward per decision.
    pub value: f64,
    /// Pessimistic lower confidence bound at `z` — the number the deploy gate
    /// compares against the incumbent (ship iff this beats the incumbent's LCB).
    pub lower_confidence_bound: f64,
    /// `DataCOPE` coverage ratio `ESS / n` in [0, 1]; low = data barely covers the
    /// candidate, so the estimate is not believable.
    pub coverage_ratio: f64,
    /// Whether `coverage_ratio` cleared `min_coverage_ratio` (fail-closed gate).
    pub trustworthy: bool,
    /// Count of reward-back-filled rows actually used (None-reward rows excluded).
    pub usable_samples: usize,
    /// Set on a load error; the numeric fields are then non-gating defaults.
    pub error: Option<String>,
}

/// A constant reward model `r̂(x, a) = mean(reward)` — the low-variance DM anchor
/// for V1. It ignores context (every prediction is the global mean reward), so
/// Doubly-Robust reduces to IPS plus a mean baseline: unbiased if the logged
/// propensities are right, with DM's variance reduction. A context-aware model
/// (ridge over the features) is the Layer-C upgrade.
struct MeanRewardModel {
    mean: f64,
}

impl RewardModel for MeanRewardModel {
    fn predict(&self, _context: &[f64], _action: Action) -> f64 {
        self.mean
    }
}

/// Evaluate a candidate gate policy against the logged decisions (Layer B / D4).
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` only on transport-level failure; a store
/// load error is reported in `OpeEvaluateResult.error` with `success = false`.
pub async fn ope_evaluate(
    ctx: &AppState,
    params: OpeEvaluateParams,
) -> Result<OpeEvaluateResult, ErrorObjectOwned> {
    let raw = match kavach_surreal::list_bandit_rows(&ctx.db, params.limit).await {
        Ok(rows) => rows,
        Err(e) => return Ok(load_failed(e.to_string())),
    };

    let samples: Vec<LoggedSample> = raw.iter().filter_map(|json| sample_from_row(json)).collect();

    let policy = FixedPolicy::new(params.allow, params.ask, params.block);
    let model = MeanRewardModel { mean: mean_reward(&samples) };

    let estimate = kavach_ope::doubly_robust::estimate(&samples, &policy, &model);
    let trust = kavach_ope::trust::assess(&samples, &policy);

    Ok(OpeEvaluateResult {
        success: true,
        value: estimate.value,
        lower_confidence_bound: estimate.lower_confidence_bound(params.z),
        coverage_ratio: trust.coverage_ratio,
        trustworthy: trust.is_trustworthy(params.min_coverage_ratio),
        usable_samples: samples.len(),
        error: None,
    })
}

/// The non-gating result for a store-load failure: zero value, `-inf` lower
/// bound (never greenlights a deploy), not trustworthy.
const fn load_failed(error: String) -> OpeEvaluateResult {
    OpeEvaluateResult {
        success: false,
        value: 0.0,
        lower_confidence_bound: f64::NEG_INFINITY,
        coverage_ratio: 0.0,
        trustworthy: false,
        usable_samples: 0,
        error: Some(error),
    }
}

/// Mean realized reward over the usable samples (0.0 when empty) — the constant
/// DM model's single prediction.
#[expect(
    clippy::float_arithmetic,
    reason = "averaging rewards is the DM anchor's single statistic; the estimator math itself lives in kavach-ope"
)]
fn mean_reward(samples: &[LoggedSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let n = f64::from(u32::try_from(samples.len()).unwrap_or(u32::MAX));
    samples.iter().map(|s| s.reward).sum::<f64>() / n
}

/// Project one stored `BanditRow` JSON into a `LoggedSample`, or `None` if the
/// row is unparseable OR its reward is not yet back-filled (un-rewarded rows are
/// not usable for OPE — only graded decisions carry signal).
fn sample_from_row(json: &str) -> Option<LoggedSample> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let action = action_of(value.get("action")?.as_str()?)?;
    let propensity = value.get("propensity")?.as_f64()?;
    // None-reward rows are excluded — only a back-filled reward is usable signal.
    let reward = reward_scalar(value.get("reward")?)?;
    let context = context_features(value.get("context"));
    Some(LoggedSample::with_context(action, propensity, reward, context))
}

/// Map the `bandit_log` `action` string (`snake_case`) to the OPE action.
fn action_of(s: &str) -> Option<Action> {
    match s {
        "allow" => Some(Action::Allow),
        "ask" => Some(Action::Ask),
        "block" => Some(Action::Block),
        _ => None,
    }
}

/// Map the wire `reward` enum (`kavach_patterns::Reward`, `snake_case`) to its
/// scalar: `verified_clean = +1`, `needed_ask = 0`, `false_decision = -1`.
/// `null`/absent → `None` (un-rewarded; the caller drops the row).
fn reward_scalar(v: &serde_json::Value) -> Option<f64> {
    match v.as_str()? {
        "verified_clean" => Some(1.0),
        "needed_ask" => Some(0.0),
        "false_decision" => Some(-1.0),
        _ => None,
    }
}

/// Project the `BanditContext` object into the numeric feature vector the Direct
/// Method consumes: `[diff_bytes, prior_fire_count, risk_level]`, where the risk
/// label is ordinal-encoded (low=0, medium=1, high=2, unknown=0).
fn context_features(ctx: Option<&serde_json::Value>) -> Vec<f64> {
    let Some(c) = ctx else { return Vec::new() };
    let diff_bytes = c.get("diff_bytes").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
    let prior = c.get("prior_fire_count").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
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
