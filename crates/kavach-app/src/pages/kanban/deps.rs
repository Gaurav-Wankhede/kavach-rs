//! Dependency awareness for the kanban board — the GUI mirror of the CLI's
//! always-on DAG view. A card declares prerequisites via `DEPENDS_ON:` /
//! `BLOCKED_BY:` lines in its content (the same convention the scheduler and
//! `kavach db kanban` read); a card is BLOCKED when any declared prerequisite is
//! not yet `verified`/`done`. The board surfaces that as a badge so the desktop
//! view has the same readiness awareness as the CLI, without a separate graph.
use std::collections::HashMap;

use kavach_types::MemoryStatus;

use crate::state::EntryRef;

/// Parse `DEPENDS_ON:`/`BLOCKED_BY:` keys from a card's content. Mirrors the
/// scheduler's `parse_declared_deps` (kavach-rpc) — kept inline so the WASM-
/// targetable app pulls in no extra crate. Tolerant: no such line -> empty.
fn declared_deps(content: &str) -> Vec<&str> {
    let mut deps = Vec::new();
    let mut in_block = false;
    for raw in content.lines() {
        let line = raw.trim();
        if let Some(rest) = line
            .strip_prefix("BLOCKED_BY:")
            .or_else(|| line.strip_prefix("DEPENDS_ON:"))
        {
            in_block = true;
            deps.extend(rest.split([',', ' ', '\t']).map(str::trim).filter(|k| !k.is_empty()));
            continue;
        }
        if in_block {
            if let Some(bullet) = line.strip_prefix("- ") {
                if let Some(key) = bullet.split_whitespace().next() {
                    deps.push(key);
                }
                continue;
            }
            if !line.is_empty() {
                in_block = false;
            }
        }
    }
    deps
}

/// True when `entry` has at least one declared prerequisite that is NOT yet
/// satisfied (i.e. some dep key is present in the project and not done/verified).
/// A dep whose key is absent from the project is ignored (cannot block), matching
/// the scheduler's tolerance. `by_key` maps every card's key to its status.
#[must_use]
pub(crate) fn is_blocked(entry: &EntryRef, by_key: &HashMap<&str, MemoryStatus>) -> bool {
    declared_deps(&entry.content).iter().any(|dep| {
        by_key
            .get(*dep)
            .is_some_and(|st| !matches!(st, MemoryStatus::Verified | MemoryStatus::Done))
    })
}

/// Build the key -> status index over all rows, for `is_blocked` lookups.
#[must_use]
pub(crate) fn status_index(rows: &[EntryRef]) -> HashMap<&str, MemoryStatus> {
    rows.iter().map(|r| (r.key.as_str(), r.status)).collect()
}

#[cfg(test)]
#[path = "deps_test.rs"]
mod tests;
