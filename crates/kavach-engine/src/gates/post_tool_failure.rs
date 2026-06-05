//! `PostToolUseFailure` gate: two-tier self-evolving error handler.
//!
//! Tier 1 (autonomous): known pattern found in `gate_patterns` — inject cached
//!   `fix_strategy` + `imperative_rewrite` directly. Zero research needed.
//! Tier 2 (research): novel error — classify, inject `[SELF_EVOLVE]` advisory,
//!   seed the pattern store so occurrence count accumulates toward promotion.
mod classify;
mod rpc;

#[cfg(test)]
mod tests;

use std::fmt::Write as _;

use kavach_types::HookInput;

use classify::{action_for_type, classify_failure};
use rpc::{find_autonomous_via_rpc, self_evolve_block, upsert_via_rpc};

use crate::error::EngineError;

#[expect(
    clippy::unnecessary_wraps,
    reason = "uniform gate dispatch via gate_runner.rs"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let tool_name = &input.tool_name;
    let error_text = if input.error.is_empty() {
        &input.reason
    } else {
        &input.error
    };

    let failure_type = classify_failure(error_text);
    let mut session = kavach_session::get_or_create_session();
    session.increment_turn();
    session.record_failure_typed(tool_name, failure_type);
    let turn_str = session.turn_count.to_string();
    let retryable = if failure_type == "transient" {
        "true"
    } else {
        "false"
    };

    let context =
        if let Some(pat) = find_autonomous_via_rpc(&session.project, error_text, tool_name) {
            // Tier 1: inject stored fix + imperative rewrite. Bump occurrence count.
            upsert_via_rpc(&session.project, error_text, &pat, tool_name);
            let mut ctx = kavach_hook::context_block(
                "TOOL_FAILURE",
                &[
                    ("tool", tool_name),
                    ("t", &turn_str),
                    ("err", failure_type),
                    ("retry", retryable),
                    ("tier", "autonomous"),
                    ("fix", &pat.fix_strategy),
                    ("action", &pat.imperative_rewrite),
                    ("n", &pat.occurrence_count.to_string()),
                ],
            );
            ctx.push('\n');
            writeln!(ctx, "[DSA_RATIONALE]\n{}", pat.dsa_rationale).ok();
            ctx
        } else {
            // Tier 2: novel error — seed pattern store, inject SELF_EVOLVE block.
            let action = action_for_type(failure_type);
            super::event_log::log_tool_failure(&super::event_log::ToolFailureLog {
                session_id: &session.session_id,
                tool_name,
                error: error_text,
                fix_strategy: action,
                imperative_rewrite: "",
                dsa_rationale: "",
                gate_name: "post_tool_failure",
                project_slug: &session.project,
            });
            let mut ctx = kavach_hook::context_block(
                "TOOL_FAILURE",
                &[
                    ("tool", tool_name),
                    ("t", &turn_str),
                    ("err", failure_type),
                    ("retry", retryable),
                    ("tier", "research"),
                    ("action", action),
                ],
            );
            ctx.push('\n');
            ctx.push_str(&self_evolve_block(error_text, tool_name, failure_type));
            ctx
        };

    drop(kavach_hook::exit_post_tool_failure_context(&context));
    Ok(())
}
