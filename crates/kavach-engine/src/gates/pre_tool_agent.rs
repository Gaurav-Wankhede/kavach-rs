//! PreToolUse:Agent — validate the agent contract, inject guardrails, allow.
//!
//! `contract` holds the per-agent security registry; `context` builds the
//! injected `[AGENT_*]` lines. SOURCE: code.claude.com/docs/en/agent-sdk/hooks.
mod context;
mod contract;

#[cfg(test)]
mod tests;

pub(crate) use contract::get_contract;

use kavach_types::HookInput;

/// PreToolUse:Agent handler. Exit 0 allows (with optional context); a research
/// nudge is advisory-only. `Ok(())` always (uniform gate dispatch).
#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch signature")]
pub(crate) fn handle_agent(input: &HookInput) -> Result<(), crate::error::EngineError> {
    let session = kavach_session::get_or_create_session();
    let cfg = kavach_config::load_gates_config();
    let agent_type_input = input.get_string("subagent_type");
    let agent_type = if agent_type_input.is_empty() {
        "general-purpose"
    } else {
        agent_type_input
    };

    // 1. Research nudge for non-local-analysis intents.
    let intent_is_local_analysis = matches!(
        session.intent_type.as_str(),
        "audit" | "analyze" | "explain" | "read" | "review" | "explore"
    );
    if cfg.research.enabled
        && cfg.research.require_before_code
        && !session.research_done
        && session.intent_risk != "low"
        && !intent_is_local_analysis
    {
        let topic = if session.research_topic.is_empty() {
            "relevant topic"
        } else {
            &session.research_topic
        };
        drop(kavach_hook::exit_pre_tool_allow(Some(&format!(
            "[ADVISORY:research-agent-spawn] WebSearch recommended before \
             spawning agents. Topic: {topic}."
        ))));
        return Ok(());
    }

    // 2. Look up contract, persist session (spawn tracking is in subagent.rs).
    let contract = get_contract(agent_type);
    session.save().ok();

    // 3. Build + emit context injection.
    let ctx = context::build_agent_context(input.get_string("description"), contract);
    let joined = ctx.join("\n");
    drop(kavach_hook::exit_pre_tool_allow(if ctx.is_empty() {
        None
    } else {
        Some(&joined)
    }));
    Ok(())
}
