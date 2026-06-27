//! DataCOPE-style trust check: is an OPE estimate believable, or is the logged
//! data too thin under the target policy to trust?
//!
//! IPS/SNIPS can return a confident-looking value from data that barely covers
//! the target policy — a handful of huge importance weights dominate. The
//! effective sample size `ESS = (Σw)² / Σw²` measures this: it equals `n` when
//! all weights are equal and collapses toward 1 when one weight dominates. A low
//! `ESS / n` ratio means "do not trust this estimate, gather more data" — the
//! fail-closed guard before any deploy decision (design: `DataCOPE` trust check).

use crate::ips::TargetPolicy;
use crate::sample::LoggedSample;

/// A trust verdict over the logged data for a target policy.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Trust {
    /// Effective sample size from the importance weights.
    pub effective_sample_size: f64,
    /// Raw sample count.
    pub n: usize,
    /// `ESS / n` in `[0, 1]`: 1.0 = perfect coverage, →0 = a few weights dominate.
    pub coverage_ratio: f64,
}

impl Trust {
    /// Whether the estimate clears a coverage floor (e.g. 0.1 = ESS ≥ 10% of n).
    ///
    /// Fail-closed: below the floor the data does not support the target policy,
    /// so a deploy decision must NOT rely on the OPE estimate.
    #[must_use]
    pub fn is_trustworthy(&self, min_coverage_ratio: f64) -> bool {
        self.coverage_ratio >= min_coverage_ratio
    }
}

/// Compute the trust verdict for a target policy over logged samples.
///
/// Empty input is untrustworthy (zero ESS, zero coverage) — no data supports any
/// estimate.
#[must_use]
pub fn assess<P: TargetPolicy>(samples: &[LoggedSample], policy: &P) -> Trust {
    let n = samples.len();
    if n == 0 {
        return Trust {
            effective_sample_size: 0.0,
            n: 0,
            coverage_ratio: 0.0,
        };
    }
    let weights = samples.iter().map(|s| policy.prob(s.action) / s.propensity);
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    for w in weights {
        sum += w;
        sum_sq = w.mul_add(w, sum_sq);
    }
    // ESS = (Σw)² / Σw². Σw² == 0 only if every weight is 0 (target supports none
    // of the logged actions) -> zero ESS, zero coverage.
    let ess = if sum_sq > 0.0 {
        (sum * sum) / sum_sq
    } else {
        0.0
    };
    let n_f = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    Trust {
        effective_sample_size: ess,
        n,
        coverage_ratio: ess / n_f,
    }
}

#[cfg(test)]
#[path = "trust_test.rs"]
#[cfg(test)]
#[path = "trust_test.rs"]
mod tests;