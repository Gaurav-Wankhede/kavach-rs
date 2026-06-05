//! Graph-boost loading for skill ranking: per-skill outgoing-edge count via RPC,
//! clamped and weighted into the matcher's boost map.
use std::collections::HashMap;

use kavach_rag_core::RagTree;

use super::rpc_entity_find;

/// Load graph boosts for all skill entities via kavach-rpc.
/// Returns `None` if RPC unavailable or no skills have edges.
/// Boost weight = `min(edge_count, 3) * 30` — mirrors the prior storage impl.
// ALGO: PerSkillEdgeCountViaRPC
// PROBLEM_CLASS: graph_traversal
// REJECTED: [{"name":"DedicatedEdgeCountRPC","reason":"new method + handler — defer until profiling shows it matters"},{"name":"InProcEdgeCountCache","reason":"cache invalidation cost > win at n<=16 edges per skill"}]
// TIME: O(n*k) where n=skills (<50), k=edges/skill (capped at 16) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: pulls full edge rows when only count is needed; tolerable at n<=50 skills, hot path runs once per session
// BENCHMARK: https://surrealdb.com/blog/surrealdb-3-0-benchmarks-a-new-foundation-for-performance
pub(in crate::gates::rag_router) fn load_graph_boosts(
    trees: &[RagTree],
) -> Option<HashMap<String, u32>> {
    const WEIGHT_GRAPH_BOOST: u32 = 30;
    const MAX_EDGE_MULTIPLIER: u32 = 3;
    let mut skill_names: Vec<&str> = Vec::new();
    for tree in trees {
        collect_skill_names(&tree.root, &mut skill_names);
    }
    let _ = skill_names.first()?;
    let mut map: HashMap<String, u32> = HashMap::new();
    for name in skill_names {
        let Some(entity_id) = rpc_entity_find("skill", name) else {
            continue;
        };
        let edges = rpc_edge_count(&entity_id);
        let clamped = edges.min(MAX_EDGE_MULTIPLIER);
        let boost = clamped.saturating_mul(WEIGHT_GRAPH_BOOST);
        if boost > 0 {
            map.insert(name.to_owned(), boost);
        }
    }
    if map.is_empty() {
        return None;
    }
    Some(map)
}

/// Count outgoing edges from a skill entity via `graph.get_related`. Returns 0 on
/// any RPC failure — fail-closed for ranking (no boost rather than wrong boost).
// O(k) time | O(k) space | k = LIMIT (16)
fn rpc_edge_count(entity_id: &str) -> u32 {
    let params = serde_json::json!({"from": entity_id, "limit": 16});
    let result: Result<serde_json::Value, _> =
        kavach_rpc::client::call("graph.get_related", Some(params));
    let Ok(serde_json::Value::Array(arr)) = result else {
        return 0;
    };
    u32::try_from(arr.len()).unwrap_or(u32::MAX)
}

fn collect_skill_names<'a>(node: &'a kavach_rag_core::TreeNode, out: &mut Vec<&'a str>) {
    let stripped = node.id.trim_end_matches("/SKILL.md");
    let Some(name) = stripped.rsplit('/').next() else {
        return;
    };
    if !name.is_empty() {
        out.push(name);
    }
    for child in &node.children {
        collect_skill_names(child, out);
    }
}
