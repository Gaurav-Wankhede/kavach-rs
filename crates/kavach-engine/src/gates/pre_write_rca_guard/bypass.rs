//! Bulk-sweep env escape from the RCA gate. The manifest (created at sweep boundary
//! via `kavach bulk start` after explicit user approval) carries the SHARED RCA
//! `+` `scope_glob` `+` `fix_strategy`. Per-Edit gate then skips the per-Edit RCA
//! demand and lets post-write emit a `bulk_apply` event tagged with this `sweep_id`.
//! `SOURCE`: `roadmap.unit.kavach-bulk-mode` (four-layer agentic pattern).

/// Bulk-mode active sweep id. When set, the manifest (created at sweep boundary
/// via `kavach bulk start` after explicit user approval) carries the SHARED RCA
/// `+` `scope_glob` `+` `fix_strategy`. Per-Edit gate then skips the per-Edit RCA
/// demand and lets post-write emit a `bulk_apply` event tagged with this `sweep_id`.
/// `SOURCE`: `roadmap.unit.kavach-bulk-mode` (four-layer agentic pattern).
pub(super) const BULK_SWEEP_ENV: &str = "KAVACH_BULK_SWEEP_ID";

/// Returns `Some(sweep_id)` when an active bulk-mode sweep authorizes this edit.
/// The `sweep_id` is opaque here; daemon-side post-write event-emission verifies
/// the manifest still exists + the file matches `scope_glob` + the diff matches
/// `fix_strategy`. Audit trail = `bulk_apply` event per Edit tagged with `sweep_id`.
pub(super) fn active_bulk_sweep() -> Option<String> {
    std::env::var(BULK_SWEEP_ENV).ok().filter(|s| !s.is_empty())
}
