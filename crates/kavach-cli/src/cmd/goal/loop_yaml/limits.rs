// Loop-control limits for a goal loop. Every field maps to a Claude Code
// Workflow primitive in the compiler (`max_attempts` -> while-guard,
// `budget_floor` -> the Workflow `budget.remaining()` check).
//
// SOURCE: decision.goal-oracle-workflow.
use super::OnMaxAttempts;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Loop-control limits — the runaway-spend brakes. All fields are plain numeric
/// caps + a terminal-policy enum.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopLimits {
    /// Hard cap on attempts — the primary runaway-spend brake.
    pub max_attempts: u32,
    /// Stop looping if the Workflow's remaining token budget falls below this
    /// floor. YAML key kept as `budget_floor_tokens` for on-disk compatibility.
    #[serde(rename = "budget_floor_tokens")]
    pub budget_floor: u64,
    /// How many independent root-cause agents to fan out on a failed attempt.
    pub parallel_diagnostics: u32,
    /// Terminal policy when attempts run out.
    pub on_max_attempts: OnMaxAttempts,
}

// Hand-written Debug: every field is a plain numeric cap / policy enum (no
// secret material), printed in full. Satisfies the RUST196 sensitive-type gate
// that flags a derived Debug on a struct whose field name matches `budget`.
impl fmt::Debug for LoopLimits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoopLimits")
            .field("max_attempts", &self.max_attempts)
            .field("budget_floor", &self.budget_floor)
            .field("parallel_diagnostics", &self.parallel_diagnostics)
            .field("on_max_attempts", &self.on_max_attempts)
            .finish()
    }
}

impl Default for LoopLimits {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            budget_floor: 10_000,
            parallel_diagnostics: 0,
            on_max_attempts: OnMaxAttempts::Escalate,
        }
    }
}
