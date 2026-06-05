//! Session-state mutation at boot: model detection, stale-state reset, title.
use kavach_types::HookInput;

/// Detect model from hook input (CC 2.1 sends model field), else `CLAUDE_MODEL`.
pub(super) fn set_model(session: &mut kavach_session::SessionState, input: &HookInput) {
    let model = &input.model;
    if !model.is_empty() {
        session.set_model(model);
    } else if let Ok(env_model) = std::env::var("CLAUDE_MODEL")
        && !env_model.is_empty()
    {
        session.set_model(&env_model);
    }
}

/// Reset subagent/test/phase tracking that may be stale from a prior session.
pub(super) fn reset_stale_state(session: &mut kavach_session::SessionState) {
    // SubagentStop may never fire (crash, timeout, abrupt end), leaving
    // active_subagents > 0 and blocking the Stop gate.
    // See: github.com/anthropics/claude-code/issues/7881
    session.active_subagents = 0;
    session.subagent_outputs.clear();
    session.active_teammates = 0;

    // Background tasks that complete via notification bypass PostToolUse:Bash,
    // so clear_test_run never fires. Fresh session = no tests running.
    session.active_test_crates.clear();

    // ARCH: PhaseGatedSessionStart — initialize phase to PLAN if not set.
    // Per Stanford Meta-Harness: fresh sessions start in PLAN phase.
    if session.current_phase.is_empty() {
        session.current_phase = "PLAN".into();
        session.phase_start_turn = 0;
    }
    session.update_context_phase();
}

/// CC 2.1.152: name the session by project + dev phase for agent/bg views.
pub(super) fn session_title(session: &kavach_session::SessionState) -> String {
    if session.project.is_empty() {
        String::new()
    } else if session.current_phase.is_empty() {
        format!("kavach: {}", session.project)
    } else {
        format!("kavach: {} [{}]", session.project, session.current_phase)
    }
}
