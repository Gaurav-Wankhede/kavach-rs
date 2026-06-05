/// Severity level for API boundary violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApiSeverity {
    /// Blocking violation requiring immediate fix.
    P0Block,
    /// Advisory warning for best practices.
    P1Advisory,
    /// Deprecation or style warning.
    P2Warning,
}

/// Detected API boundary contract violation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ApiViolation {
    /// Severity level of the violation.
    pub severity: ApiSeverity,
    /// Pattern name that triggered the violation.
    pub pattern: &'static str,
    /// Recommended fix for the violation.
    pub fix: &'static str,
    /// Line number in the file where the violation was detected.
    pub line: usize,
}
