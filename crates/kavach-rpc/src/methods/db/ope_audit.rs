//! `db.ope_audit` RPC — the Layer-P5 reward-hacking audit over `bandit_log`.
//!
//! Two independent fail-closed checks from the harness-rl design §4, the gate a
//! candidate policy must clear BEFORE any promotion (it is stricter than, and
//! orthogonal to, the D4 value comparison in `db.ope_evaluate`):
//!
//! 1. C2 SAFETY FLOOR — over every logged decision that carries BOTH the rule
//!    action and the shadow (controller) action, count how many times the shadow
//!    would have RELAXED a hard rule `Block`. That count MUST be zero.
//! 2. TWO-TIER DRIFT — compare the HARD witness reward channel against the SOFT
//!    held-out (`held_out: true`) channel. If soft trails hard beyond tolerance,
//!    the policy is gaming the cheap witness ⇒ freeze + alarm.
//!
//! Pure report: no policy is promoted here. The promotion gate reads this verdict
//! and refuses to ship on any floor violation or a `Hacking` drift verdict.

use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_ope::Estimate;
use kavach_ope::audit::{AuditVerdict, detect_reward_hacking, first_floor_violation};
use kavach_ope::Action;
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "ope_audit_test.rs"]
mod tests;

/// Audit request: the scan budget plus the soft-vs-hard drift tolerance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct OpeAuditParams {
    /// Max bandit rows to load (newest first). 0 is treated as "no rows".
    pub limit: u32,
    /// How far the SOFT held-out value may trail the HARD witness value before it
    /// counts as reward hacking. A small positive slack absorbs sampling noise.
    pub drift_tolerance: f64,
}

/// The reward-hacking audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct OpeAuditResult {
    /// Whether the audit ran (false only on a store-load error).
    pub success: bool,
    /// PROMOTABLE iff the C2 floor held (no relaxed block) AND drift is not
    /// `Hacking`. A single failure on either axis sets this false — fail-closed.
    pub promotable: bool,
    /// Count of logged decisions where the shadow action would have RELAXED a
    /// hard rule `Block`. Must be 0; any positive value is a release blocker.
    pub floor_violations: usize,
    /// The two-tier drift verdict as a tag: `healthy` | `hacking` | `inconclusive`.
    pub drift: String,
    /// Observed `hard − soft` gap when `drift == "hacking"`, else 0.
    pub drift_gap: f64,
    /// Hard-channel (witness) sample count.
    pub hard_samples: usize,
    /// Soft-channel (held-out) sample count.
    pub soft_samples: usize,
    /// Set on a load error; the verdict fields are then fail-closed defaults.
    pub error: Option<String>,
}

/// Run the P5 reward-hacking audit over the logged decisions.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` only on transport failure; a store-load
/// error is reported in `OpeAuditResult.error` with `success = false` and a
/// non-promotable verdict.
pub async fn ope_audit(
    ctx: &AppState,
    params: OpeAuditParams,
) -> Result<OpeAuditResult, ErrorObjectOwned> {
    let raw = match kavach_surreal::list_bandit_rows(&ctx.db, params.limit).await {
        Ok(rows) => rows,
        Err(e) => return Ok(load_failed(e.to_string())),
    };

    let pairs: Vec<(Action, Action)> =
        raw.iter().filter_map(|json| rule_shadow_pair(json)).collect();
    let floor_violation = first_floor_violation(&pairs);
    let floor_violations = pairs.iter().filter(|&&(r, s)| relaxes_block(r, s)).count();

    let hard: Vec<f64> = raw.iter().filter_map(|j| channel_reward(j, false)).collect();
    let soft: Vec<f64> = raw.iter().filter_map(|j| channel_reward(j, true)).collect();
    let drift =
        detect_reward_hacking(&mean_estimate(&hard), &mean_estimate(&soft), params.drift_tolerance);

    let (tag, gap) = match drift {
        AuditVerdict::Healthy => ("healthy", 0.0),
        AuditVerdict::Hacking { gap } => ("hacking", gap),
        // AuditVerdict is #[non_exhaustive]; Inconclusive + any future verdict is
        // non-promotable by default (fail-closed).
        AuditVerdict::Inconclusive | _ => ("inconclusive", 0.0),
    };
    let promotable = floor_violation.is_none() && matches!(drift, AuditVerdict::Healthy);

    Ok(OpeAuditResult {
        success: true,
        promotable,
        floor_violations,
        drift: tag.to_owned(),
        drift_gap: gap,
        hard_samples: hard.len(),
        soft_samples: soft.len(),
        error: None,
    })
}

/// Whether `(rule, shadow)` is a relaxation of a hard block — the predicate the
/// floor-violation count tallies (the audit crate decides per-pair; we only
/// negate its floor check to count, not just find-first).
const fn relaxes_block(rule: Action, shadow: Action) -> bool {
    !kavach_ope::audit::safety_floor_held(rule, shadow)
}

/// Project a bandit row into the `(rule_action, shadow_action)` pair the C2 floor
/// audits, or `None` when the row lacks a shadow action (most rows — only canary
/// shadow rows carry both).
fn rule_shadow_pair(json: &str) -> Option<(Action, Action)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let rule = action_of(v.get("action")?.as_str()?)?;
    let shadow = action_of(v.get("shadow_action")?.as_str()?)?;
    Some((rule, shadow))
}

/// The realized reward for a row, IF it belongs to the requested channel.
/// `want_held_out = false` ⇒ the hard witness channel (rows without
/// `held_out: true`); `true` ⇒ the soft held-out channel.
fn channel_reward(json: &str, want_held_out: bool) -> Option<f64> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let held_out = v.get("held_out").and_then(serde_json::Value::as_bool).unwrap_or(false);
    if held_out != want_held_out {
        return None;
    }
    reward_scalar(v.get("reward")?)
}

/// Map a `snake_case` action string to the OPE action.
fn action_of(s: &str) -> Option<Action> {
    match s {
        "allow" => Some(Action::Allow),
        "ask" => Some(Action::Ask),
        "block" => Some(Action::Block),
        _ => None,
    }
}

/// Map the wire `reward` enum to its scalar: `verified_clean = +1`,
/// `needed_ask = 0`, `false_decision = -1`; `null`/absent → `None`.
fn reward_scalar(v: &serde_json::Value) -> Option<f64> {
    match v.as_str()? {
        "verified_clean" => Some(1.0),
        "needed_ask" => Some(0.0),
        "false_decision" => Some(-1.0),
        _ => None,
    }
}

/// A sample-mean estimate over a reward channel, with the standard error of the
/// mean. An empty channel yields a zero-sample, infinite-SE estimate so the audit
/// reads it as non-informative (`Inconclusive`) rather than a confident zero.
#[expect(
    clippy::float_arithmetic,
    reason = "computing a sample mean + SE for the audit's two reward channels; the estimator math proper lives in kavach-ope"
)]
fn mean_estimate(rewards: &[f64]) -> Estimate {
    let n = rewards.len();
    if n == 0 {
        return Estimate::non_informative();
    }
    let denom = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let mean = rewards.iter().sum::<f64>() / denom;
    let var = rewards.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / denom;
    Estimate::new(mean, (var / denom).sqrt(), n)
}

/// The fail-closed result for a store-load failure: not promotable, zero
/// channels, the error surfaced.
const fn load_failed(error: String) -> OpeAuditResult {
    OpeAuditResult {
        success: false,
        promotable: false,
        floor_violations: 0,
        drift: String::new(),
        drift_gap: 0.0,
        hard_samples: 0,
        soft_samples: 0,
        error: Some(error),
    }
}
