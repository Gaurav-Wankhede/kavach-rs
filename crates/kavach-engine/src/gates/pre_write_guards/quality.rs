//! P1 quality advisories — never block, push findings into the accumulator.
//! Rust/TS production guards, banned-CSS, UX, and complexity. (Platform
//! response/infra/api-gateway advisories live in the hub so the P0 microservice
//! block can sit between them, matching the original dispatch order.)
use super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// Rust + TypeScript production-code quality advisories (P1).
pub(super) fn lang(ctx: &WriteContext<'_>, acc: &mut Acc) {
    // Rust production guard — P1 (quality advisory, not security block).
    // Per Meta-Harness: complex verification degrades performance.
    // unwrap/todo are quality issues, not security vulnerabilities.
    if ctx.is_rust
        && !ctx.is_test
        && let Some(msg) = super::super::pre_write_rust_guard::check(ctx.file_path, ctx.content)
    {
        acc.p1_advisories.push(format!("[RUST_GUARD_P1] {msg}"));
    }
    // TypeScript production guard — P1 (quality advisory)
    if ctx.is_frontend && !ctx.is_test {
        if let Some(msg) = super::super::pre_write_ts_guard::check(ctx.file_path, ctx.content) {
            acc.p1_advisories.push(format!("[TS_GUARD_P1] {msg}"));
        }
        if let Some(msg) =
            super::super::pre_write_ts_guard::check_component_oversized(ctx.file_path, ctx.content)
        {
            acc.p1_advisories.push(format!("[TS_MONOLITH_P1] {msg}"));
        }
    }
}

/// Frontend (CSS/UX) + complexity advisories (P1).
pub(super) fn presentation(ctx: &WriteContext<'_>, acc: &mut Acc) {
    if ctx.is_frontend && !ctx.is_test {
        if let Some(msg) = kavach_patterns::banned_css_guard::check(ctx.file_path, ctx.content) {
            acc.p1_advisories.push(format!("[CSS_GUARD_P1] {msg}"));
        }
        if let Some(msg) = kavach_patterns::ux_guard::check(ctx.file_path, ctx.content) {
            acc.p1_advisories.push(format!("[UX_GUARD_P1] {msg}"));
        }
    }
    if !ctx.is_test
        && let Some(msg) =
            kavach_patterns::complexity_guard::check(ctx.file_path, &ctx.effective_content)
    {
        acc.p1_advisories.push(format!("[COMPLEXITY_P1] {msg}"));
    }
}
