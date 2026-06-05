//! Inverse Propensity Scoring — the unbiased baseline OPE estimator.
//!
//! For each logged sample `(a, p, r)`, reweight its reward by how much more (or
//! less) likely the TARGET policy is to take `a` than the logging policy was:
//! `w = π(a) / p`. The mean of `w · r` is an unbiased estimate of the target
//! policy's value. Unbiased but high-variance when `w` is large — hence the CI.

use crate::estimate::Estimate;
use crate::sample::{Action, LoggedSample};

/// A target policy as its action probabilities. The estimator only needs the
/// probability the policy assigns to the action that was actually logged, so a
/// policy is just `Action -> probability in [0, 1]`.
pub trait TargetPolicy {
    /// Probability the target policy assigns to `action` for this sample.
    fn prob(&self, action: Action) -> f64;
}

/// A fixed (context-free) action distribution — the simplest target policy.
/// Useful for evaluating "always allow" / "ask 20% of the time" style rules.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FixedPolicy {
    /// P(Allow).
    pub allow: f64,
    /// P(Ask).
    pub ask: f64,
    /// P(Block).
    pub block: f64,
}

impl TargetPolicy for FixedPolicy {
    fn prob(&self, action: Action) -> f64 {
        match action {
            Action::Allow => self.allow,
            Action::Ask => self.ask,
            Action::Block => self.block,
        }
    }
}

/// Estimate the target policy's value from logged samples via IPS, with a
/// normal-approximation standard error.
///
/// Empty input yields a zero value with infinite SE (no data → no confidence),
/// so its lower bound is `-inf` and never greenlights a deploy.
#[must_use]
pub fn estimate<P: TargetPolicy>(samples: &[LoggedSample], policy: &P) -> Estimate {
    let n = samples.len();
    if n == 0 {
        return Estimate { value: 0.0, std_error: f64::INFINITY, n: 0 };
    }
    // Per-sample IPS reward: (pi(a)/p) * r.
    let weighted: Vec<f64> = samples
        .iter()
        .map(|s| (policy.prob(s.action) / s.propensity) * s.reward)
        .collect();

    // Sample counts are far below 2^52, so f64 represents them exactly; the
    // lossy-cast lint is conservative here. u32 cap keeps it provably exact.
    let n_f = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let mean = weighted.iter().sum::<f64>() / n_f;

    // Sample variance of the per-sample estimates; SE = sqrt(var / n). With a
    // single sample the variance is undefined, so SE is infinite (no spread
    // information) — again a non-gating lower bound.
    let std_error = if n == 1 {
        f64::INFINITY
    } else {
        let var = weighted.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / (n_f - 1.0);
        (var / n_f).sqrt()
    };

    Estimate { value: mean, std_error, n }
}

/// Self-normalized IPS (SNIPS) — divide the weighted reward sum by the weight
/// sum instead of by `n`.
///
/// Plain IPS is unbiased but its variance explodes when importance weights are
/// large or don't average to 1 (common when the logging policy is near-
/// deterministic — exactly the rule-gate case). SNIPS trades a small bias for a
/// large variance reduction by normalizing: `sum(w*r) / sum(w)`. It is the more
/// reliable point estimate in practice; the CI here is the delta-method SE.
#[must_use]
pub fn estimate_self_normalized<P: TargetPolicy>(samples: &[LoggedSample], policy: &P) -> Estimate {
    let n = samples.len();
    if n == 0 {
        return Estimate { value: 0.0, std_error: f64::INFINITY, n: 0 };
    }
    let weights: Vec<f64> = samples.iter().map(|s| policy.prob(s.action) / s.propensity).collect();
    let weight_sum: f64 = weights.iter().sum();
    if weight_sum <= 0.0 {
        // No target-policy support over the logged actions -> value undefined.
        return Estimate { value: 0.0, std_error: f64::INFINITY, n };
    }
    let weighted_reward: f64 =
        weights.iter().zip(samples).map(|(w, s)| w * s.reward).sum();
    let value = weighted_reward / weight_sum;

    // Delta-method SE: var of (w*(r - value)) / weight_sum, scaled by n. With a
    // single sample there is no spread information, so SE is infinite.
    let std_error = if n == 1 {
        f64::INFINITY
    } else {
        let n_f = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
        let residual_var: f64 = weights
            .iter()
            .zip(samples)
            .map(|(w, s)| (w * (s.reward - value)).powi(2))
            .sum::<f64>()
            / (n_f - 1.0);
        let mean_weight = weight_sum / n_f;
        (residual_var / n_f).sqrt() / mean_weight
    };

    Estimate { value, std_error, n }
}

#[cfg(test)]
#[path = "ips_test.rs"]
mod tests;
