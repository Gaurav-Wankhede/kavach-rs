#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaSeverity {
    P1Advisory,
    P2Warning,
}

#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaClass {
    Lookup,
    Insertion,
    Pagination,
    Sort,
    Allocation,
    Recursion,
    Hash,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone)]
pub struct DsaViolation {
    pub severity: DsaSeverity,
    pub class: DsaClass,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}
