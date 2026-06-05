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

    // CC 2.1.160: `ultracode` is the workflow-orchestration trigger. Word-boundary
    // match to avoid firing on substrings.
    if prompt
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case("ultracode"))
    {
        context.push_str(
            "\n[ULTRACODE] Workflow orchestration requested (CC 2.1.160). \
             Author + run a Workflow (multi-agent fan-out) for this task; \
             adversarially verify each delegated result before trusting it.",
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
