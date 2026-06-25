#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SysSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SysViolation {
    pub severity: SysSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}
