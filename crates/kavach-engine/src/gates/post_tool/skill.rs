//! Skill-invocation tracking for the enforcement gate (records the skill name
//! and surfaces it as post-tool context).
use kavach_types::HookInput;

use crate::error::EngineError;

/// Track skill invocation for enforcement gate.
#[expect(
    clippy::unnecessary_wraps,
    reason = "uniform gate dispatch via Result type"
)]
pub(super) fn handle_skill_done(
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Result<(), EngineError> {
    let skill_name = input.get_string("skill");
    if skill_name.is_empty() {
        drop(kavach_hook::exit_silent());
    } else {
        session.record_skill_invoked(skill_name);

        let context = kavach_hook::context_block("POST_TOOL:SKILL", &[("skill", skill_name)]);
        drop(kavach_hook::exit_post_tool_context(&context));
    }
    Ok(())
}
