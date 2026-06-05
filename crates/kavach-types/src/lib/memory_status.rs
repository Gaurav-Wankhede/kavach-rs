// ALGO: declarative_enum_iterate
// TIME: O(n) for all() where n=variants (constant 4); O(n) for allowed_list (constant 4)
// SPACE: O(n) output buffer; BENCHMARK: strum derive + IntoEnumIterator, std lib 2026

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString};

/// Typed lifecycle states for roadmap/decision entries.
/// SOURCE: https://docs.rs/strum/0.28 (EnumString + Display + EnumIter)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString,
    EnumIter, AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MemoryStatus {
    Todo,
    InProgress,
    Done,
    Verified,
}

impl MemoryStatus {
    /// Comma-separated list of all variants in canonical wire form.
    /// Used for error messages — replaces hand-rolled `VALID_STATUSES.join`(", ").
    #[must_use]
    pub fn allowed_list() -> String {
        use strum::IntoEnumIterator;
        Self::iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// All variants in canonical order. The single source of truth for any
    /// UI status picker — callers (e.g. the kavach-app editor dropdown) must
    /// iterate this, never a hand-rolled string list.
    /// BOUNDED: enum has exactly 4 variants (Todo, InProgress, Done, Verified).
    #[must_use]
    pub fn all() -> Vec<Self> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }
}
