//! The `Tool` enum: the canonical Rust CLI binary set with POSIX fallbacks.
//!
//! Each variant maps to a preferred Rust binary (`program`) plus a legacy
//! fallback (`fallback`); `resolve` picks the available one via the cache.
use super::cache::which;

#[derive(Debug, Clone, Copy)]
#[expect(
    clippy::exhaustive_enums,
    reason = "constructed cross-crate; non_exhaustive => E0639"
)]
pub enum Tool {
    Rg,
    Fd,
    Bat,
    Sd,
    Difft,
    Jaq,
    Dust,
    Procs,
    Xh,
    Tokei,
    Erd,
    Gron,
    Dasel,
    Hyperfine,
    Sg,
}

impl Tool {
    #[must_use]
    pub const fn program(&self) -> &'static str {
        match self {
            Self::Rg => "rg",
            Self::Fd => "fd",
            Self::Bat => "bat",
            Self::Sd => "sd",
            Self::Difft => "difft",
            Self::Jaq => "jaq",
            Self::Dust => "dust",
            Self::Procs => "procs",
            Self::Xh => "xh",
            Self::Tokei => "tokei",
            Self::Erd => "erd",
            Self::Gron => "gron",
            Self::Dasel => "dasel",
            Self::Hyperfine => "hyperfine",
            Self::Sg => "sg",
        }
    }

    #[must_use]
    pub const fn fallback(&self) -> &'static str {
        match self {
            Self::Rg | Self::Sg => "grep",
            Self::Fd => "find",
            Self::Bat | Self::Gron | Self::Dasel => "cat",
            Self::Sd => "sed",
            Self::Difft => "diff",
            Self::Jaq => "jq",
            Self::Dust => "du",
            Self::Procs => "ps",
            Self::Xh => "curl",
            Self::Tokei => "wc",
            Self::Erd => "tree",
            Self::Hyperfine => "time",
        }
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        which(self.program())
    }

    #[must_use]
    pub fn resolve(&self) -> &'static str {
        if self.is_available() {
            self.program()
        } else {
            self.fallback()
        }
    }
}
