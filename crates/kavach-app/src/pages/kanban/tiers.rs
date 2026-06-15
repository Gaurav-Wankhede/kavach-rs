//! Topological tier layout for the desktop DAG view — the GUI mirror of the
//! CLI's `kavach db kanban` tiered text (TIER 0 = ready now, TIER N unlocks once
//! every tier `<N` prerequisite is done). The CLI computes tiers from a
//! prerequisites-first topo order in one pass (`dag_render.rs::tiers`); the app
//! has no guaranteed ordering over the loaded rows, so it runs a cycle-safe
//! iterative fixpoint instead — same RESULT (depth = max prereq depth + 1),
//! reached without a topo sort and without ever looping on a dependency cycle.

use std::collections::HashMap;

use crate::pages::kanban::deps::declared_deps;
use crate::state::EntryRef;

/// A node placed on the DAG: the card plus its computed tier and the prereq keys
/// that gate it (rendered as a `⤷ depends-on:` suffix, like the CLI).
pub struct TierNode {
    pub entry: EntryRef,
    pub deps: Vec<String>,
}

/// Assign every card a dependency tier and bucket the cards by it. Tier 0 = no
/// declared prerequisite present on the board; tier N = one past the deepest
/// prerequisite. Returns `(tiers, cyclic)` where `tiers[i]` is the nodes at depth
/// `i` (sorted by key for stable render) and `cyclic` holds any cards that never
/// converged — i.e. sit on a dependency cycle — so the caller renders them
/// separately rather than silently dropping or mis-placing them (fail-visible,
/// mirroring the CLI's "cycle rendered apart from the tiers" contract).
#[must_use]
pub fn layout(rows: &[EntryRef]) -> (Vec<Vec<TierNode>>, Vec<EntryRef>) {
    // Only deps whose key is actually on the board can gate a tier; an absent
    // key (cross-project prereq) does not deepen the tier (we cannot know its
    // depth). Filtering to on-board deps here also guarantees the fixpoint
    // terminates against a finite key set.
    let present: HashMap<&str, &EntryRef> = rows.iter().map(|r| (r.key.as_str(), r)).collect();
    let on_board_deps: HashMap<&str, Vec<&str>> = rows
        .iter()
        .map(|r| {
            let deps = declared_deps(&r.content)
                .into_iter()
                .filter(|d| present.contains_key(d))
                .collect();
            (r.key.as_str(), deps)
        })
        .collect();

    // Fixpoint: depth[node] = max(depth[prereq]) + 1, else 0. Iterate until no
    // depth changes. A node on a cycle can never settle (its depth keeps being
    // bumped by a prereq that itself depends on it), so it stays unresolved after
    // the bound — that is exactly the cycle signal. The bound is `len` passes: an
    // acyclic chain resolves one new node per pass at worst, so `len` always
    // suffices; anything still unresolved is provably cyclic.
    let mut depth: HashMap<&str, usize> = on_board_deps
        .iter()
        .filter(|(_, deps)| deps.is_empty())
        .map(|(k, _)| (*k, 0usize))
        .collect();
    for _ in 0..rows.len() {
        let mut changed = false;
        for (key, deps) in &on_board_deps {
            if deps.is_empty() {
                continue;
            }
            // Resolvable only when EVERY prereq already has a depth this pass.
            let Some(max_prereq) = deps.iter().map(|d| depth.get(d).copied()).max().flatten() else {
                continue;
            };
            if deps.iter().all(|d| depth.contains_key(d)) {
                let want = max_prereq.saturating_add(1);
                if depth.get(*key) != Some(&want) {
                    depth.insert(key, want);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    let max_tier = depth.values().copied().max().unwrap_or(0);
    let mut tiers: Vec<Vec<TierNode>> = (0..=max_tier).map(|_| Vec::new()).collect();
    let mut cyclic: Vec<EntryRef> = Vec::new();
    for row in rows {
        let key = row.key.as_str();
        let node_deps: Vec<String> = on_board_deps
            .get(key)
            .map(|ds| ds.iter().map(|s| (*s).to_owned()).collect())
            .unwrap_or_default();
        // `depth[key] <= max_tier` by construction, so the tier bucket always
        // exists; `get_mut` keeps it panic-free regardless (clippy::indexing).
        match depth.get(key).and_then(|&t| tiers.get_mut(t)) {
            Some(bucket) => bucket.push(TierNode { entry: row.clone(), deps: node_deps }),
            None => cyclic.push(row.clone()),
        }
    }
    for tier in &mut tiers {
        tier.sort_by(|a, b| a.entry.key.cmp(&b.entry.key));
    }
    cyclic.sort_by(|a, b| a.key.cmp(&b.key));
    (tiers, cyclic)
}

#[cfg(test)]
#[path = "tiers_test.rs"]
mod tests;
