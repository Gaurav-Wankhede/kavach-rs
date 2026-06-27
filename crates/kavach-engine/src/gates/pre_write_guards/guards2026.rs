//! 2026 guards runner — preserves the original interleaved dispatch order so a
//! P0 block carries exactly the advisories pushed before it. Advisory-only
//! detectors live in `advise`; severity-routed detectors (`rust_196`, `async_sync`,
//! `database_ops`, pii, migration, webhook) live in the `severity` submodule. This
//! hub interleaves them in source order and returns the first block reason.
mod advise;
mod bloatware;
mod dedup;
mod severity;
use super::result::Acc;
use crate::gates::pre_write_context::WriteContext;
/// Run the 2026 guard block in the original order. `Some(reason)` blocks.
pub(super) fn check(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    if ctx.is_test {
        return None;
    }
    advise::solid_dsa_design(ctx, acc);
    advise::atomic_ui(ctx, acc);
    if let Some(b) = severity::rust_196(ctx, acc) {
        return Some(b);
    }
    advise::dioxus_axum(ctx, acc);
    if let Some(b) = severity::async_sync(ctx, acc) {
        return Some(b);
    }
    advise::api_mgmt_design(ctx, acc);
    if let Some(b) = severity::database_ops(ctx, acc) {
        return Some(b);
    }
    advise::observability_finops(ctx, acc);
    if let Some(b) = severity::pii(ctx, acc) {
        return Some(b);
    }
    if let Some(b) = severity::migration(ctx, acc) {
        return Some(b);
    }
    if let Some(b) = dedup::dedup(ctx, acc) {
        return Some(b);
    }
    if let Some(b) = bloatware::bloatware(ctx, acc) {
        return Some(b);
    }
    severity::webhook(ctx, acc)
}
#[cfg(test)]
#[path = "guards2026_test.rs"]
#[cfg(test)]
#[path = "guards2026_test.rs"]
mod tests;
