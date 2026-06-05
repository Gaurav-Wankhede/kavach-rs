//! Tests for `RuleIndex` lookups.

use kavach_rule_ast::{ResearchGate, SkillDefinition, SkillMetadata};
use kavach_rule_storage::{RuleIndex, StoredRule};
use std::collections::HashMap;
use std::path::PathBuf;

fn make_rule(name: &str, triggers: &[&str], protocol: &str) -> StoredRule {
    StoredRule::new(
        SkillDefinition {
            metadata: SkillMetadata {
                name: name.into(),
                description: String::new(),
                protocol: protocol.into(),
                triggers: triggers.iter().map(ToString::to_string).collect(),
                file_patterns: Vec::new(),
                priority: kavach_rule_ast::SkillPriority::default(),
            },
            research_gate: ResearchGate {
                mandatory: false,
                rule: String::new(),
            },
        },
        PathBuf::from("/tmp/fake.toon"),
        String::new(),
        String::new(),
        1,
    )
}

#[test]
fn lookup_by_trigger() {
    let mut idx = RuleIndex::new();
    let mut cache = HashMap::new();
    cache.insert(
        "rust-skill".into(),
        make_rule("rust-skill", &["rust", "cargo"], "SP/3.0"),
    );
    cache.insert("go-skill".into(), make_rule("go-skill", &["go"], "SP/3.0"));
    idx.rebuild(&cache);
    let found = idx.by_trigger("rust");
    assert_eq!(found.len(), 1);
    assert!(found.contains(&"rust-skill"));
}

#[test]
fn lookup_by_category() {
    let mut idx = RuleIndex::new();
    let mut cache = HashMap::new();
    cache.insert("s1".into(), make_rule("s1", &[], "SP/3.0"));
    cache.insert("s2".into(), make_rule("s2", &[], "SP/3.0"));
    idx.rebuild(&cache);
    let found = idx.by_category("sp");
    assert_eq!(found.len(), 2);
}

#[test]
fn trigger_lookup_is_case_insensitive() {
    let mut idx = RuleIndex::new();
    let mut cache = HashMap::new();
    cache.insert("r1".into(), make_rule("r1", &["Rust"], "SP/1.0"));
    idx.rebuild(&cache);
    assert_eq!(idx.by_trigger("rust").len(), 1);
    assert_eq!(idx.by_trigger("RUST").len(), 1);
}
