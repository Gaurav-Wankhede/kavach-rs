// Stage 3: Language guards — chain verification + tiered severity system.
// ARCH: TieredGuardSeverity — per Meta-Harness research, complex verification degrades performance
// PATTERN: tiered_severity | SCOPE: pre_write | CAP: AP | SEARCHED: 2026-04
// P0 (block): Security-critical. P1 (advisory): Quality. P2 (silent): Style.
//
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
mod tdd_guard;
mod retired_pattern;
mod security;

#[cfg(test)]
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

    // TDD runs FIRST: no production code lands without a test-first (Red) signal.
    if let Some(block) = tdd_guard::check(ctx, session) {
        return GuardResult {
            block: Some(block),
            algo_advisory: None,
            runner_compact: runner.to_compact(),
            p1_advisories: Vec::new(),
        };
    }

    // Internet-first RESOLVES, never blocks: drives the lookup + attaches a P1
    // advisory. The `[RESEARCH_FIRST]` Stop teeth enforce citation before the turn ends.
    if let Some(advisory) = research_consume::check(ctx, session) {
        acc.p1_advisories.push(advisory);
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
