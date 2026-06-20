use std::fmt::Write as _;

use kavach_types::HookInput;

use crate::error::EngineError;

/// True when the `agent_type` identifies a code-reviewer specialist.
/// Matches namespaced `subagent_type` strings like `pr-review-toolkit:code-reviewer`
/// (Claude Code plugin marketplace convention) as well as `code-reviewer`,
/// `pr-test-analyzer`, etc. Case-insensitive.
fn is_reviewer_agent(agent_type: &str) -> bool {
    let lower = agent_type.to_lowercase();
    lower.contains("reviewer") || lower.contains("code-review")
}

/// `SubagentStart` gate: track agent lifecycle, inject budget context.
#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch")]
pub(crate) fn run_start(input: &HookInput) -> Result<(), EngineError> {
    let agent_id = &input.agent_id;
    let agent_type = &input.agent_type;

    let mut session = kavach_session::get_or_create_session();
    session.track_subagent_start(agent_id);

    // Build budget context for the subagent
    let limits = kavach_config::load_output_limits();
    let effective_limit = session.get_effective_output_limit(
        agent_type,
        &limits
            .agent_limits
            .iter()
            .map(|(k, v)| (k.clone(), i32::try_from(*v).unwrap_or(i32::MAX)))
            .collect(),
    );

    let limit_str = effective_limit.to_string();
    let active_str = session.active_subagents.to_string();
    let phase = session.context_phase.clone();
    let mut context = kavach_hook::context_block(
        "SUBAGENT_START",
        &[
            ("id", agent_id),
            ("type", agent_type),
            ("limit", &limit_str),
            ("active", &active_str),
            ("phase", &phase),
        ],
    );

    let modules = kavach_config::load_modules(&["agents", "model-routing"]);
    if !modules.is_empty() {
        context.push_str("\n[MODULE:LAZY_LOADED]\n");
        context.push_str(&modules);
    }

    // P0 SECURITY: Propagate denied tools from parent context to subagent.
    // SOURCE: github.com/nousresearch/hermes-agent — gate inheritance pattern
    let denied_ctx = session.get_denied_tools_context();
    if !denied_ctx.is_empty() {
        context.push('\n');
        context.push_str(&denied_ctx);
    }

    // Inject blast radius warning if threshold exceeded.
    if session.is_blast_escalated() {
        let (files, apis, db) = session.get_blast_stats();
        // Tag + stat lines frozen; the WARNING imperative is research-refreshed.
        let warn = crate::gates::directive_cache::dyn_directive(
            "subagent.blast-escalated-warning",
            "WARNING: Cumulative subagent blast radius exceeded threshold. Gates escalated to P0.",
        );
        writeln!(
            context,
            "\n[BLAST_ESCALATED]\nfiles_written: {files}\nexternal_apis: {apis}\ndb_mutations: {db}\n{warn}"
        ).ok();
    }

    session.queue_lifecycle_relay(&context);
    // CC path: systemMessage. Cursor drops allow output — relay above.
    drop(kavach_hook::exit_subagent_start(&context));
    Ok(())
}

/// `SubagentStop` gate: record output size, update tracking.
#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch")]
pub(crate) fn run_stop(input: &HookInput) -> Result<(), EngineError> {
    let agent_id = &input.agent_id;
    let agent_type = &input.agent_type;

    let mut session = kavach_session::get_or_create_session();

    // Estimate output size from tool response
    let output_size = input
        .tool_response
        .as_ref()
        .map(|r| serde_json::to_string(r).unwrap_or_default().len())
        .map_or(0, |len| i32::try_from(len).unwrap_or(i32::MAX));

    session.track_subagent_stop(agent_id, output_size);

    // Bug B fix: stamp review-completion when a reviewer agent finishes.
    // Cures REVIEW_GATE over-firing — completion_guard reads this to skip
    // the warning when an existing review covers the current diff.
    // SOURCE: decision:rca.review_gate_overfires (2026-05-10)
    if is_reviewer_agent(agent_type) {
        session.mark_review_completed();
    }

    let size_str = output_size.to_string();
    let active_str = session.active_subagents.to_string();
    let context = kavach_hook::context_block(
        "SUBAGENT_STOP",
        &[
            ("id", agent_id),
            ("chars", &size_str),
            ("agents", &active_str),
        ],
    );

    session.queue_lifecycle_relay(&context);
    drop(kavach_hook::exit_subagent_stop(&context));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_start_default() {
        let input = HookInput {
            agent_id: "test-agent".into(),
            agent_type: "Explore".into(),
            ..Default::default()
        };
        // Should succeed
        run_start(&input).expect("test setup");
    }
}
