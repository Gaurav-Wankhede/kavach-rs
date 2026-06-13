//! The canary promotion gate (design D4).
use crate::estimate::Estimate;

/// A CANDIDATE policy ships only if its pessimistic value strictly beats the
/// INCUMBENT's pessimistic value.
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
