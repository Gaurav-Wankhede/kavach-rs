//! Public types for the destructive-CLI guard: severity tier, category, and hit.
//! Split from the hub to keep both within the ≤100-LOC micro-file budget.

#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructiveSeverity {
    P0Block,
    P1Confirm,
    P2Warn,
}

impl DestructiveSeverity {
    /// Stable wire/display name. Single source of truth — callers must not
    /// re-derive this with their own `match` (that breaks on every new variant).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::P0Block => "P0Block",
            Self::P1Confirm => "P1Confirm",
            Self::P2Warn => "P2Warn",
        }
    }
}

#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructiveCategory {
    FilesystemNuke,
    PermissionsWipe,
    DiskOverwrite,
    ForkBomb,
    KernelModule,
    HistoryScrub,
    PipeToShell,
    PrivilegeEscalation,
    SystemHalt,
    DangerousFlag,
    HexObfuscation,
    /// "Safe-name" command weaponized via a code-exec/file-write flag
    /// (rg `--pre`, find `-exec`/`-delete`, git `--output`, go test `-exec`).
    CodeExecFlag,
}

impl DestructiveCategory {
    /// Stable wire/display name. Single source of truth — callers must not
    /// re-derive this with their own `match` (that breaks on every new variant).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FilesystemNuke => "FilesystemNuke",
            Self::PermissionsWipe => "PermissionsWipe",
            Self::DiskOverwrite => "DiskOverwrite",
            Self::ForkBomb => "ForkBomb",
            Self::KernelModule => "KernelModule",
            Self::HistoryScrub => "HistoryScrub",
            Self::PipeToShell => "PipeToShell",
            Self::PrivilegeEscalation => "PrivilegeEscalation",
            Self::SystemHalt => "SystemHalt",
            Self::DangerousFlag => "DangerousFlag",
            Self::HexObfuscation => "HexObfuscation",
            Self::CodeExecFlag => "CodeExecFlag",
        }
    }
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone)]
pub struct DestructiveHit {
    pub severity: DestructiveSeverity,
    pub category: DestructiveCategory,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub canonical: String,
}
