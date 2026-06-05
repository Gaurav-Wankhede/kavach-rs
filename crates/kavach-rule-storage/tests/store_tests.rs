//! Integration tests for `RuleStore` round-trip (save/load/remove).

use kavach_rule_ast::{ResearchGate, SkillDefinition, SkillMetadata};
use kavach_rule_storage::{RuleStore, StoredRule};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

#[expect(
    clippy::expect_used,
    reason = "test setup must verify directory creation"
)]
fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("kavach-rule-test-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sample_rule(name: &str) -> StoredRule {
    StoredRule::new(
        SkillDefinition {
            metadata: SkillMetadata {
                name: name.into(),
                description: "Test rule".into(),
                protocol: "SP/3.0".into(),
                triggers: vec!["rust".into(), "cargo".into()],
                file_patterns: Vec::new(),
                priority: kavach_rule_ast::SkillPriority::default(),
            },
            research_gate: ResearchGate {
                mandatory: true,
                rule: "WebSearch first".into(),
            },
        },
        PathBuf::new(),
        String::new(),
        String::new(),
        1,
    )
}

#[test]
fn save_and_list() {
    let dir = temp_dir();
    let mut store = RuleStore::new(dir.clone());
    store.save(&sample_rule("test-rule")).expect("save rule");
    let names = store.list();
    assert!(names.contains(&"test-rule"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn save_load_roundtrip() {
    let dir = temp_dir();
    let mut store = RuleStore::new(dir.clone());
    store.save(&sample_rule("roundtrip-skill")).expect("save");
    let mut store2 = RuleStore::new(dir.clone());
    store2.load_all().expect("load_all");
    let loaded = store2.get("roundtrip-skill");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().definition.metadata.description, "Test rule");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn remove_deletes_file() {
    let dir = temp_dir();
    let mut store = RuleStore::new(dir.clone());
    store.save(&sample_rule("to-delete")).expect("save");
    store.remove("to-delete").expect("remove");
    assert!(store.get("to-delete").is_none());
    assert!(!dir.join("to-delete.toon").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn get_missing_returns_none() {
    let dir = temp_dir();
    let store = RuleStore::new(dir.clone());
    assert!(store.get("nonexistent").is_none());
    std::fs::remove_dir_all(&dir).ok();
}
