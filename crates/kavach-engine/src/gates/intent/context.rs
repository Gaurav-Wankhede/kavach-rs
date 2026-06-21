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
    // Carry-forward queued advisories from the previous turn's stop gate
    // before processing new work. See decision.engine.carry_forward.
    if let Some(carried) = session.drain_pending_advisories() {
        context.push_str("\n[CARRY_FORWARD] unfinished from last turn — FIX these at their root THIS turn, before any new work (close it or file a card; do not re-summarize):");
        for adv in carried {
            context.push_str("\n- ");
            context.push_str(&adv);
        }
    }

    append_forbidden(context, forbidden);
    append_memory_db(context, intent_type);
    append_verify_existing(context, intent_type);
    append_root_cause_protocol(context, intent_type);
    append_agent_dispatch(context, intent_type, prompt, &session.research_topic);
    append_db_query_required(context, prompt);

    append_mermaid_views(context, &session.project, prompt);

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
        context.push_str("\n[ULTRACODE] ");
        context.push_str(&crate::gates::directive_cache::dyn_directive(
            "ultracode.in-subagent",
            "You are already running INSIDE a Workflow subagent — \
             nesting is one level only, so do NOT call the Workflow tool again. \
             Execute the task you were dispatched with and return its result.",
        ));
    } else if ultracode {
        // Top-level Claude Code loop: the Workflow tool is available and a call is
        // MANDATORY this turn. This is the lever against the failure mode where the
        // model answers with a hedging essay / option-menu instead of orchestrating.
        context.push_str("\n[ULTRACODE] ");
        context.push_str(&crate::gates::directive_cache::dyn_directive(
            "ultracode.top-level",
            "Workflow orchestration requested. The \
             Workflow tool IS available in this Claude Code session. You MUST call \
             Workflow this turn — author a multi-agent fan-out for the task and run \
             it. Do NOT answer with a prose essay, a two-readings hedge, an option \
             menu, or a \"want me to…?\" question in place of the call. Adversarially \
             verify each delegated result before trusting it.",
        ));
    }

    if let Some(test_ctx) = super::super::test_inject::build_test_context(session) {
        context.push('\n');
        context.push_str(&test_ctx);
    }
    // Inject the LIVE board status, read from the kavach DB at hook time — do NOT
    // tell the agent to go read it. The Stop gate already reads the same RPC
    // (open_set_census + next_open_task); the entry hooks must too, or the agent is
    // perpetually told to fetch what the hook could have handed it. Fail-soft to the
    // legacy nag on RPC outage so prompt submission is never blocked.
    if !session.memory_queried && session.turn_count <= 2 {
        if append_live_kanban(context, &session.project) {
            session.memory_queried = true;
        } else {
            context
                .push_str("\n[MEMORY] status:PENDING action:kavach db kanban --project <slug>");
        }
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

/// The SINGLE canonical emitter of the three Mermaid-VIEW blocks (all read-side,
/// fail-soft, never stored): `[DECISION_MAP]` settled architecture,
/// `[PRACTICE_DELTA]` retired worst- vs best-practice, `[PATTERN_DAG]`
/// research-refreshed pattern supersession. BOTH the user-prompt (intent) and
/// the session-start hook call THIS function — never a hand-listed copy — so the
/// triad can never drift out of one hook again (it once vanished from session-start
/// for that reason). At session-start pass `prompt = ""` (whole-spine, not
/// relevance-narrowed). See decision.harness.shared-mermaid-injection-emitter.
pub(in crate::gates) fn append_mermaid_views(context: &mut String, project: &str, prompt: &str) {
    if let Some(map) = super::decision_map::decision_map_block(project, prompt) {
        context.push_str(&map);
    }
    if let Some(delta) = super::practice_delta::practice_delta_block() {
        context.push_str(&delta);
    }
    if let Some(pd) = super::pattern_dag::pattern_dag_block(project) {
        context.push_str(&pd);
    }
}

/// Read the LIVE kanban board from the kavach DB (same RPC path the Stop gate
/// uses) and append a `[KANBAN]` status block to `context`. Returns `true` when
/// the board was observed (block injected), `false` on an empty slug or RPC
/// outage so the caller fails soft to the legacy "go query it yourself" nag.
///
/// Census `(runnable, blocked, cyclic)`: runnable = roadmap cards in a
/// dispatchable status (`todo` / `in_progress`) — the ONLY work queue. A 0/0/0
/// board is genuinely drained; the agent is told so plainly instead of being sent
/// to re-query. The next dispatchable card (if any) is named so the agent can start
/// it without a round-trip.
/// Cross-module entry (used by `session_start`): append the live `[KANBAN]`
/// block, ignoring the observed/outage flag. Session start is never blocked on
/// an outage — the block is simply omitted.
pub(in crate::gates) fn append_live_kanban_block(context: &mut String, project: &str) {
    let _ = append_live_kanban(context, project);
}

fn append_live_kanban(context: &mut String, project: &str) -> bool {
    if project.is_empty() {
        return false;
    }
    // RPC-ONLY: a hot entry hook must NOT trigger the daemon self-heal /
    // direct-DB cold-open path (`open_set_census`) — that blocks the hook on a
    // RocksDB open when no daemon is warm (observed as a nextest SIGTERM on the
    // SessionStart lifecycle test). The Stop gate may use the heavy path because
    // it only runs the census on its already-drained branch where a daemon is
    // already warm; SessionStart / UserPromptSubmit have no such guarantee, so
    // they read RPC-only and fail soft+fast to the legacy nag on any outage.
    let Some((runnable, blocked, cyclic)) =
        crate::gates::stop_dispatch::census_rpc_only(project)
    else {
        return false; // daemon unreachable -> fail soft to the nag, never block
    };
    context.push_str("\n[KANBAN] read live from the kavach DB this turn — do NOT re-query to confirm.");
    write!(
        context,
        "\nproject: {project} · runnable: {runnable} · blocked: {blocked} · cyclic: {cyclic}"
    )
    .ok();
    if runnable == 0 {
        context.push_str(
            "\nstatus: no runnable roadmap card. The work queue (roadmap+todo/in_progress) is \
             drained. Do NOT invent work; if the user gave no task, await direction.",
        );
    } else if let Some((key, title)) = crate::gates::stop_dispatch::next_task_rpc_only(project) {
        write!(context, "\nnext runnable card: [{key}] {title} — claim and START it.").ok();
    }
    true
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
    fn empty_project_yields_no_live_kanban_and_no_panic() {
        // Empty slug must short-circuit BEFORE any RPC — fail soft, no block.
        let mut ctx = String::new();
        let observed = super::append_live_kanban(&mut ctx, "");
        assert!(!observed, "empty slug is never 'observed'");
        assert!(ctx.is_empty(), "no [KANBAN] block for an empty slug: {ctx}");
    }

    #[test]
    fn live_kanban_falls_soft_to_nag_on_outage() {
        // With no daemon reachable in the unit-test process, the census read
        // returns None -> append_context_blocks must fall back to the legacy
        // PENDING nag rather than emit a half-formed [KANBAN] block or panic.
        let input = HookInput { transcript_path: "/tmp/s.jsonl".to_owned(), ..HookInput::default() };
        let mut ctx = String::new();
        let mut session = SessionState { project: "kavach-rs".to_owned(), ..SessionState::default() };
        append_context_blocks(&mut ctx, &input, &mut session, "debug", "fix the gate", &[]);
        // Either the live block (daemon up in this env) or the nag (daemon down):
        // exactly one path fires, never a panic, never both half-rendered.
        let has_live = ctx.contains("[KANBAN]");
        let has_nag = ctx.contains("[MEMORY] status:PENDING");
        assert!(has_live ^ has_nag, "exactly one of live-board / nag must fire: {ctx}");
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
