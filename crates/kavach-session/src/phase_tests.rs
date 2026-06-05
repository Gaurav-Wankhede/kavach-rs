use crate::state::SessionState;

#[test]
fn test_needs_reinforcement() {
    let mut s = SessionState::default();
    s.turn_count = 14;
    assert!(!s.needs_reinforcement());
    s.turn_count = 15;
    assert!(s.needs_reinforcement());
}

#[test]
fn test_update_context_phase() {
    let mut s = SessionState::default();
    s.token_budget_used = 10000;
    s.update_context_phase();
    assert_eq!(s.context_phase, "early");

    s.token_budget_used = 90000;
    s.update_context_phase();
    assert_eq!(s.context_phase, "mid");

    s.token_budget_used = 140_000;
    s.update_context_phase();
    assert_eq!(s.context_phase, "late");

    s.token_budget_used = 170_000;
    s.update_context_phase();
    assert_eq!(s.context_phase, "critical");
}

#[test]
fn test_reset_research_with_task() {
    let mut s = SessionState::default();
    s.research_done = true;
    s.required_skills = vec!["rust".into()];
    s.current_task = "active".into();
    s.reset_research_for_new_prompt();
    // Enforcement MUST reset even when task is active
    assert!(!s.research_done);
    assert!(s.required_skills.is_empty());
}

#[test]
fn test_reset_research_without_task() {
    let mut s = SessionState::default();
    s.research_done = true;
    s.reset_research_for_new_prompt();
    assert!(!s.research_done);
}
