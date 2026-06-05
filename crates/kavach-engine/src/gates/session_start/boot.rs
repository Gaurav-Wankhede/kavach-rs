//! Skill-registry build at session start.

/// Scan ~/.claude/skills/, build registry, cache if stale.
pub(super) fn build_skill_registry() {
    let skills_dir = kavach_config::skills_dir();
    let cache_path = kavach_config::registry_cache_path();

    let mut store = kavach_rule_storage::RuleStore::new(skills_dir);
    if store.load_all().is_err() {
        return;
    }

    let rules: Vec<_> = store
        .list()
        .iter()
        .filter_map(|name| store.get(name).cloned())
        .collect();

    let registry = kavach_rule_storage::build_from_rules(&rules);

    if kavach_rule_storage::is_stale(&cache_path, &registry.hash) {
        kavach_rule_storage::save_registry(&cache_path, &registry).ok();
    }
}
