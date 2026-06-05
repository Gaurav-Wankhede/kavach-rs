//! Chain verification (code files only) — runs the full `kavach_chain::Runner`
//! and returns a block reason when the chain state is blocked.
use crate::gates::pre_write_checks::extract_write_context;
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;

/// Run chain verification for code files. `Some(reason)` blocks the write.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &kavach_session::SessionState,
    runner: &mut kavach_chain::Runner,
) -> Option<String> {
    if !ctx.is_code {
        return None;
    }
    let tool_input = input.tool_input.clone().unwrap_or_default();
    let prompt = extract_write_context(input);
    let chain_state = runner.run_full(&prompt, ctx.tool_name, &tool_input, session.research_done);
    chain_state
        .is_blocked()
        .then(|| chain_state.get_block_reason())
}
