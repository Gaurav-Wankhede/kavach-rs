// Stage 3: Language guards — chain verification + tiered severity system.
// See decision.engine.tiered_guard_severity and CLAUDE.md § WIRING MAP.
// hub: re-exports `check` + `GuardResult`; the guard groups live in submodules.
// Severity tiers (by comment convention):
//   P0 = security-critical (hard block) — OWASP, crypto, DB security, frontend security
//   P1 = quality (advisory in context) — rust patterns, ts patterns, complexity, UX
//   P2 = style (silent log) — algo complexity, secrecy, alloc hints
mod advisories;
mod algo_arch;
mod chain;
mod guards2026;
mod nanofile;
mod quality;
mod research_consume;
mod result;
mod retired_pattern;
mod security;
pub(crate) mod tdd_guard;
#[cfg(test)]
#[path = "pre_write_guards_test.rs"]
mod tests;
use result::Acc;
pub(crate) use result::GuardResult;
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;
/// Run chain verification and tiered guards.
/// P0 (security) = hard block. P1 (quality) = advisory. P2 (style) = silent.
///
/// The guard groups run in a fixed order; the first to return a block reason
/// short-circuits, carrying whatever advisories were accumulated before it.
pub(crate) fn check(
    ctx: &WriteContext<'_>,
    input: &HookInput,
    session: &kavach_session::SessionState,
) -> GuardResult {
    let mut acc = Acc::default();
    let mut runner = kavach_chain::Runner::new(&session.session_id);
    // TDD is advisory, never a block. SOURCE: decision.tdd-gate-is-advisory-not-block.
    if let Some(nudge) = tdd_guard::check(ctx, session) {
        acc.p1_advisories.push(nudge);
    }
    // Comment-bloat BLOCK: deny only a write that INTRODUCES new bloat (pre-existing
    // bloat stays editable). Escape: KAVACH_COMMENT_BLOCK_OFF=1.
    if std::env::var_os("KAVACH_COMMENT_BLOCK_OFF").is_none() {
        let old = std::fs::read_to_string(ctx.file_path).unwrap_or_default();
        if kavach_patterns::comment_noise_guard::introduces_bloat(
            ctx.file_path,
            &old,
            &ctx.effective_content,
        ) {
            return GuardResult {
                block: Some(
                    "[COMMENT_BLOAT:P0] TRIM THE COMMENT, MOVE RATIONALE TO A DECISION ROW. \
                     This write adds a 6+-line prose comment block — keep comments concise. \
                     If this looks wrong, READ this guard's source and fix the real cause — never route around it."
                        .to_owned(),
                ),
                algo_advisory: None,
                runner_compact: runner.to_compact(),
                p1_advisories: Vec::new(),
            };
        }
    }
    // Internet-first is a P0 LAW: a research-required production write with no cited
    // source is BLOCKED at write time (fail-closed). Runs FIRST — no source, no claim,
    // no other guard even evaluated. The gate still drives the lookup so the agent can
    // cite + retry immediately. Carve-outs (test/non-code/local-analysis/bypass) inside.
    if let Some(block) = research_consume::check(ctx, session) {
        return GuardResult {
            block: Some(block),
            algo_advisory: None,
            runner_compact: runner.to_compact(),
            p1_advisories: Vec::new(),
        };
    }
    // Ordered guard chain; the first `Some(reason)` blocks and short-circuits.
    let block = retired_pattern::check(ctx, &session.project) // F: ledger-retired pattern
        .or_else(|| chain::check(ctx, input, session, &mut runner))
        .or_else(|| {
            quality::lang(ctx, &mut acc); // P1 rust/ts
            security::check(ctx) // P0 sql/db/owasp/silent/crypto/gnap/frontend-sec
        })
        .or_else(|| {
            quality::presentation(ctx, &mut acc); // P1 css/ux/complexity
            nanofile::nano_file(ctx, &mut acc) // P0/P1 nano-file
        })
        .or_else(|| {
            advisories::collect(ctx, &mut acc); // P2 algo/secrecy/alloc/a11y
            algo_arch::algo(ctx, session, &mut acc) // P0/inject algo hunter
        })
        .or_else(|| algo_arch::arch(ctx, session, &mut acc)) // P0/inject arch
        .or_else(|| platform(ctx, &mut acc)) // P1 response/infra + P0 microservice
        .or_else(|| guards2026::check(ctx, &mut acc)); // 2026 guard block
    GuardResult {
        block,
        algo_advisory: acc.algo_advisory,
        runner_compact: runner.to_compact(),
        p1_advisories: acc.p1_advisories,
    }
}
/// Universal platform guards in original order: response (P1), microservice
/// (P0 block), infra (P1), then the API-gateway pattern advisory (P1).
fn platform(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    if ctx.is_test {
        return None;
    }
    if let Some(msg) = super::pre_write_response_guard::check(ctx.file_path, ctx.content) {
        acc.p1_advisories.push(format!("[RESPONSE_P1] {msg}"));
    }
    if let Some(block) = nanofile::microservice(ctx) {
        return Some(block);
    }
    if let Some(msg) = super::pre_write_infra_guard::check(ctx.file_path, ctx.content) {
        acc.p1_advisories.push(format!("[INFRA_P1] {msg}"));
    }
    if let Some(msg) = super::api_gateway_guard::check(ctx.file_path, ctx.content) {
        acc.p1_advisories.push(format!("[API_GATEWAY_P1] {msg}"));
    }
    production_audit(ctx, acc);
    None
}
/// Multi-category production-pattern audit (`kavach_patterns::production_patterns`).
/// This detector was a `pub mod` with ZERO call sites — defined-but-never-enforced.
/// Wired here as a P1 ADVISORY rollup (NOT a block): it overlaps existing P0
/// security guards, so blocking on its full set would risk a false-positive storm
/// against the <1% bar. Surfaced as one compact advisory naming the highest-
/// severity hits so the signal is no longer dark. Reuses the crate's own pattern
/// table — no rule is re-declared in the engine.
fn production_audit(ctx: &WriteContext<'_>, acc: &mut Acc) {
    let matches = kavach_patterns::production_patterns::scan(ctx.file_path, &ctx.effective_content);
    if matches.is_empty() {
        return;
    }
    let crit = kavach_patterns::production_patterns::count_critical(&matches);
    let mut codes: Vec<&str> = matches.iter().take(5).map(|m| m.code).collect();
    codes.dedup();
    acc.p1_advisories.push(format!(
        "[PRODUCTION_AUDIT_P1] {} pattern hit(s){} — codes: {}. \
         Review each before declaring done (these are quality nudges, not blocks).",
        matches.len(),
        if crit > 0 {
            format!(", {crit} critical")
        } else {
            String::new()
        },
        codes.join(", "),
    ));
}
