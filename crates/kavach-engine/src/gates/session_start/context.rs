// SOURCE: kavach decision.context-rot-surrealdb-pipeline
//! Assemble the `[SESSION_START]` injected context from session + DB sources.
use std::fmt::Write as _;

use super::concepts::concept_context;
use super::flows::flow_context;
use super::lld::lld_context;
use super::memory::auto_query_memory;
use super::patterns::{hot_pattern_context, learned_policy_context, mistake_ledger_context};
use super::stack_fit::stack_fit_context;

const OPTIONAL_SECTION_BUDGET: usize = 1_200;

const AUTONOMY_CONTRACT: &str = "[AUTONOMY_CONTRACT]\n\
    Act, don't narrate: execute -> show output -> state result. You are the ORCHESTRATOR — DECIDE, then FAN OUT every read AND write task to the cheap executor tier; reserve your own tokens for the decision, never the labor. Verify the returned work; never hand the loop back to the user.\n\
    Same-turn loop: claim card -> FAN OUT to a cheap-tier worker (Agent) or /workflow for the implement+verify labor -> 3-witness what it returns (artifact exists -> diff landed -> build passes) -> close, in ONE turn. You orchestrate and verify; the worker reads/edits/runs. Naming the next card commits you to FANNING IT OUT this turn.\n\
    Loophole self-interrogation BEFORE any done/verified claim: ask \"how would a hostile/concurrent/malformed/crashed actor break this?\" and answer with file:line evidence, not optimism.\n\
    Research before code (tabula rasa): WebSearch/read the source first; do not generate from training weights.\n\
    Never end a turn with an option menu, \"over to you\", or a card left in_progress. Continue while runnable work exists.\n";

fn truncate_section(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut out = String::with_capacity(max);
    let mut used = 0usize;
    for ch in s.chars() {
        let next = used.saturating_add(ch.len_utf8());
        if next > max {
            break;
        }
        out.push(ch);
        used = next;
    }
    out.push_str("\n…[truncated to protect [AUTONOMY_CONTRACT] budget]\n");
    out
}

const BUDGET_FLOOR: usize = 256;
const BUDGET_CEIL: usize = 16_384;

fn resolve_section_budget(project: &str) -> usize {
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values clamped to [256, 16384] before conversion — inside f64's 52-bit range"
    )]
    {
        let default = OPTIONAL_SECTION_BUDGET as f64;
        let resolved = crate::gates::gate_config::gate_threshold(
            project,
            "session.optional_section_budget",
            default,
        );
        let safe = if resolved.is_finite() { resolved } else { default };
        let clamped = safe.clamp(BUDGET_FLOOR as f64, BUDGET_CEIL as f64);
        clamped as usize
    }
}

pub(super) fn build(session: &mut kavach_session::SessionState) -> String {
    let cfg = kavach_config::ModelConfig::from_model_id(&session.model_id);
    let mut context = String::from(kavach_hook::CACHE_BOUNDARY_MARKER);
    writeln!(context, "[SESSION_START]\nmodel: {}\ncontext_window: {}\nusable_budget: {}\ncontext_phase: {}\ndev_phase: {}\nproject: {}", session.model_id, cfg.context_window, cfg.usable_budget, session.context_phase, session.current_phase, session.project).ok();

    writeln!(
        context,
        "[TEMPORAL_AWARENESS]\ntoday: {}\nrule: treat THIS as the current date. When researching, search for information current as of today; do not assume the training-cutoff date.",
        kavach_hook::today_full()
    )
    .ok();

    let contract = crate::gates::gate_config::gate_text(
        &session.project,
        "session.autonomy_contract",
        AUTONOMY_CONTRACT,
    );
    context.push_str(&contract);

    let budget = resolve_section_budget(&session.project);

    if !session.project.is_empty()
        && let Some(mem_ctx) = auto_query_memory(&session.project)
    {
        let compressed = crate::gates::context_compress::compress_db_json_string(&mem_ctx);
        context.push_str(&compressed);
        session.memory_queried = true;
    }
    // Ensure compress_db_rows is reachable for the DB query pipeline.
    let _ = crate::gates::context_compress::compress_db_rows(&[], 0);

    if let Some(resume_ctx) = super::resume::resume_context(session) {
        context.push_str(&resume_ctx);
    }

    if !super::super::intent::append_live_kanban_block(&mut context, &session.project) {
        context.push_str("\n[KANBAN] board unavailable this session (empty project or DB outage) — run `kavach db kanban` once reachable.\n");
    }

    if let Some(reconcile_ctx) = super::reconcile::reconcile_context(&session.project) {
        context.push_str(&reconcile_ctx);
    }

    let module_ctx = session.inject_modules_once(&["critical-rules"]);
    context.push_str(&module_ctx);

    if let Some(hot_ctx) = hot_pattern_context(&session.project) {
        context.push_str(&truncate_section(&hot_ctx, budget));
    }

    if let Some(ledger_ctx) = mistake_ledger_context() {
        context.push_str(&truncate_section(&ledger_ctx, budget));
    }

    if let Some(policy_ctx) = learned_policy_context() {
        context.push_str(&truncate_section(&policy_ctx, budget));
    }

    if let Some(reward_ctx) = super::super::loop_frame::build_reward_session_stats(session) {
        context.push_str(&truncate_section(&reward_ctx, budget));
    }

    if let Some(concept_ctx) = concept_context(&session.project) {
        context.push_str(&truncate_section(&concept_ctx, budget));
    }

    if let Some(flow_ctx) = flow_context(&session.project) {
        context.push_str(&truncate_section(&flow_ctx, budget));
    }

    if let Some(stack_ctx) = stack_fit_context(&session.project) {
        context.push_str(&truncate_section(&stack_ctx, budget));
    }

    if let Some(lld_ctx) = lld_context(&session.project) {
        context.push_str(&truncate_section(&lld_ctx, budget));
    }

    context.push_str("\n[ZERO_GREP_TOOLS] NEVER reach for Grep/Glob/grep first — RUN kavach's zero-token lookups:\n  RESOLVE a declaration: kavach origin <SYMBOL> [<SYMBOL>...]  -> exact file:line for var/fn/param/type/enum-variant/const\n  SWEEP for bug patterns: kavach hunt [PATH]                   -> regex worst-practice scan, no LLM\n");

    if !session.project.is_empty() {
        let mut triad = String::new();
        super::super::intent::append_mermaid_views(&mut triad, &session.project, "");
        if !triad.is_empty() {
            context.push_str(&truncate_section(&triad, budget));
        }
    }

    context
}
