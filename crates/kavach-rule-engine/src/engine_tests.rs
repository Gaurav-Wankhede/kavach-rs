//! Tests for the rule engine.

use super::*;
use std::path::PathBuf;

#[test]
fn test_new_engine() {
    let engine = RuleEngine::new(PathBuf::from("/nonexistent/skills"));
    assert!(engine.skills.is_empty());
}

#[test]
fn test_load_skills_missing_dir() {
    let mut engine = RuleEngine::new(PathBuf::from("/nonexistent/skills"));
    engine.load_skills();
    assert!(engine.skills.is_empty());
}

#[test]
fn test_evaluate_empty_engine() {
    let engine = RuleEngine::new(PathBuf::from("/tmp"));
    let ctx = EvalContext::new("Read", "show file").with_research(true);
    let results = engine.evaluate(&ctx);
    assert!(results.iter().all(|r| r.action != RuleAction::Block));
}

#[test]
fn test_worst_action_empty() {
    assert_eq!(RuleEngine::worst_action(&[]), RuleAction::Allow);
}

#[test]
fn test_worst_action_mixed() {
    let results = vec![
        RuleResult::allow("a", "ok"),
        RuleResult::warn("b", "caution", 3),
        RuleResult::block("c", "stop", 9),
    ];
    assert_eq!(RuleEngine::worst_action(&results), RuleAction::Block);
}

#[test]
fn test_worst_action_warns_only() {
    let results = vec![
        RuleResult::warn("a", "caution", 3),
        RuleResult::allow("b", "ok"),
    ];
    assert_eq!(RuleEngine::worst_action(&results), RuleAction::Warn);
}
