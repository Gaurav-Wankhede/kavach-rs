use kavach_types::HookInput;

use crate::error::EngineError;
use crate::gates::{
    pre_tool_agent, pre_tool_bash, pre_tool_question, pre_tool_read, pre_tool_search,
    pre_tool_skill, pre_tool_task, rule_eval,
};

/// Rate-limited per-turn quality nudge for the generic tool-allow path.
///
/// Returns `Some(advisory)` at most once per `reinforce_every_n` turn window
/// (reusing the purpose-built `needs_reinforcement`/`mark_reinforcement_done`
/// turn-window limiter) so it can NEVER spam: ≤1 fire per turn, and only on the
/// Nth-turn boundary. Returns `None` on every other call. The nudge restates the
/// same-turn autonomy contract as a soft advisory — no block, no control-flow
/// change. SOURCE: gate-severity policy §default-advisory + §no-spam budget.
fn perturn_nudge(session: &mut kavach_session::SessionState) -> Option<String> {
    if !session.needs_reinforcement() {
        return None;
    }
    session.mark_reinforcement_done();
    Some(String::from(
        "[QUALITY_NUDGE] Orchestrate, don't do the labor: decide -> FAN OUT to a cheap-tier \
         worker (Agent) or /workflow -> verify what it returns -> state result. Close the \
         active card this turn (claim -> spawn worker for implement+verify -> 3-witness its \
         result -> close); run the loophole self-check on the returned work before any done \
         claim. Do not hand labor back to the user, and do not do it yourself when a worker can.",
    ))
}

/// Helper to format rule context and apply nudge if needed.
fn apply_rule_context_and_nudge(
    rule_results: &[kavach_rule_engine::RuleResult],
    session: &mut kavach_session::SessionState,
) -> Option<String> {
    let mut ctx = rule_eval::results_to_context(rule_results);
    if ctx.is_empty()
        && let Some(nudge) = perturn_nudge(session)
    {
        ctx.push_str(&nudge);
    }
    if ctx.is_empty() { None } else { Some(ctx) }
}

/// Handle default tool in default arm: check rule engine output and enforce blocks.
fn handle_unmatched_tool(input: &HookInput, mut session: kavach_session::SessionState) {
    let rule_results = rule_eval::evaluate_rules(input);

    // Check if any rule produced a Block action. If so, fail-closed: deny.
    let worst = kavach_rule_engine::RuleEngine::worst_action(&rule_results);
    if worst == kavach_rule_engine::RuleAction::Block {
        let reason = rule_eval::results_to_context(&rule_results);
        let deny_reason = if reason.is_empty() {
            String::from("Rule engine: Block action enforced (no details)")
        } else {
            reason
        };
        super::turn_relay::exit_pre_tool_deny(&deny_reason);
    } else {
        // Warn and Allow/other: both allow relay, with optional context/nudge.
        let mut ctx = apply_rule_context_and_nudge(&rule_results, &mut session);
        if let Some(fan) = super::fanout_advisory::nudge(&mut session, &input.tool_name) {
            ctx = Some(match ctx {
                Some(c) => format!("{fan}\n\n{c}"),
                None => fan,
            });
        }
        super::turn_relay::exit_pre_tool_allow_relay(&mut session, ctx.as_deref());
    }
}

/// Pre-tool umbrella gate: bash blocklist + read validation + subagent budget.
/// Runs before any tool use (except Write/Edit which go through pre-write).
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    // MCP / shell-proxy tools that carry a raw `command` must run the destructive
    // blocklist too — else `rm -rf /` through an MCP tool (tool_name="MCP: …")
    // falls into the `_` allow arm and bypasses the Bash guard entirely.
    // The `command` field is the security-critical signal, not the tool name.
    // SOURCE: loophole audit (cursor-edge), runtime-proven via beforeMCPExecution.
    let carries_shell_command = input.tool_name != "Bash"
        && input.tool_input.as_ref().is_some_and(|ti| {
            ti.get("command")
                .and_then(|v| v.as_str())
                .is_some_and(|c| !c.is_empty())
        });
    if carries_shell_command {
        return pre_tool_bash::handle_bash(input);
    }

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
        "AskUserQuestion" => {
            pre_tool_question::handle_question(input);
            Ok(())
        }
        "Grep" | "Glob" => {
            let pattern = input
                .tool_input
                .as_ref()
                .and_then(|ti| ti.get("pattern"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(advisory) = super::symbol_search_guard::check_tool_search(&input.tool_name, pattern) {
                let mut session = kavach_session::get_or_create_session();
                super::turn_relay::exit_pre_tool_allow_relay(&mut session, Some(&advisory));
                return Ok(());
            }
            Ok(())
        }
        _ => {
            let mut session = kavach_session::get_or_create_session();
            let cfg = kavach_config::load_gates_config();
            // Block Agent tool when research required but not done (with read-only carve-out).
            // See decision.engine.agent_research_requirement.
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
                    "[ADVISORY:research-agent-spawn] STOP. Hit the internet FIRST. \
                     Run WebSearch on \"{topic}\" NOW, THEN spawn the agent. \
                     TABULA RASA: assume your training weights are STALE and WRONG — \
                     trust nothing from memory. Internet-first, always: SEARCH, \
                     corroborate across 2+ current sources, THEN delegate."
                );
                super::turn_relay::exit_pre_tool_allow_relay(&mut session, Some(&reason));
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
                    "[RESEARCH_PENDING] STOP. Hit the internet FIRST. Run WebSearch on \
                     \"{topic}\" NOW, BEFORE you write a single line. TABULA RASA: \
                     your training weights are STALE — generate NOTHING from memory. \
                     SEARCH the current authoritative source, corroborate, THEN write."
                );
                super::turn_relay::exit_pre_tool_allow_relay(&mut session, Some(&ctx));
                return Ok(());
            }
            handle_unmatched_tool(input, session);
            Ok(())
        }
    }
}
