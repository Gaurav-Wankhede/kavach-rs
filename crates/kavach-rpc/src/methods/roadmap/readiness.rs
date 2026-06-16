pub mod cycle;
pub mod dep_key;
pub mod status_check;

pub use cycle::{dep_index, is_in_cycle};
pub use dep_key::{dep_key_satisfied, parse_declared_deps};
pub use status_check::{is_parked, is_runnable_status, is_umbrella};

/// Check if an entry is dispatchable to an agent.
///
/// True iff every declared `DEPENDS_ON` prerequisite resolves to a
/// verified/done row. A card with no declared deps is trivially ready.
/// `dep_pool` is the row set dependency KEYS resolve against — distinct from
/// the project-scoped candidate list. Dependency keys are a GLOBAL key space:
/// a card may legitimately declare a prerequisite owned by another project.
///
/// Pure topological ordering: a card whose prerequisite is not yet done simply
/// waits its turn. There is no owner-gate / block path — a card that cannot be
/// built is deleted, never flagged (owner directive 2026-06-16).
#[must_use]
pub fn deps_satisfied(
    entry: &kavach_surreal::MemoryEntry,
    dep_pool: &[kavach_surreal::MemoryEntry],
) -> bool {
    parse_declared_deps(&entry.content)
        .iter()
        .all(|dep| dep_key_satisfied(dep, dep_pool))
}
