//! `db.policy_improve` RPC — THE closure orchestrator (the heart of P6).
//!
//! Loads the reward-back-filled `bandit_log` rows, derives a learned advisory
//! policy (per-action Doubly-Robust value + RSCB-MC `choose`), then UPSERTs it
//! into the `deployed_policy` graph node ONLY if it clears all three fail-closed
//! gates in order: GATE-A trust coverage (`DataCOPE` ESS/n ≥ floor), GATE-B the
//! reward-hacking audit (reused verbatim from `db.ope_audit`), GATE-C a STRICT
//! lower-confidence-bound win over the persisted incumbent. Persistence happens
//! NOWHERE else — that conjunction is the reward-hacking + ope-validity mitigation.
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_ope::Estimate;
use kavach_ope::controller::promote;
use kavach_ope::ips::FixedPolicy;
use kavach_surreal::{
    DeployedPolicyProps, graph_top_deployed_policies, graph_upsert_deployed_policy,
};
use serde::{Deserialize, Serialize};

use derive::derive_candidate;
use result::{Metrics, blocked_reason, finish, load_failed};

use super::ope_audit::{OpeAuditParams, ope_audit};
use super::ope_shared::sample_from_row;

mod derive;
mod result;

#[cfg(test)]
#[path = "policy_improve_test.rs"]
mod tests;

/// The canonical advisory-policy scope (one versioned singleton node).
const POLICY_SCOPE: &str = "policy.advisory.global";

/// Request for one offline policy-improvement pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct PolicyImproveParams {
    /// Max bandit rows to load (newest first). 0 is treated as "no rows".
    pub limit: u32,
    /// z-score for the lower confidence bound (e.g. 1.96 ≈ 95%).
    pub z: f64,
    /// Coverage floor the candidate must clear (ESS/n) — GATE-A.
    pub min_coverage_ratio: f64,
    /// Soft-vs-hard drift tolerance forwarded to the reward-hacking audit — GATE-B.
    pub drift_tolerance: f64,
}

/// The policy-improvement verdict: whether a learned policy was promoted, and if
/// not, which gate refused it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct PolicyImproveResult {
    /// Whether the pass ran (false only on a store-load / persist error).
    pub success: bool,
    /// Whether a new policy was promoted and persisted (all three gates cleared).
    pub promoted: bool,
    /// Which gate refused promotion, when not promoted.
    pub blocked_by: Option<String>,
    /// Candidate pessimistic value (LCB) at `z`.
    pub candidate_lcb: f64,
    /// Incumbent LCB the candidate had to beat (the floor the first time).
    pub incumbent_lcb: f64,
    /// `DataCOPE` coverage ratio (ESS/n) of the candidate over the logged data.
    pub coverage_ratio: f64,
    /// C2 safety-floor violations from the audit (must be 0 to promote).
    pub floor_violations: usize,
    /// Two-tier drift verdict tag from the audit.
    pub drift: String,
    /// Reward-filled samples the decision rested on.
    pub usable_samples: usize,
    /// Set on a load / persist error; verdict fields are then fail-closed defaults.
    pub error: Option<String>,
}

/// Run one offline policy-improvement pass; persist the learned policy IFF it
/// clears trust coverage AND the reward-hacking audit AND a strict LCB win.
///
/// # Errors
/// Returns an RPC error only on transport failure of the nested audit call; a
/// store-load or persist error is reported in the result with `success = false`.
pub async fn policy_improve(
    ctx: &AppState,
    params: PolicyImproveParams,
) -> Result<PolicyImproveResult, ErrorObjectOwned> {
    let raw = match kavach_surreal::list_bandit_rows(&ctx.db, params.limit).await {
        Ok(rows) => rows,
        Err(e) => return Ok(load_failed(e.to_string())),
    };
    let samples: Vec<_> = raw.iter().filter_map(|j| sample_from_row(j)).collect();
    if samples.is_empty() {
        return Ok(finish(false, Some("no_samples"), Metrics::empty()));
    }

    let cand = derive_candidate(&samples, params.z);
    let cand_policy = FixedPolicy::new(cand.allow, cand.ask, cand.block);
    let cand_lcb = cand.estimate.lower_confidence_bound(params.z);

    // GATE-A trust coverage; GATE-B reward-hacking audit (reused); GATE-C the
    // strict LCB win over the persisted incumbent (the floor on the first run).
    let trust = kavach_ope::trust::assess(&samples, &cand_policy);
    let audit = ope_audit(
        ctx,
        OpeAuditParams {
            limit: params.limit,
            drift_tolerance: params.drift_tolerance,
        },
    )
    .await?;
    // Fail closed on an incumbent-read fault. A swallowed error here would default
    // the incumbent LCB to 0.0, forging a false win in the GATE-C promotion check
    // below — deploying an untested policy on a transient DB blip. Surface it like
    // the bandit-load fault above (success=false).
    let incumbent_lcb = match graph_top_deployed_policies(&ctx.db, 1).await {
        Ok(rows) => rows.first().map_or(0.0, |p| p.lcb),
        Err(e) => return Ok(load_failed(e.to_string())),
    };
    let beats = promote(&cand.estimate, &Estimate::new(incumbent_lcb, 0.0, 1), params.z);

    let m = Metrics {
        candidate_lcb: cand_lcb,
        incumbent_lcb,
        coverage_ratio: trust.coverage_ratio,
        floor_violations: audit.floor_violations,
        drift: audit.drift.clone(),
        usable_samples: samples.len(),
    };
    if let Some(reason) = blocked_reason(
        trust.is_trustworthy(params.min_coverage_ratio),
        audit.floor_violations,
        audit.promotable,
        beats,
    ) {
        return Ok(finish(false, Some(reason), m));
    }

    // All gates cleared -> persist the learned advisory policy (single-writer).
    let props = DeployedPolicyProps::new(
        cand.allow,
        cand.ask,
        cand.block,
        cand_lcb,
        incumbent_lcb,
        trust.coverage_ratio,
        samples.len(),
    );
    if let Err(e) = graph_upsert_deployed_policy(&ctx.db, POLICY_SCOPE, &props).await {
        return Ok(load_failed(e.to_string()));
    }
    Ok(finish(true, None, m))
}
