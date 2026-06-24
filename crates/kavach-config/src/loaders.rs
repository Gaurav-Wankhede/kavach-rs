use std::collections::HashMap;

#[must_use]
pub fn get_skill_patterns() -> HashMap<String, Vec<String>> {
    crate::cache::load_patterns("skill-patterns.toon")
}

#[must_use]
pub fn get_agent_mappings() -> HashMap<String, Vec<String>> {
    crate::cache::load_patterns("agent-mappings.toon")
}

#[must_use]
pub fn get_router_mappings() -> HashMap<String, Vec<String>> {
    crate::cache::load_patterns("router-mappings.toon")
}

#[must_use]
pub fn get_framework_patterns() -> HashMap<String, Vec<String>> {
    crate::cache::load_patterns("frameworks.toon")
}
