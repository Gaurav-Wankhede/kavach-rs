//! `db.ope_evaluate` RPC — the Layer-B offline policy-evaluation report.
//!
//! Loads the logged `bandit_log` tuples, keeps only the reward-back-filled ones,
//! projects each into a `kavach_ope::LoggedSample`, and scores a CANDIDATE fixed
//! policy by Doubly-Robust OPE plus a `DataCOPE` trust check. The result is the
//! deploy-gate verdict (D4): the candidate's pessimistic lower-confidence bound
//! and whether the logged data even covers it. No policy is promoted here — this
//! RPC only REPORTS; the controller (Layer C) decides.
//!
//! The projection helpers (`sample_from_row`, `mean_reward`, `MeanRewardModel`)
//! live in `ope_shared` so `db.policy_improve` reuses the exact same projection.

use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_ope::LoggedSample;
use kavach_ope::ips::FixedPolicy;
use serde::{Deserialize, Serialize};

use super::ope_shared::{MeanRewardModel, mean_reward, sample_from_row};

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

    let samples: Vec<LoggedSample> = raw
        .iter()
        .filter_map(|json| sample_from_row(json))
        .collect();

    let policy = FixedPolicy::new(params.allow, params.ask, params.block);
    let model = MeanRewardModel {
        mean: mean_reward(&samples),
    };

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
