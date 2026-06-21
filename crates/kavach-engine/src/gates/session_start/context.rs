//! Assemble the `[SESSION_START]` injected context from session + DB sources.
use std::fmt::Write as _;

use super::memory::auto_query_memory;
use super::patterns::{hot_pattern_context, learned_policy_context, mistake_ledger_context};
use super::concepts::concept_context;
use super::flows::flow_context;
use super::stack_fit::stack_fit_context;

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

/// Lower / upper byte clamp for the optional-section budget. The floor stops a
/// hostile zero override from starving every block; the ceiling stops one row
/// from blowing the session-start token budget. Both are well within `f64`'s
/// 52-bit lossless integer range, so the conversions below cannot lose data.
const BUDGET_FLOOR: usize = 256;
const BUDGET_CEIL: usize = 16_384;

/// Resolve the per-project optional-section byte budget from the gate-config
/// overlay, clamped to `[BUDGET_FLOOR, BUDGET_CEIL]`. Any miss / NaN / out-of-range
/// override collapses into the clamp — fail-closed, never a starved or runaway cap.
fn resolve_section_budget(project: &str) -> usize {
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are clamped to [256, 16384] before conversion — far inside f64's \
                  52-bit lossless integer range, so no precision/sign/truncation loss is possible"
    )]
    {
        let default = OPTIONAL_SECTION_BUDGET as f64;
        let resolved = crate::gates::gate_config::gate_threshold(
            project,
            "session.optional_section_budget",
            default,
        );
        // NaN fails every comparison -> clamp() would propagate NaN, so guard first.
        let safe = if resolved.is_finite() { resolved } else { default };
        let clamped = safe.clamp(BUDGET_FLOOR as f64, BUDGET_CEIL as f64);
        clamped as usize
    }
}

/// Build the full session-start context string for `session`, marking
/// `memory_queried` when the memory bank yields content.
pub(super) fn build(session: &mut kavach_session::SessionState) -> String {
    let cfg = kavach_config::ModelConfig::from_model_id(&session.model_id);
    // Prompt caching hint: static content (skills, rules) is injected by CC before this.
    // Mark boundary so CC can place cache_control breakpoint before dynamic session data.
    let mut context = String::from(kavach_hook::CACHE_BOUNDARY_MARKER);
    writeln!(context, "[SESSION_START]\nmodel: {}\ncontext_window: {}\nusable_budget: {}\ncontext_phase: {}\ndev_phase: {}\nproject: {}", session.model_id, cfg.context_window, cfg.usable_budget, session.context_phase, session.current_phase, session.project).ok();

    // Live temporal anchor (Tabula Rasa awareness): the CURRENT weekday + date,
    // injected fresh every session so the model grounds "as of today" research
    // to the precise day rather than a stale training-weight assumption. Must be
    // a LIVE value — the old static `date` module froze at authoring time, so it
    // is no longer injected (a frozen date would contradict this live line).
    writeln!(
        context,
        "[TEMPORAL_AWARENESS]\ntoday: {}\nrule: treat THIS as the current date. When researching, search for information current as of today; do not assume the training-cutoff date.",
        kavach_hook::today_full()
    )
    .ok();

    // Inject the operating contract FIRST (right after the header) and never
    // under any byte cap — on Cursor this is the only per-conversation door.
    // Resolves through the gate-config overlay (`session.autonomy_contract`) so
    // an operator can retune the per-conversation directive without a rebuild;
    // the compiled string is the fail-closed default on any miss.
    let contract = crate::gates::gate_config::gate_text(
        &session.project,
        "session.autonomy_contract",
        AUTONOMY_CONTRACT,
    );
    context.push_str(&contract);

    // Optional-block byte budget, runtime-tunable per project via the gate-config
    // overlay (`session.optional_section_budget`); the compiled constant is the
    // fail-closed default. Clamped to a sane floor so a hostile/zero override can
    // never starve every optional block to nothing.
    let budget = resolve_section_budget(&session.project);

    // Auto memory query: inject project context from kavach-db
    if !session.project.is_empty()
        && let Some(mem_ctx) = auto_query_memory(&session.project)
    {
        context.push_str(&mem_ctx);
        session.memory_queried = true;
    }

    // Live board status, read from the kavach DB at SessionStart — the same RPC
    // the Stop gate uses. The session must OPEN with the real board (counts + next
    // card), not a "run kavach db kanban yourself" reminder. Fail-soft on RPC
    // outage: the block is simply omitted (session start is never blocked).
    if !session.project.is_empty() {
        super::super::intent::append_live_kanban_block(&mut context, &session.project);
    }

    // E7 compaction-seam reconcile: if an in_progress card's TOUCHES paths match
    // the dirty tree and no status-update was recorded, auto-compact likely fired
    // between the edit and its status-update — surface a [RECONCILE] directive to
    // resume at the VERIFY step rather than re-edit. Emitted ONLY in the seam case;
    // fail-soft otherwise (omitted on a clean tree / no hint / RPC miss).
    if let Some(reconcile_ctx) = super::reconcile::reconcile_context(&session.project) {
        context.push_str(&reconcile_ctx);
    }

    // `date` module dropped: superseded by the live [TEMPORAL_AWARENESS] line
    // above (a static module froze the date at authoring time).
    let module_ctx = session.inject_modules_once(&["critical-rules"]);
    context.push_str(&module_ctx);

    // Inject hot autonomous patterns so Claude sees cached fixes immediately.
    // OPTIONAL block: subject to the soft section budget (anchor stays whole).
    if let Some(hot_ctx) = hot_pattern_context(&session.project) {
        context.push_str(&truncate_section(&hot_ctx, budget));
    }

    // Inject mistake ledger for repeat-pattern awareness. SOURCE: decision.engine.mistake_ledger_session_injection.
    if let Some(ledger_ctx) = mistake_ledger_context() {
        context.push_str(&truncate_section(&ledger_ctx, budget));
    }

    // P6: surface the RLVR-learned advisory policy (informational only — the loop
    // learned these gate preferences from verifiable rewards; never a directive).
    if let Some(policy_ctx) = learned_policy_context() {
        context.push_str(&truncate_section(&policy_ctx, budget));
    }

    if let Some(reward_ctx) = super::super::loop_frame::build_reward_session_stats(session) {
        context.push_str(&truncate_section(&reward_ctx, budget));
    }

    if let Some(concept_ctx) = concept_context(&session.project) {
        context.push_str(&truncate_section(&concept_ctx, budget));
    }

    // [FLOW] implementation-flow DAGs rendered as Mermaid — the intended order
    // of work, surfaced BEFORE the model starts so it follows the plan. OPTIONAL
    // block: capped so a large flow never displaces the [AUTONOMY_CONTRACT].
    if let Some(flow_ctx) = flow_context(&session.project) {
        context.push_str(&truncate_section(&flow_ctx, budget));
    }

    // [STACK_FIT] chosen language/tech-stack bound to its non-negotiable
    // boundaries — VIEW over stack.* app_spec rows, agnostic, fail-soft.
    if let Some(stack_ctx) = stack_fit_context(&session.project) {
        context.push_str(&truncate_section(&stack_ctx, budget));
    }

    // DECISION_MAP/PRACTICE_DELTA/PATTERN_DAG triad via the SINGLE canonical
    // emitter both hooks share (no hand-listed copy → can't drift). "" prompt →
    // whole-spine. The triad is budget-capped as a unit here (it self-truncates
    // per-block via its own RPC max_nodes). See
    // decision.harness.shared-mermaid-injection-emitter.
    if !session.project.is_empty() {
        let mut triad = String::new();
        super::super::intent::append_mermaid_views(&mut triad, &session.project, "");
        if !triad.is_empty() {
            context.push_str(&truncate_section(&triad, budget));
        }
    }

    // [ALGO_EVOLUTION] removed — ~1kB/session token waste.
    // Algo decisions are on-demand: kavach db query --project <slug> --category decision
    context
}
