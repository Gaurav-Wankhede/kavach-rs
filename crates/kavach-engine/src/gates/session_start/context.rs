//! Assemble the `[SESSION_START]` injected context from session + DB sources.
use std::fmt::Write as _;

use super::memory::auto_query_memory;
use super::patterns::{hot_pattern_context, mistake_ledger_context};

/// Build the full session-start context string for `session`, marking
/// `memory_queried` when the memory bank yields content.
pub(super) fn build(session: &mut kavach_session::SessionState) -> String {
    let cfg = kavach_config::ModelConfig::from_model_id(&session.model_id);
    // Prompt caching hint: static content (skills, rules) is injected by CC before this.
    // Mark boundary so CC can place cache_control breakpoint before dynamic session data.
    let mut context = String::from(kavach_hook::CACHE_BOUNDARY_MARKER);
    writeln!(context, "[SESSION_START]\nmodel: {}\ncontext_window: {}\nusable_budget: {}\ncontext_phase: {}\ndev_phase: {}\nproject: {}", session.model_id, cfg.context_window, cfg.usable_budget, session.context_phase, session.current_phase, session.project).ok();

    // Auto memory query: inject project context from kavach-db
    if !session.project.is_empty()
        && let Some(mem_ctx) = auto_query_memory(&session.project)
    {
        context.push_str(&mem_ctx);
        session.memory_queried = true;
    }

    let module_ctx = session.inject_modules_once(&["critical-rules", "date"]);
    context.push_str(&module_ctx);

    // Inject hot autonomous patterns so Claude sees cached fixes immediately.
    if let Some(hot_ctx) = hot_pattern_context(&session.project) {
        context.push_str(&hot_ctx);
    }

    // FIX: [contract_violation/silent_failure] no awareness of repeat mistakes.
    // SOURCE: arxiv.org/html/2512.11485 (Mistake Notebook Learning) — distill
    //   shared error patterns into structured "mistake notes"; surface them at
    //   the start of every turn so the model sees its own failure history
    //   BEFORE it can repeat it.
    // SOURCE: arxiv.org/pdf/2512.02389 — frame as anti-pattern (banned phrase
    //   + correct alternative), NEVER as raw error text (parrots).
    if let Some(ledger_ctx) = mistake_ledger_context(&session.project) {
        context.push_str(&ledger_ctx);
    }

    // [ALGO_EVOLUTION] removed — ~1kB/session token waste.
    // Algo decisions are on-demand: kavach db query --project <slug> --category decision
    context
}
