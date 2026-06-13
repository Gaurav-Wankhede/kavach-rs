//! Assemble the `[SESSION_START]` injected context from session + DB sources.
use std::fmt::Write as _;

use super::memory::auto_query_memory;
use super::patterns::{hot_pattern_context, learned_policy_context, mistake_ledger_context};
use super::concepts::concept_context;
use super::flows::flow_context;

/// Soft per-section byte budget for OPTIONAL session-start blocks (hot patterns,
/// mistake ledger, learned policy). The `[AUTONOMY_CONTRACT]` anchor is never
/// subject to this cap — a large `MISTAKE_LEDGER` must NOT crowd out the only
/// per-conversation model-readable operating contract on Cursor. Each optional
/// block is truncated to this many bytes (at a UTF-8 char boundary) before being
/// appended; the contract and the `[SESSION_START]` header are always emitted whole.
const OPTIONAL_SECTION_BUDGET: usize = 1_200;

/// The imperative Opus-style operating contract. Kept tight (<800 bytes) and
/// injected immediately after the `[SESSION_START]` header so it is the FIRST
/// model-readable directive every conversation — the single per-conversation
/// door on Cursor (no per-session system prompt is available there).
const AUTONOMY_CONTRACT: &str = "[AUTONOMY_CONTRACT]\n\
    Act, don't narrate: execute -> show output -> state result. You do ALL labor end-to-end; never hand it back.\n\
    Same-turn loop: claim card -> implement -> 3-witness verify (artifact exists -> diff landed -> build passes) -> close, in ONE turn. Naming the next card commits you to STARTING it this turn.\n\
    Loophole self-interrogation BEFORE any done/verified claim: ask \"how would a hostile/concurrent/malformed/crashed actor break this?\" and answer with file:line evidence, not optimism.\n\
    Research before code (tabula rasa): WebSearch/read the source first; do not generate from training weights.\n\
    Never end a turn with an option menu, \"over to you\", or a card left in_progress. Continue while runnable work exists.\n";

/// Truncate `s` to at most `max` bytes on a UTF-8 char boundary, appending a
/// terminal marker when it was cut. Used only for OPTIONAL blocks so the anchor
/// can never be displaced.
fn truncate_section(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    // Collect chars whose cumulative UTF-8 byte length stays within `max`. This
    // never splits a char and never indexes into the string by byte (both are
    // clippy-denied here), avoiding panics on multi-byte boundaries.
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

/// Build the full session-start context string for `session`, marking
/// `memory_queried` when the memory bank yields content.
pub(super) fn build(session: &mut kavach_session::SessionState) -> String {
    let cfg = kavach_config::ModelConfig::from_model_id(&session.model_id);
    // Prompt caching hint: static content (skills, rules) is injected by CC before this.
    // Mark boundary so CC can place cache_control breakpoint before dynamic session data.
    let mut context = String::from(kavach_hook::CACHE_BOUNDARY_MARKER);
    writeln!(context, "[SESSION_START]\nmodel: {}\ncontext_window: {}\nusable_budget: {}\ncontext_phase: {}\ndev_phase: {}\nproject: {}", session.model_id, cfg.context_window, cfg.usable_budget, session.context_phase, session.current_phase, session.project).ok();

    // Inject the operating contract FIRST (right after the header) and never
    // under any byte cap — on Cursor this is the only per-conversation door.
    context.push_str(AUTONOMY_CONTRACT);

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
    // OPTIONAL block: subject to the soft section budget (anchor stays whole).
    if let Some(hot_ctx) = hot_pattern_context(&session.project) {
        context.push_str(&truncate_section(&hot_ctx, OPTIONAL_SECTION_BUDGET));
    }

    // FIX: [contract_violation/silent_failure] no awareness of repeat mistakes.
    // SOURCE: arxiv.org/html/2512.11485 (Mistake Notebook Learning) — distill
    //   shared error patterns into structured "mistake notes"; surface them at
    //   the start of every turn so the model sees its own failure history
    //   BEFORE it can repeat it.
    // SOURCE: arxiv.org/pdf/2512.02389 — frame as anti-pattern (banned phrase
    //   + correct alternative), NEVER as raw error text (parrots).
    // OPTIONAL block: a large ledger must never displace the contract, so cap it.
    if let Some(ledger_ctx) = mistake_ledger_context(&session.project) {
        context.push_str(&truncate_section(&ledger_ctx, OPTIONAL_SECTION_BUDGET));
    }

    // P6: surface the RLVR-learned advisory policy (informational only — the loop
    // learned these gate preferences from verifiable rewards; never a directive).
    if let Some(policy_ctx) = learned_policy_context() {
        context.push_str(&truncate_section(&policy_ctx, OPTIONAL_SECTION_BUDGET));
    }

    if let Some(reward_ctx) = super::super::loop_frame::build_reward_session_stats(session) {
        context.push_str(&truncate_section(&reward_ctx, OPTIONAL_SECTION_BUDGET));
    }

    if let Some(concept_ctx) = concept_context(&session.project) {
        context.push_str(&truncate_section(&concept_ctx, OPTIONAL_SECTION_BUDGET));
    }

    // [FLOW] implementation-flow DAGs rendered as Mermaid — the intended order
    // of work, surfaced BEFORE the model starts so it follows the plan. OPTIONAL
    // block: capped so a large flow never displaces the [AUTONOMY_CONTRACT].
    if let Some(flow_ctx) = flow_context(&session.project) {
        context.push_str(&truncate_section(&flow_ctx, OPTIONAL_SECTION_BUDGET));
    }

    // [ALGO_EVOLUTION] removed — ~1kB/session token waste.
    // Algo decisions are on-demand: kavach db query --project <slug> --category decision
    context
}
