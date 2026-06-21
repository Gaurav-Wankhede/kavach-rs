// IndexMap: O(1) lookup + preserves config declaration order (HashMap random,
// BTreeMap lexical, Vec O(n)). See decision.config.router-indexmap.
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
