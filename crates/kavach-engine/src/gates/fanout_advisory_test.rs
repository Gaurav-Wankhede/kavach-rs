use super::*;

fn session_with_model(model: &str) -> SessionState {
    let mut s = SessionState::default();
    s.model_id = model.to_owned();
    s
}

#[test]
fn frontier_model_doing_read_is_nudged() {
    let mut s = session_with_model("claude-opus-4-8");
    let out = nudge(&mut s, "Read").expect("frontier + labor tool must nudge");
    assert!(out.contains("[FANOUT_NUDGE]"));
    // Names the LIVE resolved cheap tier (env-or-fallback), never a hardcoded id.
    assert!(out.contains(&kavach_config::model::cheap_executor_tier()));
    assert!(s.fanout_nudge_sent, "flag must latch after firing");
}

#[test]
fn cheap_tier_is_never_nudged() {
    let mut s = session_with_model(&kavach_config::model::cheap_executor_tier());
    assert!(nudge(&mut s, "Edit").is_none(), "the executor IS the doer");
}

#[test]
fn non_labor_tool_is_not_nudged() {
    let mut s = session_with_model("claude-opus-4-8");
    assert!(nudge(&mut s, "Agent").is_none(), "spawning IS the fan-out");
    assert!(nudge(&mut s, "WebSearch").is_none());
}

#[test]
fn nudge_fires_only_once_per_turn() {
    let mut s = session_with_model("claude-sonnet-4-6");
    assert!(nudge(&mut s, "Bash").is_some());
    assert!(
        nudge(&mut s, "Write").is_none(),
        "second labor call stays silent"
    );
}
