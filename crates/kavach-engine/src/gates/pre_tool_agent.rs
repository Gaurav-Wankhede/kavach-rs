//! PreToolUse:Agent — validate the agent contract, inject guardrails, allow.
//!
//! `contract` holds the per-agent security registry; `context` builds the
//! injected `[AGENT_*]` lines. SOURCE: code.claude.com/docs/en/agent-sdk/hooks.
mod context;
mod contract;
#[cfg(test)]
#[path = "pre_tool_agent_test.rs"]
#[cfg(test)]
#[path = "pre_tool_agent_test.rs"]
mod tests;
pub(crate) use contract::get_contract;

use kavach_hook::Vendor;
use kavach_types::HookInput;

/// Smallest doer model per harness — orchestrate strong, execute cheap.
const fn smallest_doer_model(vendor: Vendor) -> &'static str {
    match vendor {
        Vendor::Cursor => "composer-2.5",
        _ => "haiku",
    }
}

/// Build the full `BrainOS` spawn-injection block for a subagent spawn — shared by
/// the `Agent` and `Task` tool paths. `None` only when nothing was injected.
pub(crate) fn spawn_injection(description: &str, agent_type: &str) -> Option<String> {
    let session = kavach_session::get_or_create_session();
    let brain = context::BrainContext {
        project: &session.project,
        phase: session.current_phase.as_str(),
        doer_model: smallest_doer_model(kavach_hook::output_vendor()),
    };
    let lines = context::build_agent_context(description, get_contract(agent_type), &brain);
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

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

    // 2. Persist session (spawn tracking is in subagent.rs).
    session.save().ok();

    // 3. Emit BrainOS context injection (task + contract + model + research + return).
    let injection = spawn_injection(input.get_string("description"), agent_type);
    drop(kavach_hook::exit_pre_tool_allow(injection.as_deref()));
    Ok(())
}
