//! The result of an off-policy value estimate: a point value plus the standard
//! error that makes its confidence interval — the load-bearing part, since a
//! candidate policy ships only if its lower bound beats the incumbent (D4).

/// A policy-value estimate with a normal-approximation confidence interval.
///
/// `value` is the estimated mean reward per decision under the target policy.
/// `std_error` is the standard error of that mean; the CI half-width is
/// `z * std_error`. From `n` samples; `n == 0` yields a zero-value, infinite-SE
/// estimate (no data → no confidence), so its lower bound never gates a deploy.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Estimate {
    /// Estimated value (mean reward per decision) of the target policy.
    pub value: f64,
    /// Standard error of `value`.
    pub std_error: f64,
    /// Number of usable samples the estimate was computed from.
    pub n: usize,
}

impl Estimate {
    /// The lower confidence bound at the given z-score (e.g. 1.96 for ~95%).
    ///
    /// This is the number a deploy decision compares against the incumbent: a
    /// pessimistic floor on the policy's value, so a high-variance estimate
    /// cannot greenlight a risky policy on a lucky point estimate alone.
    #[must_use]
    pub fn lower_confidence_bound(&self, z: f64) -> f64 {
        if !self.std_error.is_finite() {
            return f64::NEG_INFINITY;
        }
        z.mul_add(-self.std_error, self.value)
    }
}
