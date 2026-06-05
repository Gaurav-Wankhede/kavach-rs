//! Break-glass + bulk-sweep env escapes from the RCA gate. Both are obvious-by-
//! naming with an audit trail per AWS well-architected break-glass guidance.

/// Break-glass env var for `kavach` maintainers editing the gate itself.
/// USE: `KAVACH_BYPASS_RCA=1 cargo build --release` when wiring post-tool hooks
/// that set `mark_rca_present()`. Bootstrap-only — never for normal work.
/// SOURCE: `aws.amazon.com/wellarchitected/.../ag.sad.5` — break-glass procedures
///         require obvious naming + audit trail.
/// SOURCE: `https://hoop.dev` — every break-glass use must emit a structured audit event.
pub(super) const BYPASS_ENV: &str = "KAVACH_BYPASS_RCA";

/// Bulk-mode active sweep id. When set, the manifest (created at sweep boundary
/// via `kavach bulk start` after explicit user approval) carries the SHARED RCA
/// `+` `scope_glob` `+` `fix_strategy`. Per-Edit gate then skips the per-Edit RCA
/// demand and lets post-write emit a `bulk_apply` event tagged with this `sweep_id`.
/// `SOURCE`: `roadmap.unit.kavach-bulk-mode` (four-layer agentic pattern).
pub(super) const BULK_SWEEP_ENV: &str = "KAVACH_BULK_SWEEP_ID";

/// True when bypass env var is set to "1".
pub(super) fn bypass_active() -> bool {
    std::env::var(BYPASS_ENV).ok().as_deref() == Some("1")
}

/// Returns `Some(sweep_id)` when an active bulk-mode sweep authorizes this edit.
/// The `sweep_id` is opaque here; daemon-side post-write event-emission verifies
/// the manifest still exists + the file matches `scope_glob` + the diff matches
/// `fix_strategy`. Audit trail = `bulk_apply` event per Edit tagged with `sweep_id`.
pub(super) fn active_bulk_sweep() -> Option<String> {
    std::env::var(BULK_SWEEP_ENV).ok().filter(|s| !s.is_empty())
}
