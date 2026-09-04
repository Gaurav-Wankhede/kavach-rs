// Types + pure helpers for bulk_manifest. No I/O — keeps the DB ops files
// small and the type surface unit-testable without a live Surreal handle.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb_types::{RecordId, SurrealValue};

pub(super) const STATUS_ACTIVE: &str = "active";
pub(super) const STATUS_CLOSED: &str = "closed";
pub(super) const STATUS_EXPIRED: &str = "expired";

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct BulkManifest {
    pub id: Option<RecordId>,
    pub sweep_id: String,
    pub project: String,
    pub root_rca: String,
    pub scope_glob: String,
    pub lint_class: String,
    pub fix_strategy: String,
    pub blast_estimate: i64,
    pub signed_by_session: String,
    pub approved_by: String,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub conformance_applied: i64,
    pub conformance_refused: i64,
    pub conformance_drifted: i64,
    pub status: String,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc/cli); non_exhaustive => E0639"
)]
pub struct CreateParams<'a> {
    pub sweep_id: &'a str,
    pub project: &'a str,
    pub root_rca: &'a str,
    pub scope_glob: &'a str,
    pub lint_class: &'a str,
    pub fix_strategy: &'a str,
    pub blast_estimate: i64,
    pub signed_by_session: &'a str,
    pub approved_by: &'a str,
    pub ttl_seconds: i64,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ConformanceField {
    Applied,
    Refused,
    Drifted,
}

/// Pure helper — true iff manifest still usable (active + not past TTL).
#[must_use]
pub fn is_usable(m: &BulkManifest, now: DateTime<Utc>) -> bool {
    m.status == STATUS_ACTIVE && now < m.expires_at
}
