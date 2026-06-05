use crate::state::SessionState;

#[test]
fn test_to_compact() {
    let s = SessionState::new("/tmp/test");
    let toon = s.to_compact();
    assert!(toon.contains("[SESSION]"));
    assert!(toon.contains("research: PENDING"));
    assert!(toon.contains("memory: PENDING"));
}

#[test]
fn test_to_ini_full_roundtrip() {
    let mut s = SessionState::new("/tmp/test");
    s.research_done = true;
    s.turn_count = 5;
    s.current_task = "test task".into();
    s.files_modified = vec!["a.rs".into(), "b.rs".into()];
    s.intent_type = "implement".into();
    s.token_budget_used = 50000;
    s.subagent_outputs.insert("agent-1".into(), 3000);

    let toon = s.to_ini_full();
    assert!(toon.contains("research_done: true"));
    assert!(toon.contains("turn_count: 5"));
    assert!(toon.contains("task: test task"));
    assert!(toon.contains("  - a.rs"));
    assert!(toon.contains("  - b.rs"));
    assert!(toon.contains("[INTENT_BRIDGE]"));
    assert!(toon.contains("type: implement"));
    assert!(toon.contains("token_budget_used: 50000"));
    assert!(toon.contains("output:agent-1: 3000"));
}
