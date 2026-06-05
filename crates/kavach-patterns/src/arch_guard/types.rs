//! Architecture guard types.

/// Scope categories for architectural patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate exhaustive match in kavach-engine; non_exhaustive => E0004"
)]
pub enum ArchScope {
    Scale,
    Cache,
    Messaging,
    Data,
    Service,
}

impl ArchScope {
    /// Returns the scope name as a string slice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scale => "scale",
            Self::Cache => "cache",
            Self::Messaging => "messaging",
            Self::Data => "data",
            Self::Service => "service",
        }
    }
}

/// A detected architectural pattern in code.
#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate usage only in pattern matching; non_exhaustive => E0639"
)]
pub struct ArchFinding {
    pub keyword: String,
    pub scope: ArchScope,
    pub line: usize,
}

/// Gate decision outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched in kavach-engine pre_write_arch_guard.rs; non_exhaustive => E0004"
)]
pub enum ArchGuardOutcome {
    /// No arch patterns detected.
    Allow,
    /// Valid // ARCH: comment present.
    AllowWithComment,
    /// Arch patterns detected, no comment — block.
    Block(String),
}
