//! Priority-ceiling: a dependency-blocker inherits the urgency of its most
//! urgent dependent (E3).
//!
//! PRIORITY SEMANTICS (`kavach-surreal::read`): priority is sorted ASC, LOWER =
//! MORE URGENT (pri-1 beats pri-9); absent priority sorts last (999999). So
//! "inherit `max(priority)` of dependents" in URGENCY terms is the NUMERIC MIN of
//! the dependents' priorities — the blocker rises to the rank of the hungriest
//! card waiting on it.
//!
//! THE INVERSION it fixes: a pri-1 (urgent) card `A` declares `DEPENDS_ON: B`,
//! where `B` is pri-9 (low). Raw ordering sorts B near the back, so dispatch
//! keeps skipping B for less-important pri-2..8 cards while A starves — A can
//! NEVER run until B does. Ceiling lifts B's EFFECTIVE priority to 1 (A's), so B
//! is dispatched next and A unblocks. Pure: computes a sort key, no I/O.

use super::super::readiness::parse_declared_deps;
use kavach_surreal::MemoryEntry;
use std::collections::HashMap;

/// Absent priority sorts last — mirrors `read.rs`'s `priority ?? 999999`.
const PRIORITY_NONE: i64 = 999_999;

/// Effective dispatch priority for each entry, keyed by `entry_key`: the entry's
/// own priority OR the most-urgent (numerically smallest) priority among the
/// cards that declare a `DEPENDS_ON` on it — whichever is more urgent.
///
/// Single forward pass over declared deps: for every `dependent -> blocker` edge,
/// the blocker's ceiling is pulled down to `min(blocker_eff, dependent_raw)`.
/// One level deep by design (the card's contract); a deeper transitive ceiling is
/// a separate unit if ever needed.
#[must_use]
pub(super) fn effective_priorities(entries: &[MemoryEntry]) -> HashMap<String, i64> {
    let raw = |e: &MemoryEntry| e.priority.unwrap_or(PRIORITY_NONE);
    let mut eff: HashMap<String, i64> = entries
        .iter()
        .map(|e| (e.entry_key.clone(), raw(e)))
        .collect();

    for dependent in entries {
        let dep_pri = raw(dependent);
        for blocker_key in parse_declared_deps(&dependent.content) {
            // A dependent lifts its blocker to at least the dependent's urgency.
            if let Some(slot) = eff.get_mut(&blocker_key) {
                *slot = (*slot).min(dep_pri);
            }
        }
    }
    eff
}

/// Re-sort `entries` in place by EFFECTIVE priority (ascending = most urgent
/// first), `created_at` as the stable tiebreak — matching the DB read's
/// `ORDER BY _sort_priority ASC, created_at ASC` but with ceilings applied.
pub(super) fn sort_by_effective_priority(entries: &mut [MemoryEntry]) {
    let eff = effective_priorities(entries);
    entries.sort_by(|a, b| {
        let pa = eff.get(&a.entry_key).copied().unwrap_or(PRIORITY_NONE);
        let pb = eff.get(&b.entry_key).copied().unwrap_or(PRIORITY_NONE);
        pa.cmp(&pb).then_with(|| a.created_at.cmp(&b.created_at))
    });
}

#[cfg(test)]
#[path = "priority_ceiling_test.rs"]
#[cfg(test)]
#[path = "priority_ceiling_test.rs"]
mod tests;