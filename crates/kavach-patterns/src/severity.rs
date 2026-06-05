//! Shared severity tier + violation record for ALL pattern guards.
//!
//! WHY this exists: every `*_guard.rs` used to define its own private
//! `*Severity` enum (P0Block/P1Advisory/P2Warning) and `*Violation` struct —
//! the same three/four-variant concept copy-pasted 10+ times. That is the
//! duplication the micro-file + reuse laws forbid. Guards now import this one
//! type, so the tier ladder has a single source of truth.
//! SOURCE: <https://martinfowler.com/bliki/SingleSourceOfTruth.html>
//! SOURCE: <https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html>

/// Guard severity tier. Maps to a host-hook action (see kavach-engine
/// CLAUDE.md): P0 → block, P1 → advisory, P2 → warning, P3 → info.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    /// Irreversible / correctness-critical — host blocks the write.
    P0Block,
    /// Quality nudge — host pushes a P1 advisory.
    P1Advisory,
    /// Style hint — host pushes a lower-priority warning.
    P2Warning,
    /// Informational only.
    P3Info,
}

/// A single detected violation: which tier, what pattern, the fix, the line.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Violation {
    /// Severity tier of this hit.
    pub severity: Severity,
    /// Short name of the offending pattern (e.g. `"unwrap()"`).
    pub pattern: String,
    /// Actionable remediation text shown to the author.
    pub fix: String,
    /// 1-based source line of the hit.
    pub line: usize,
}

impl Violation {
    /// Construct a violation at a 1-based line.
    #[must_use]
    pub fn new(
        severity: Severity,
        pattern: impl Into<String>,
        fix: impl Into<String>,
        line: usize,
    ) -> Self {
        Self {
            severity,
            pattern: pattern.into(),
            fix: fix.into(),
            line,
        }
    }
}
