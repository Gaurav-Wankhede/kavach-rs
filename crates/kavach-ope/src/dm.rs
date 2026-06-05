//! Direct Method (DM) — estimate value from a learned reward model `r̂(x, a)`.
//!
//! Where IPS reweights observed rewards, DM ignores the logged reward entirely
//! and instead predicts, for each logged context `x`, the reward the TARGET
//! policy would earn — `Σ_a π(a|x) · r̂(x, a)` — then averages over contexts.
//!
//! Low variance (no importance weights to explode), but BIASED by exactly the
//! reward model's error: a wrong `r̂` gives a confidently wrong value. That bias
//! is why Doubly-Robust (DM + an IPS correction on the residual) exists — DM
//! alone is the low-variance anchor, not the final word.

use crate::estimate::Estimate;
use crate::ips::TargetPolicy;
use crate::sample::{Action, LoggedSample};

/// A learned reward model: predict the reward for taking `action` in `context`.
/// The caller trains/loads this (e.g. a ridge regression over the context
/// features); the estimator only queries it.
pub trait RewardModel {
    /// Predicted reward `r̂(x, a)` for `action` given the context features.
    fn predict(&self, context: &[f64], action: Action) -> f64;
}

const ACTIONS: [Action; 3] = [Action::Allow, Action::Ask, Action::Block];

/// Estimate the target policy's value by the Direct Method over logged contexts.
///
/// For each sample, the per-context value is `Σ_a π(a|x) · r̂(x, a)`; the estimate
/// is their mean, with the standard error of that mean. Empty input yields a
/// zero value with infinite SE (non-gating).
#[must_use]
pub fn estimate<P, M>(samples: &[LoggedSample], policy: &P, model: &M) -> Estimate
where
    P: TargetPolicy,
    M: RewardModel,
{
    let n = samples.len();
    if n == 0 {
        return Estimate {
            value: 0.0,
            std_error: f64::INFINITY,
            n: 0,
        };
    }
    let per_context: Vec<f64> = samples
        .iter()
        .map(|s| {
            ACTIONS
                .iter()
                .map(|&a| policy.prob(a) * model.predict(&s.context, a))
                .sum()
        })
        .collect();

    let n_f = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let mean = per_context.iter().sum::<f64>() / n_f;
    let std_error = if n == 1 {
        f64::INFINITY
    } else {
        let var = per_context.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n_f - 1.0);
        (var / n_f).sqrt()
    };

    Estimate {
        value: mean,
        std_error,
        n,
    }
}

#[cfg(test)]
#[path = "dm_test.rs"]
mod tests;
