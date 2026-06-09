//! Append the dynamic intent-context blocks (forbidden phrases, protocols,
//! effort/ultracode hints, test debt, phase + RIR pruning, RAG skill) onto the
//! base `[INTENT]` block built by the caller.
use std::fmt::Write as _;

use kavach_session::SessionState;
use kavach_types::HookInput;

use super::super::intent_context::{
    append_agent_dispatch, append_db_query_required, append_forbidden, append_memory_db,
    append_root_cause_protocol, append_verify_existing,
};
use super::phase::append_phase_and_rir;

/// Append every intent-derived context block to `context`. `forbidden` is the
/// research-gate forbidden-phrase list for this prompt.
pub(super) fn append_context_blocks(
    context: &mut String,
    input: &HookInput,
    session: &mut SessionState,
    intent_type: &str,
    prompt: &str,
    forbidden: &[String],
) {
    append_forbidden(context, forbidden);
    append_memory_db(context, intent_type);
    append_verify_existing(context, intent_type);
    append_root_cause_protocol(context, intent_type);
    append_agent_dispatch(context, intent_type);
    append_db_query_required(context, prompt);

    // CC 2.1.133: surface the active effort tier so downstream gates' strictness
    // is legible to the model. `low` relaxes the pre-write research block.
    let effort = input.effort_level();
    if !effort.is_empty() {
        writeln!(context, "\n[EFFORT] level:{effort}").ok();
    }

    // CC 2.1.160: detect the executing harness from the hook wire-shape so the
    // model knows WHAT it is running under (and whether the Workflow tool is even
    // reachable) instead of guessing. `transcript_path` is the Claude Code
    // signature — every CC hook payload carries it; a non-empty `agent_type` means
    // we are INSIDE a subagent (e.g. a Workflow-spawned agent), where authoring a
    // new Workflow is illegal (nesting is one level only).
    let in_subagent = !input.agent_type.is_empty();
    let is_claude_code = !input.transcript_path.is_empty() || in_subagent;
    if is_claude_code {
        let harness = if in_subagent {
            "claude-code:subagent"
        } else {
            "claude-code"
        };
        writeln!(
            context,
            "\n[HARNESS_ENV] {harness}{}{}",
            if input.model.is_empty() {
                String::new()
            } else {
                format!(" model:{}", input.model)
            },
            if in_subagent {
                format!(" agent_type:{}", input.agent_type)
            } else {
                String::new()
            },
        )
        .ok();
    }

    // CC 2.1.160: `ultracode` is the workflow-orchestration trigger. Word-boundary
    // match to avoid firing on substrings.
    let ultracode = prompt
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case("ultracode"));
    if ultracode && in_subagent {
        // Already executing as a delegated agent — Workflow nesting is one level,
        // so DO the assigned task directly; do not spawn another Workflow.
        context.push_str(
            "\n[ULTRACODE] You are already running INSIDE a Workflow subagent — \
             nesting is one level only, so do NOT call the Workflow tool again. \
             Execute the task you were dispatched with and return its result.",
        );
    } else if ultracode {
        // Top-level Claude Code loop: the Workflow tool is available and a call is
        // MANDATORY this turn. This is the lever against the failure mode where the
        // model answers with a hedging essay / option-menu instead of orchestrating.
        context.push_str(
            "\n[ULTRACODE] Workflow orchestration requested (CC 2.1.160). The \
             Workflow tool IS available in this Claude Code session. You MUST call \
             Workflow this turn — author a multi-agent fan-out for the task and run \
             it. Do NOT answer with a prose essay, a two-readings hedge, an option \
             menu, or a \"want me to…?\" question in place of the call. Adversarially \
             verify each delegated result before trusting it.",
        );
    }

    if let Some(test_ctx) = super::super::test_inject::build_test_context(session) {
        context.push('\n');
        context.push_str(&test_ctx);
    }
    if !session.memory_queried && session.turn_count <= 2 {
        context.push_str("\n[MEMORY] status:PENDING action:kavach db kanban --project <slug>");
    }
    if session.turn_count > 1 && session.turn_count % 5 == 0 {
        writeln!(
            context,
            "\n[DB_WRITE_REMINDER] turn:{} — kavach db write for any decisions/roadmap this turn.",
            session.turn_count
        )
        .ok();
    }
    if session.turn_count <= 1 {
        context.push('\n');
        context.push_str(&session.to_compact());
    }

    append_phase_and_rir(context, session);

    // Phase D: RAG skill routing — emit top skill name only, not full hit list.
    let top_skill = super::super::rag_router::top_skill_names_all("", prompt, intent_type, 1);
    if let Some(skill) = top_skill.first() {
        writeln!(context, "\n[RAG:skill] {skill}").ok();
    }
}

#[cfg(test)]
mod tests {
    use kavach_session::SessionState;
    use kavach_types::HookInput;

    use super::append_context_blocks;

    fn run(input: &HookInput, prompt: &str) -> String {
        let mut ctx = String::new();
        let mut session = SessionState::default();
        append_context_blocks(&mut ctx, input, &mut session, "security", prompt, &[]);
        ctx
    }

    #[test]
    fn claude_code_detected_from_transcript_path() {
        let input = HookInput {
            transcript_path: "/tmp/sess.jsonl".to_owned(),
            model: "claude-opus-4-8".to_owned(),
            ..HookInput::default()
        };
        let ctx = run(&input, "do the thing");
        assert!(ctx.contains("[HARNESS_ENV] claude-code"));
        assert!(ctx.contains("model:claude-opus-4-8"));
    }

    #[test]
    fn no_harness_env_without_evidence() {
        let ctx = run(&HookInput::default(), "do the thing");
        assert!(!ctx.contains("[HARNESS_ENV]"));
    }

    #[test]
    fn ultracode_top_level_mandates_workflow_call() {
        let input = HookInput {
            transcript_path: "/tmp/sess.jsonl".to_owned(),
            ..HookInput::default()
        };
        let ctx = run(&input, "ultracode fix this behaviour");
        assert!(ctx.contains("[ULTRACODE]"));
        assert!(ctx.contains("You MUST call"));
        // Must explicitly forbid the observed failure mode (hedging essay/menu).
        assert!(ctx.contains("hedge") || ctx.contains("option") || ctx.contains("essay"));
    }

    #[test]
    fn ultracode_inside_subagent_forbids_nesting() {
        let input = HookInput {
            transcript_path: "/tmp/sess.jsonl".to_owned(),
            agent_type: "general-purpose".to_owned(),
            ..HookInput::default()
        };
        let ctx = run(&input, "ultracode do the assigned task");
        assert!(ctx.contains("[HARNESS_ENV] claude-code:subagent"));
        assert!(ctx.contains("do NOT call the Workflow tool again"));
        assert!(!ctx.contains("You MUST call"));
    }

    #[test]
    fn ultracode_word_boundary_no_false_fire() {
        let input = HookInput {
            transcript_path: "/tmp/sess.jsonl".to_owned(),
            ..HookInput::default()
        };
        let ctx = run(&input, "this is ultracoded into the substring");
        assert!(!ctx.contains("[ULTRACODE]"));
    }
}
