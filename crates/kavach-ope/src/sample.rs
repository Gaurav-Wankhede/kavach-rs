//! The logged bandit sample an estimator consumes — a denormalized projection
//! of one `bandit_log` row. The OPE crate is decoupled from kavach-patterns:
//! the caller maps a deserialized `BanditRow` into this minimal shape.

use serde::{Deserialize, Serialize};

/// A gate action, mirrored locally so this crate needs no kavach-patterns dep.
/// The wire string matches `bandit_log`'s `snake_case` so a row maps 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Action {
    /// Let the tool through.
    Allow,
    /// Defer to the human.
    Ask,
    /// Hard-stop the tool.
    Block,
}

/// One logged decision with its REALIZED reward. Only rewarded rows are usable
/// for OPE — a `None`-reward row (decision not yet verified) is filtered out by
/// the caller before estimation.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LoggedSample {
    /// The action the logging policy actually took.
    pub action: Action,
    /// The logging policy's propensity for `action` in (0, 1]. Must be > 0 for
    /// IPS to be defined (a zero-propensity sample cannot be reweighted).
    pub propensity: f64,
    /// The realized reward of that action (e.g. +1 verified-clean, -1 false).
    pub reward: f64,
    /// Context features `x` of the decision (e.g. diff bytes, prior fire count,
    /// risk level encoded numerically). Empty for IPS/SNIPS (which ignore `x`);
    /// the Direct-Method reward model consumes it.
    pub context: Vec<f64>,
}

impl LoggedSample {
    /// Construct a context-free sample (IPS/SNIPS path). Propensity is clamped
    /// into `(0, 1]` — a non-positive logged propensity is a logging bug, but
    /// clamping keeps IPS finite rather than dividing by zero (fail-closed: such
    /// a sample contributes ~its reward at weight 1, never an infinite spike).
    #[must_use]
    pub const fn new(action: Action, propensity: f64, reward: f64) -> Self {
        Self::with_context(action, propensity, reward, Vec::new())
    }

    /// Construct a sample carrying context features for the Direct Method.
    #[must_use]
    pub const fn with_context(
        action: Action,
        propensity: f64,
        reward: f64,
        context: Vec<f64>,
    ) -> Self {
        Self {
            action,
            propensity: propensity.clamp(f64::MIN_POSITIVE, 1.0),
            reward,
            context,
        }
    }
}
