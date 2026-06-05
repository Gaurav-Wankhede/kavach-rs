use std::path::PathBuf;

use kavach_rule_storage::RuleStore;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

fn skills_dir() -> PathBuf {
    kavach_config::skills_dir()
}

pub(super) fn run(name: &str) -> i32 {
    let dir = skills_dir();
    let mut store = RuleStore::new(dir);
    if let Err(e) = store.load_all() {
        let msg = format!("rules show: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let Some(rule) = store.get(name) else {
        let msg = format!("rules show: rule not found: {name}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        let available = store.list();
        if !available.is_empty() {
            let avail_msg = format!("available: {}", available.join(", "));
            if let Err(io_err) = ewrite_or_exit(&avail_msg) {
                return into_exit_code(io_err);
            }
        }
        return 1;
    };
    let lines: [String; 6] = [
        format!("Rule: {}", rule.definition.metadata.name),
        format!("  Description: {}", rule.definition.metadata.description),
        format!("  Protocol: {}", rule.definition.metadata.protocol),
        format!("  Version: {}", rule.version),
        format!("  Source: {}", rule.source_path.display()),
        format!("  Hash: {}", rule.content_hash),
    ];
    for l in &lines {
        if let Err(io_err) = print_or_exit(l) {
            return into_exit_code(io_err);
        }
    }
    let rg = &rule.definition.research_gate;
    if let Err(io_err) = print_or_exit("  Research Gate:") {
        return into_exit_code(io_err);
    }
    let mandatory_line = format!("    Mandatory: {}", rg.mandatory);
    if let Err(io_err) = print_or_exit(&mandatory_line) {
        return into_exit_code(io_err);
    }
    let rule_line = format!("    Rule: {}", rg.rule);
    if let Err(io_err) = print_or_exit(&rule_line) {
        return into_exit_code(io_err);
    }
    if !rule.definition.metadata.triggers.is_empty() {
        if let Err(io_err) = print_or_exit("  Triggers:") {
            return into_exit_code(io_err);
        }
        for t in &rule.definition.metadata.triggers {
            let line = format!("    - {t}");
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
    }
    0
}
