use kavach_types::HookInput;

use crate::error::EngineError;
use crate::gates::{
    pre_tool_agent, pre_tool_bash, pre_tool_read, pre_tool_search, pre_tool_skill, pre_tool_task,
    rule_eval,
};

/// Pre-tool umbrella gate: bash blocklist + read validation + subagent budget.
/// Runs before any tool use (except Write/Edit which go through pre-write).
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    match input.tool_name.as_str() {
        "Bash" => pre_tool_bash::handle_bash(input),
        "Read" => {
            pre_tool_read::handle_read(input);
            Ok(())
        }
        "Task" => pre_tool_task::handle_task(input),
        "Skill" => {
            pre_tool_skill::handle_skill(input);
            Ok(())
        }
        "WebSearch" | "WebFetch" => {
            pre_tool_search::run(input);
            Ok(())
        }
        "Agent" => pre_tool_agent::handle_agent(input),
        _ => {
            let mut session = kavach_session::get_or_create_session();
            let cfg = kavach_config::load_gates_config();
            // Hard block Agent tool when research required but not done.
            // Agent spawns can generate code from training weights in subagent context
            // where pre-write research gate is exempted for subagents.
            //
            // CARVE-OUT 1: audit/analyze/explain/read intents on local artifacts
            // don't need external research — they inspect existing code.
            // Per ~/.claude/CLAUDE.md §Research Cadence carve-outs.
            //
            // FIX: [contract_violation] pre_tool.rs:29
            // SYMPTOM: Agent tool blocked for research-director which performs research
            // WHY5: Gate blocked ALL agents without checking if agent CAN do research
            // ROOT_CAUSE: Missing exemption for read_only agents that perform research
            // BLAST_SITE: 1/1 — only site of agent research gate
            // RESEARCH: github.com/NousResearch/hermes-agent/issues/21916 — similar deadlock
            // SOLUTION: Check agent_type against AGENT_CONTRACTS; exempt read_only agents
            let intent_is_local_analysis = matches!(
                session.intent_type.as_str(),
                "audit" | "analyze" | "explain" | "read" | "review" | "explore"
            );
            // CARVE-OUT 2: read_only agents (research-director, Explore, code-reviewer,
            // spec-author, context-curator) CAN be spawned to PERFORM the research.
            // Blocking them creates a deadlock where research can't be done.
            let agent_is_read_only = input.tool_name == "Agent" && {
                let agent_type = input
                    .tool_input
                    .as_ref()
                    .and_then(|ti| ti.get("subagent_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                pre_tool_agent::get_contract(agent_type).is_some_and(|c| c.read_only)
            };
            if input.tool_name == "Agent"
                && cfg.research.enabled
                && cfg.research.require_before_code
                && !session.research_done
                && session.intent_risk != "low"
                && !intent_is_local_analysis
                && !agent_is_read_only
            {
                let topic = if session.research_topic.is_empty() {
                    "relevant topic"
                } else {
                    &session.research_topic
                };
                let reason = format!(
                    "[ADVISORY:research-agent-spawn] WebSearch recommended before \
                     spawning agents. Topic: {topic}. Tabula rasa: do not trust \
                     training weights — WebSearch first, then delegate."
                );
                drop(kavach_hook::exit_pre_tool_allow(Some(&reason)));
                return Ok(());
            }
            // Advisory: remind model about pending research — fire once per intent window.
            if cfg.research.enabled
                && !session.research_done
                && !session.research_topic.is_empty()
                && !session.research_advisory_sent
            {
                let topic = session.research_topic.clone();
                session.research_advisory_sent = true;
                session.save().ok();
                let ctx = format!(
                    "[RESEARCH_PENDING] WebSearch \"{topic}\" before writing code. \
                     Do not generate code from training weights."
                );
                drop(kavach_hook::exit_pre_tool_allow(Some(&ctx)));
                return Ok(());
            }
            let rule_ctx = rule_eval::results_to_context(&rule_eval::evaluate_rules(input));
            if rule_ctx.is_empty() {
                drop(kavach_hook::exit_pre_tool_allow(None));
            } else {
                drop(kavach_hook::exit_pre_tool_allow(Some(&rule_ctx)));
            }
            Ok(())
        }
    }
}
