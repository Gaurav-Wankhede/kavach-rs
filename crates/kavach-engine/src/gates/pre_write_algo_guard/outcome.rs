//! Outcome of the algorithm guard check.

/// Three-way verdict from the algorithm pre-write guard.
pub(crate) enum AlgoGuardOutcome {
    /// Write is approved — no action needed.
    Allow,
    /// Write is approved, but inject prior decision as advisory context.
    AutoInject(String),
    /// Write is blocked — invoke /arch first.
    Block(String),
}
