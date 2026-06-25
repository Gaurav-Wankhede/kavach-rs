#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched/constructed cross-crate; non_exhaustive => E0639/E0004"
)]
pub enum Rust196Severity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "matched/constructed cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct Rust196Violation {
    pub severity: Rust196Severity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}
