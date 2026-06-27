//! SOLID Gate — Rust Backend (SRP / OCP / LSP / ISP / DIP).
//! Structural anti-pattern detector tuned to P1Advisory/P2Warning.

#[expect(
    clippy::exhaustive_enums,
    reason = "closed severity set; exhaustively matched cross-crate"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidSeverity {
    P1Advisory,
    P2Warning,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidLetter {
    S,
    O,
    L,
    I,
    D,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SolidViolation {
    pub severity: SolidSeverity,
    pub letter: SolidLetter,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

mod dip_checks;
mod helpers;
mod isp_checks;
mod lsp_checks;
mod ocp_checks;
mod orchestrator;
mod other_checks;
mod pattern_strs;
mod srp_checks;

/// Detect SOLID violations. Returns every violation found, or empty vec for
/// non-Rust backend / test files.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<SolidViolation> {
    if !helpers::is_rust_backend_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let p = pattern_strs::get_patterns();
    let mut v = Vec::new();
    orchestrator::run_all(p, file_path, content, &mut v);
    v
}

#[must_use]
pub fn warn_count(file_path: &str, content: &str) -> usize {
    detect(file_path, content).len()
}

#[cfg(test)]
#[path = "solid_guard_test.rs"]
#[cfg(test)]
#[path = "solid_guard_test.rs"]
mod tests;