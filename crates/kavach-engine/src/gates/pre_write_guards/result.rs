//! Guard outcome type + the shared accumulator the grouped guards write into.
//!
//! The dispatcher runs guard groups in a fixed order, short-circuiting on the
//! first block. Each group mutates `Acc` (advisory state) and returns
//! `Option<String>` — `Some(reason)` means "block now". Centralising the
//! accumulator here removes the 16 repeated `GuardResult { block: Some(..), .. }`
//! literals the flat dispatcher carried.

/// Result of the `PreWrite` guard chain. Consumed by `gates::pre_write`.
pub(crate) struct GuardResult {
    pub block: Option<String>,
    pub algo_advisory: Option<String>,
    pub runner_compact: String,
    /// P1 advisories (quality issues) — shown in context but don't block.
    pub p1_advisories: Vec<String>,
}

/// Mutable advisory state threaded through the guard groups.
#[derive(Default)]
pub(crate) struct Acc {
    pub algo_advisory: Option<String>,
    pub p1_advisories: Vec<String>,
}

impl Acc {
    /// Merge an auto-inject context block into `algo_advisory`, appending with a
    /// blank-line separator when a prior block already exists.
    pub(crate) fn merge_advisory(&mut self, inject: String) {
        self.algo_advisory = match self.algo_advisory.take() {
            Some(mut existing) => {
                existing.push_str("\n\n");
                existing.push_str(&inject);
                Some(existing)
            }
            None => Some(inject),
        };
    }
}
