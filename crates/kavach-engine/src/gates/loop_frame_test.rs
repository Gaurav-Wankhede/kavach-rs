use kavach_session::{RewardOutcome, SessionState};

use super::build_loop_compact;
use super::build_loop_stop;
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
fn loop_stop_frame_is_legible_goal_iteration_termination() {
    // F3 (unit.loop-eng-injection.f3-loop-goal-legible): the stop frame that
    // prepends every [AUTO_CONTINUE] must be LEGIBLE — name the goal, the
    // iteration, and the termination PREDICATE — not a bare "do not stop". This
    // frame is now prepended on ALL three dispatch paths (task/hunt/backlog).
    let mut session = SessionState::default();
    session.turn_count = 4;
    let frame = build_loop_stop(&session, Some("unit.demo-card"));
    assert!(
        frame.starts_with("[LOOP]"),
        "frame must lead with the [LOOP] tag"
    );
    assert!(frame.contains("goal: unit.demo-card"), "goal must be named");
    assert!(frame.contains("iteration:"), "iteration must be present");
    assert!(
        frame.contains("terminate ONLY on"),
        "the termination predicate is the whole point — it replaces bare 'do not stop'"
    );
    assert!(
        frame.contains("3-witness"),
        "termination predicate must state the 3-witness bar, not just 'keep going'"
    );
    assert!(
        frame.contains("on done:"),
        "the frame must tell the loop what to do on completion (close + dispatch next)"
    );
}

#[test]
fn loop_stop_frame_commands_fan_out_not_inline_labor() {
    // The dispatched card's read/verify/edit labor must FAN OUT to the cheap tier
    // (a worker Agent or /workflow), not be done inline by the frontier orchestrator.
    let session = SessionState::default();
    let frame = build_loop_stop(&session, Some("unit.demo-card"));
    assert!(
        frame.to_lowercase().contains("fan out") || frame.contains("FAN OUT"),
        "the dispatch frame must command fan-out: {frame}"
    );
    assert!(
        frame.contains("Agent") || frame.contains("/workflow") || frame.contains("workflow"),
        "fan-out must name the mechanism (Agent subagent or /workflow): {frame}"
    );
}

#[test]
fn reward_session_stats_emits_when_data_present() {
    let mut session = SessionState::default();
    session.record_reward_outcome("unit.a", RewardOutcome::Passed);
    let stats = super::build_reward_session_stats(&session).expect("stats");
    assert!(stats.contains("[REWARD:stats]"));
    assert!(stats.contains("session_pass_rate"));
}
