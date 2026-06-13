use kavach_session::SessionState;

use super::build_loop_compact;
use super::build_turn_shadow;

#[test]
fn turn_shadow_includes_intent_harness_and_cursor_tag() {
    let session = SessionState::default();
    let intent = kavach_chain::analyze_intent("implement the rust gate fix");
    let shadow = build_turn_shadow(&session, &intent, "loop-until-done", Some("/rust"));
    assert!(shadow.contains("[INTENT]"));
    assert!(shadow.contains("[HARNESS] loop-until-done"));
    assert!(shadow.contains("[RAG:skill] /rust"));
    assert!(shadow.contains("cursor:native"));
    assert!(shadow.len() <= super::TURN_SHADOW_CAP);
}

#[test]
fn loop_compact_is_single_block() {
    let mut session = SessionState::default();
    session.current_kanban_card = "unit.test".into();
    let block = build_loop_compact(&session, None);
    assert!(block.starts_with("[LOOP]"));
    assert!(block.contains("unit.test"));
}

#[test]
fn reward_session_stats_emits_when_data_present() {
    let mut session = SessionState::default();
    session.record_reward_outcome("unit.a", true);
    let stats = super::build_reward_session_stats(&session).expect("stats");
    assert!(stats.contains("[REWARD:stats]"));
    assert!(stats.contains("session_pass_rate"));
}
