// ARCH: see kavach db get --category decision --key arch.decision.silent_io_guard_shipped
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Show skill registry entries with enforcement tiers and file patterns.
pub(super) fn run() -> i32 {
    let cache_path = kavach_config::registry_cache_path();
    let registry = match kavach_rule_storage::load_registry(&cache_path) {
        Ok(r) => r,
        Err(_) => {
            if let Some(r) = build_registry(&cache_path) {
                r
            } else {
                if let Err(io_err) = print_or_exit("No skill registry found and rebuild failed.") {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        }
    };

    if registry.skills.is_empty() {
        if let Err(io_err) =
            print_or_exit("Skill registry is empty (no skills with file_patterns).")
        {
            return into_exit_code(io_err);
        }
        return 0;
    }

    let header = format!(
        "Skill Registry ({} enforced skill(s)):",
        registry.skills.len()
    );
    if let Err(io_err) = print_or_exit(&header) {
        return into_exit_code(io_err);
    }
    let built_line = format!("Built: {}", registry.built_at);
    if let Err(io_err) = print_or_exit(&built_line) {
        return into_exit_code(io_err);
    }
    if let Err(io_err) = print_or_exit("") {
        return into_exit_code(io_err);
    }

    for entry in &registry.skills {
        let tier = if entry.priority.is_critical() {
            "CRITICAL"
        } else {
            "advisory"
        };
        let entry_line = format!("  [{tier}] {}", entry.name);
        if let Err(io_err) = print_or_exit(&entry_line) {
            return into_exit_code(io_err);
        }
        for pat in &entry.file_patterns {
            let pat_line = format!("    pattern: {pat}");
            if let Err(io_err) = print_or_exit(&pat_line) {
                return into_exit_code(io_err);
            }
        }
    }
    0
}

/// Builder returns None on any error; caller (`run`) maps None→exit 1.
/// All stderr writes use `.ok()` (rust-lang.org documented explicit discard)
/// because this builder's caller already has a typed-error path on None.
fn build_registry(cache_path: &std::path::Path) -> Option<kavach_rule_storage::SkillRegistry> {
    let skills_dir = kavach_config::skills_dir();
    let mut store = kavach_rule_storage::RuleStore::new(skills_dir);
    if let Err(e) = store.load_all() {
        let msg = format!("rules list: failed to load skills: {e}");
        ewrite_or_exit(&msg).ok();
        return None;
    }
    let rules: Vec<_> = store
        .list()
        .iter()
        .filter_map(|name| store.get(name).cloned())
        .collect();
    let registry = kavach_rule_storage::build_from_rules(&rules);
    if let Err(e) = kavach_rule_storage::save_registry(cache_path, &registry) {
        let msg = format!("rules list: failed to save registry: {e}");
        ewrite_or_exit(&msg).ok();
    }
    Some(registry)
}
