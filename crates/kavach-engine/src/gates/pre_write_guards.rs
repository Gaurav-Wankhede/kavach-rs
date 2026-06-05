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
mod microfile;
mod quality;
mod result;
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

    // Ordered guard chain; the first `Some(reason)` blocks and short-circuits.
    let block = chain::check(ctx, input, session, &mut runner)
        .or_else(|| {
            quality::lang(ctx, &mut acc); // P1 rust/ts
            security::check(ctx) // P0 sql/db/owasp/silent/crypto/gnap/frontend-sec
        })
        .or_else(|| {
            quality::presentation(ctx, &mut acc); // P1 css/ux/complexity
            microfile::micro_file(ctx, &mut acc) // P0/P1 micro-file
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
    if let Some(block) = microfile::microservice(ctx) {
        return Some(block);
    }
    if let Some(msg) = super::pre_write_infra_guard::check(ctx.file_path, ctx.content) {
        acc.p1_advisories.push(format!("[INFRA_P1] {msg}"));
    }
    if let Some(msg) = super::api_gateway_guard::check(ctx.file_path, ctx.content) {
        acc.p1_advisories.push(format!("[API_GATEWAY_P1] {msg}"));
    }
    None
}
