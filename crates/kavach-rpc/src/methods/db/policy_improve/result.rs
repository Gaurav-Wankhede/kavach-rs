//! Result + gate-decision helpers for `db.policy_improve` — keeps the
//! ten-field DTO construction and the fail-closed blocking order out of the
//! orchestrator so the hub reads as load -> derive -> gate -> persist.
use super::PolicyImproveResult;

/// The gate metrics captured for the result regardless of outcome.
pub(super) struct Metrics {
    /// Candidate pessimistic value (LCB) at the configured `z`.
    pub candidate_lcb: f64,
    /// Incumbent LCB the candidate had to beat (the `confidence_floor` first time).
    pub incumbent_lcb: f64,
    /// `DataCOPE` coverage ratio (ESS/n) of the candidate over the logged data.
    pub coverage_ratio: f64,
    /// C2 safety-floor violations from the audit (must be 0 to promote).
    pub floor_violations: usize,
    /// Two-tier drift verdict tag from the audit.
    pub drift: String,
    /// Reward-filled samples the decision rested on.
    pub usable_samples: usize,
}

impl Metrics {
    /// The non-gating metrics for an early exit (no samples / load failure).
    pub(super) const fn empty() -> Self {
        Self {
            candidate_lcb: f64::NEG_INFINITY,
            incumbent_lcb: f64::NEG_INFINITY,
            coverage_ratio: 0.0,
            floor_violations: 0,
            drift: String::new(),
            usable_samples: 0,
        }
    }
}

/// The fail-closed blocking order: trust coverage, then C2 floor, then audit
/// drift, then the incumbent comparison. `None` means every gate cleared.
pub(super) const fn blocked_reason(
    trustworthy: bool,
    floor_violations: usize,
    audit_promotable: bool,
    beats_incumbent: bool,
) -> Option<&'static str> {
    if !trustworthy {
        Some("trust_coverage")
    } else if floor_violations > 0 {
        Some("safety_floor")
    } else if !audit_promotable {
        Some("audit_drift")
    } else if !beats_incumbent {
        Some("incumbent_not_beaten")
    } else {
        None
    }
}

/// Build the success-path result (`promoted` true only when every gate cleared).
pub(super) fn finish(
    promoted: bool,
    blocked_by: Option<&'static str>,
    m: Metrics,
) -> PolicyImproveResult {
    PolicyImproveResult {
        success: true,
        promoted,
        blocked_by: blocked_by.map(str::to_owned),
        candidate_lcb: m.candidate_lcb,
        incumbent_lcb: m.incumbent_lcb,
        coverage_ratio: m.coverage_ratio,
        floor_violations: m.floor_violations,
        drift: m.drift,
        usable_samples: m.usable_samples,
        error: None,
    }
}

/// The fail-closed result for a store-load / persist error: not promoted, the
/// error surfaced, numeric fields non-gating.
pub(super) fn load_failed(error: String) -> PolicyImproveResult {
    PolicyImproveResult {
        success: false,
        promoted: false,
        blocked_by: Some("error".to_owned()),
        candidate_lcb: f64::NEG_INFINITY,
        incumbent_lcb: f64::NEG_INFINITY,
        coverage_ratio: 0.0,
        floor_violations: 0,
        drift: String::new(),
        usable_samples: 0,
        error: Some(error),
    }
}
