//! P0 security guards — each blocks on hit. SQL injection, DB security (RLS /
//! SET LOCAL / UPDATE-without-WHERE / SQL concat), OWASP injection, silent
//! failures, banned crypto, GNAP/OAuth, and frontend security (CSP / Tailwind
//! arbitrary XSS). All skipped on test files. Returns the first block reason.
use crate::gates::pre_write_context::WriteContext;

/// Run the P0 security guard group. Returns `Some(reason)` to block.
pub(super) fn check(ctx: &WriteContext<'_>) -> Option<String> {
    if ctx.is_test {
        return None;
    }

    // SQL production guard — P0 (security: SQL injection)
    if let Some(msg) = super::super::pre_write_sql_guard::check(ctx.file_path, ctx.content) {
        return Some(msg);
    }
    // DB security guard — P0 (RLS, SET LOCAL, UPDATE without WHERE, SQL concat)
    if let Some(msg) = kavach_patterns::db_security_guard::check(ctx.file_path, ctx.content) {
        return Some(msg);
    }
    // OWASP guard — P0 (injection attacks)
    if let Some(msg) = kavach_patterns::owasp_guard::check(ctx.file_path, ctx.content) {
        return Some(msg);
    }
    // Silent-failure guard — P0 (let _ = Result-expr, .map_err(|_|), let _ = lock())
    // SOURCE: doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html (let_underscore_drop)
    // SOURCE: doc.rust-lang.org/rustc/lints/listing/deny-by-default.html (let_underscore_lock)
    if let Some(msg) = kavach_patterns::silent_io_guard::check(ctx.file_path, ctx.content) {
        return Some(msg);
    }
    // Crypto guard — P0 (banned algorithms)
    if let Some(msg) = kavach_patterns::crypto_guard::check(ctx.file_path, ctx.content) {
        return Some(msg);
    }
    // GNAP guard — P0 (OAuth/Bearer patterns banned, enforce GNAP + httpsig)
    if let Some(msg) = kavach_patterns::gnap_guard::check(ctx.file_path, ctx.content) {
        return Some(msg);
    }
    // Frontend security guard — P0 (CSP, Tailwind arbitrary XSS)
    if ctx.is_frontend
        && let Some(msg) =
            kavach_patterns::frontend_security_guard::check(ctx.file_path, ctx.content)
    {
        return Some(msg);
    }
    // Rust production guard — P0 (unwrap, panic, todo, etc.)
    if ctx.is_rust {
        if let Some(msg) = super::super::pre_write_rust_guard::check(ctx.file_path, ctx.content) {
            return Some(msg);
        }
    }
    // TypeScript production guard — P0 (as any, hardcoded URLs, mock data, XSS)
    if ctx.is_frontend {
        if let Some(msg) = super::super::pre_write_ts_guard::check(ctx.file_path, ctx.content) {
            return Some(msg);
        }
    }
    None
}
