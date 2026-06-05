use kavach_types::HookInput;

/// Handle Skill tool pre-check: record invocation + inject context.
///
/// CRITICAL: Skill invocation is recorded HERE (`PreToolUse`) because
/// `PostToolUse` may not receive `tool_input` for the Skill tool.
/// Logs to events table for dynamic skill loadout scoring.
pub(crate) fn handle_skill(input: &HookInput) {
    let skill_name = input.get_string("skill");

    if !skill_name.is_empty() {
        let mut session = kavach_session::get_or_create_session();
        session.record_skill_invoked(skill_name);
        // Log for dynamic loadout scoring — creates session→uses_skill graph edge.
        super::event_log::log_skill_invoke(&session.session_id, skill_name, &session.project);
    }
    drop(kavach_hook::exit_pre_tool_allow(None));
}
