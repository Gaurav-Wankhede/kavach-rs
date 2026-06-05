//! Graph-neighbor expansion: append each skill's `cross_invoke` neighbors after
//! the direct hits, deduped. Degrades silently if kavach-db is unavailable.
use std::collections::HashSet;

use super::super::rpc::{rpc_entity_find, rpc_get_related_cross_invoke};

/// Expand `skills` with their `cross_invoke` graph neighbors (deduped, appended).
pub(super) fn expand_with_graph_neighbors(skills: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = skills.iter().cloned().collect();
    let mut expanded = skills;
    for skill in expanded.clone() {
        let Some(entity_id) = rpc_entity_find("skill", &skill) else {
            continue;
        };
        for neighbor_name in rpc_get_related_cross_invoke(&entity_id) {
            if seen.insert(neighbor_name.clone()) {
                expanded.push(neighbor_name);
            }
        }
    }
    expanded
}
