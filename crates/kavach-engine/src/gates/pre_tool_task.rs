use kavach_types::HookInput;

use crate::error::EngineError;

/// Handle Task tool pre-check: subagent budget + context phase + agent type.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature required by calling convention; kavach gate handlers return Result<(), EngineError>"
)]
pub(crate) fn handle_task(input: &HookInput) -> Result<(), EngineError> {
    let agent_type = input.get_string("subagent_type");
    let session = kavach_session::get_or_create_session();

    // BrainOS injection rides every allow exit so a Task spawn is never brain-blind.
    let injection =
        super::pre_tool_agent::spawn_injection(input.get_string("description"), agent_type);

    // Check subagent budget
    let limits = kavach_config::load_output_limits();
    let max_parallel_i32 = i32::try_from(limits.max_parallel).unwrap_or(i32::MAX);
    if session.active_subagents >= max_parallel_i32 {
        let context = format!(
            "subagent limit reached: {}/{} active",
            session.active_subagents, limits.max_parallel
        );
        drop(kavach_hook::exit_pre_tool_allow(Some(&merge(&context, injection.as_deref()))));
        return Ok(());
    }

    // Critical context phase: advise compaction, never hard-block agent teams.
    if session.context_phase == "critical" {
        drop(kavach_hook::exit_pre_tool_allow(Some(&merge(
            "WARNING: context phase critical — compact before spawning more agents",
            injection.as_deref(),
        ))));
        return Ok(());
    }

    // Validate agent type if provided
    if !agent_type.is_empty() && !kavach_config::is_valid_agent(agent_type) {
        let context = format!("unrecognized agent type: {agent_type}");
        drop(kavach_hook::exit_pre_tool_allow(Some(&merge(&context, injection.as_deref()))));
        return Ok(());
    }

    drop(kavach_hook::exit_pre_tool_allow(injection.as_deref()));
    Ok(())
}

/// Append the `BrainOS` injection to a gate warning so neither is dropped.
fn merge(warning: &str, injection: Option<&str>) -> String {
    injection.map_or_else(|| warning.to_owned(), |i| format!("{warning}\n\n{i}"))
}
