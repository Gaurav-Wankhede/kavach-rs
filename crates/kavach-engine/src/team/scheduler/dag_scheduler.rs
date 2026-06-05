//! `DagScheduler`: ready-set → free-slot → claim → spawn, looped on completion.
//!
//! CAPACITY: <= 2000 roadmap rows/project (`roadmap_dag.rs` CAPACITY note).
//! CONCURRENCY: `cap` = min(16, cores-2); `free` = `cap` - `active_teammates`.
//! CONSISTENCY: single-writer CAS claim (`UNIQUE(project,entry_key)`) makes
//!   over-spawn impossible even when two ticks race.
//! `FAILURE_MODE`: cycle in DAG -> `TeamDispatchError::Cycle` (fail closed, names keys).
mod critical_path;
mod exec;

use super::types::SpawnerKind;

/// cap = min(16, cores-2), matching the Workflow tool's concurrency cap.
#[must_use]
pub(crate) fn default_cap() -> usize {
    let cores = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    cores.saturating_sub(2).clamp(1, 16)
}

/// DAG-aware parallel dispatcher over a project's roadmap.
#[derive(Debug)]
#[non_exhaustive]
pub struct DagScheduler {
    /// Hard concurrency ceiling: min(16, cores-2).
    cap: usize,
    /// Which spawn backend [`Self::dispatch`] uses. Private so all fields are
    /// private (the construction-time clamp on `cap` is the invariant);
    /// expose via [`Self::spawner_kind`].
    spawner_kind: SpawnerKind,
}

impl Default for DagScheduler {
    fn default() -> Self {
        Self {
            cap: default_cap(),
            spawner_kind: SpawnerKind::CcTeams,
        }
    }
}

impl DagScheduler {
    /// Construct with an explicit cap (e.g. CLI `--max-parallel`).
    #[must_use]
    pub fn with_cap(cap: usize, spawner_kind: SpawnerKind) -> Self {
        Self {
            cap: cap.clamp(1, 16),
            spawner_kind,
        }
    }

    /// CLI constructor: `Some(cap)` clamps to [1,16]; `None` uses the default
    /// cap min(16, cores-2). Avoids a cross-crate struct literal (the type is
    /// `#[non_exhaustive]`).
    #[must_use]
    pub fn for_cli(max_parallel: Option<usize>, spawner_kind: SpawnerKind) -> Self {
        let cap = max_parallel.map_or_else(default_cap, |c| c.clamp(1, 16));
        Self { cap, spawner_kind }
    }

    /// The clamped concurrency ceiling [1,16]. Read-only: mutation would bypass
    /// the construction-time clamp, so callers get a copy via this getter.
    #[must_use]
    pub const fn cap(&self) -> usize {
        self.cap
    }

    /// Which spawn backend [`Self::dispatch`] uses.
    #[must_use]
    pub const fn spawner_kind(&self) -> SpawnerKind {
        self.spawner_kind
    }
}
