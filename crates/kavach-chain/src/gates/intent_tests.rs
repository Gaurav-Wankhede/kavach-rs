use super::*;

#[test]
fn test_analyze_intent_implement() {
    let a = analyze_intent("implement user profile page");
    assert_eq!(a.intent_type, "implement");
    assert!(a.requires_research);
    assert_eq!(a.complexity, "moderate");
}

#[test]
fn test_analyze_intent_debug() {
    let a = analyze_intent("fix the login bug");
    assert_eq!(a.intent_type, "debug");
    let skill_exists = kavach_config::paths::skills_dir()
        .join("debug-like-expert")
        .join("SKILL.md")
        .exists();
    assert_eq!(
        a.required_skills.contains(&"debug-like-expert".to_owned()),
        skill_exists
    );
}

#[test]
fn test_analyze_intent_deploy() {
    let a = analyze_intent("deploy to production");
    assert_eq!(a.intent_type, "deploy");
    assert_eq!(a.risk_level, "high");
    assert!(a.requires_research);
}

#[test]
fn test_analyze_intent_deletion_critical() {
    let a = analyze_intent("delete the database");
    assert_eq!(a.risk_level, "critical");
    assert!(
        a.confidence >= 0.7,
        "deletion should set confidence >= 0.7 to avoid false blocks"
    );
}

#[test]
fn test_analyze_intent_debug_requires_research() {
    let a = analyze_intent("fix the login bug");
    assert!(
        a.requires_research,
        "debug intent must require research (tabula rasa)"
    );
}

#[test]
fn test_analyze_intent_general_requires_research() {
    let a = analyze_intent("what is the status of this project");
    assert_eq!(a.intent_type, "general");
    assert!(
        a.requires_research,
        "general intent must require research by default (tabula rasa)"
    );
}

#[test]
fn test_analyze_intent_memory_no_research() {
    let a = analyze_intent("save to memory this finding");
    assert_eq!(a.intent_type, "memory");
    assert!(
        !a.requires_research,
        "memory intent should not require research"
    );
}

#[test]
fn test_intent_gate_does_not_block_on_critical_risk() {
    let mut state = ChainState::new("test");
    run_gate(&mut state, "remove old dropdown styles");
    assert!(!state.is_blocked(), "intent gate should warn, not block");
}
