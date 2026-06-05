// ALGO: IndexMap (hashbrown-backed map + insertion-order vector)
// PROBLEM_CLASS: ordered key-value lookup with first-match-wins iteration
// REJECTED: [
//   {"name":"std HashMap","reason":"random iteration order — first-match-wins becomes non-deterministic"},
//   {"name":"BTreeMap","reason":"sorts by key lexicographically — does not preserve config-file declaration order"},
//   {"name":"Vec<(String, String)>","reason":"O(n) lookup; loses hashed-key access for .get()"}
// ]
// TIME: O(1) lookup, O(n) iteration | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: ~5% memory overhead vs HashMap due to dual-storage; gains deterministic order
// BENCHMARK: https://docs.rs/indexmap/2.13/indexmap/#performance
// SOURCE: https://docs.rs/indexmap/2 (uses hashbrown internally; same lookup perf as std HashMap)
use crate::loaders::get_router_mappings;
use indexmap::IndexMap;

#[must_use]
pub fn get_intent_skill_mappings() -> IndexMap<String, String> {
    let data = get_router_mappings();
    let mut result = IndexMap::new();
    if let Some(lines) = data.get("SKILL:INTENT_MAPPINGS") {
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                result.insert(k.trim().to_owned(), v.trim().to_owned());
            }
        }
    }
    result
}

#[must_use]
pub fn get_complex_indicators() -> Vec<String> {
    get_router_mappings()
        .get("COMPLEX:INDICATORS")
        .cloned()
        .unwrap_or_default()
}

#[must_use]
pub fn get_skill_agent_defaults() -> IndexMap<String, String> {
    let data = get_router_mappings();
    let mut result = IndexMap::new();
    if let Some(lines) = data.get("SKILL:AGENT_DEFAULTS") {
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                result.insert(k.trim().to_owned(), v.trim().to_owned());
            }
        }
    }
    result
}

#[must_use]
pub fn get_skill_preferred_keywords() -> Vec<String> {
    get_router_mappings()
        .get("SKILL:PREFERRED_KEYWORDS")
        .cloned()
        .unwrap_or_default()
}
