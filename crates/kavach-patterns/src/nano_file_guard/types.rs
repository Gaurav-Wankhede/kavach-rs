//! Public result types for the micro-file guard, split from the hub to keep it
//! under the 100-LOC ceiling the guard itself enforces.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched in kavach-engine pre_write_guards.rs"
)]
pub enum MicroSeverity {
    P0Block,
    P1Advisory,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MicroFileViolation {
    pub severity: MicroSeverity,
    pub pattern: &'static str,
    pub fix: String,
}
