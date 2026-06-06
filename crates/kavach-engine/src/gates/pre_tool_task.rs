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

    // Check subagent budget
    let limits = kavach_config::load_output_limits();
    let max_parallel_i32 = i32::try_from(limits.max_parallel).unwrap_or(i32::MAX);
    if session.active_subagents >= max_parallel_i32 {
        let context = format!(
            "subagent limit reached: {}/{} active",
            session.active_subagents, limits.max_parallel
        );
        drop(kavach_hook::exit_pre_tool_allow(Some(&context)));
        return Ok(());
    }

    // Warn (but allow) in critical context phase — agent teams are
    // a core Opus 4.6 feature and hard-blocking breaks team workflows.
    // The model itself manages context; kavach should advise, not block.
    if session.context_phase == "critical" {
        drop(kavach_hook::exit_pre_tool_allow(Some(
            "WARNING: context phase critical — compact before spawning more agents",
        )));
        return Ok(());
    }

    // Validate agent type if provided
    if !agent_type.is_empty() && !kavach_config::is_valid_agent(agent_type) {
        let context = format!("unrecognized agent type: {agent_type}");
        drop(kavach_hook::exit_pre_tool_allow(Some(&context)));
        return Ok(());
    }

    drop(kavach_hook::exit_pre_tool_allow(None));
    Ok(())
}
