//! Doubly-Robust (DR) — the estimator that is unbiased if EITHER the reward
//! model OR the logged propensities are correct.
//!
//! DR starts from the Direct Method's low-variance baseline `r̂(x, π)` and adds
//! an IPS-weighted correction on the part the model got wrong:
//!
//! ```text
//! v_DR(x) = Σ_a π(a|x)·r̂(x,a)  +  (π(a_logged|x) / p) · (r − r̂(x, a_logged))
//! ```
//!
//! If `r̂` is exact the correction term has mean zero (unbiased via the model);
//! if the propensities are exact the IPS part is unbiased regardless of `r̂`.
//! Lower variance than plain IPS (the model absorbs most of the signal) and less
//! biased than DM (the correction fixes model error) — the best default OPE
//! estimate for the deploy gate.

use crate::dm::RewardModel;
use crate::estimate::Estimate;
use crate::ips::TargetPolicy;
use crate::sample::{Action, LoggedSample};

const ACTIONS: [Action; 3] = [Action::Allow, Action::Ask, Action::Block];

/// Estimate the target policy's value by the Doubly-Robust method.
///
/// Per-sample value = DM baseline + IPS correction on the logged action's
/// residual; the estimate is their mean with the SE of that mean. Empty input
/// yields a zero value with infinite SE (non-gating).
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
    let per_sample: Vec<f64> = samples
        .iter()
        .map(|s| {
            let baseline: f64 = ACTIONS
                .iter()
                .map(|&a| policy.prob(a) * model.predict(&s.context, a))
                .sum();
            let residual = s.reward - model.predict(&s.context, s.action);
            let correction = (policy.prob(s.action) / s.propensity) * residual;
            baseline + correction
        })
        .collect();

    let n_f = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let mean = per_sample.iter().sum::<f64>() / n_f;
    let std_error = if n == 1 {
        f64::INFINITY
    } else {
        let var = per_sample.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n_f - 1.0);
        (var / n_f).sqrt()
    };

    Estimate {
        value: mean,
        std_error,
        n,
    }
}

#[cfg(test)]
#[path = "doubly_robust_test.rs"]
mod tests;
