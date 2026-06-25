#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-engine async_sync_guard; non_exhaustive => E0004"
)]
pub enum AsyncSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AsyncViolation {
    pub severity: AsyncSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}
