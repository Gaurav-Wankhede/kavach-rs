#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AtomicSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AtomicViolation {
    pub severity: AtomicSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Level {
    Atom,
    Molecule,
    Organism,
    Template,
    Page,
    Unknown,
}
