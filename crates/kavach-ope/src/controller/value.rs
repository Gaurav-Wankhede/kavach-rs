//! Action-value pairing + the risk-sensitive controller's configuration — the
//! data the selector and promotion gate score over.
use crate::estimate::Estimate;
use crate::sample::Action;

/// One action's offline-estimated value, paired with the action it scores.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ActionValue {
    /// The action this estimate is for.
    pub action: Action,
    /// Its off-policy value estimate (point value + CI).
    pub estimate: Estimate,
}

impl ActionValue {
    /// Pair an action with its estimate.
    #[must_use]
    pub const fn new(action: Action, estimate: Estimate) -> Self {
        Self { action, estimate }
    }
}

/// The risk-sensitive controller's configuration.
///
/// `z` sets how pessimistic the per-action score is (higher = more
/// conservative); `confidence_floor` is the minimum pessimistic score an action
/// must clear to be chosen over abstention.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RiskConfig {
    /// z-score for the lower confidence bound (e.g. 1.96 ≈ 95%).
    pub z: f64,
    /// An action's LCB must exceed this to be selected; otherwise the controller
    /// abstains to `Ask`. A floor of `0.0` means "only act when the pessimistic
    /// estimate is net-positive".
    pub confidence_floor: f64,
}

impl RiskConfig {
    /// The default conservative configuration: ~95% pessimism, act only on a
    /// net-positive pessimistic estimate.
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            z: 1.96,
            confidence_floor: 0.0,
        }
    }

    /// A configuration with explicit pessimism `z` and acceptance `confidence_floor`.
    #[must_use]
    pub const fn new(z: f64, confidence_floor: f64) -> Self {
        Self { z, confidence_floor }
    }
}
