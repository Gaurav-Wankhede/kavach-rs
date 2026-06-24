// bulk.sweep_get — fetch one manifest by sweep_id. Used by pre-write gate
// to verify a sweep is still usable + matches scope before allowing an Edit.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::bulk_manifest::{BulkManifest, get as bm_get};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at RPC handler boundary from BulkManifest"
)]
pub struct GetResult {
    pub sweep_id: String,
    pub project: String,
    pub scope_glob: String,
    pub lint_class: String,
    pub fix_strategy: String,
    pub blast_estimate: i64,
    pub expires_at: DateTime<Utc>,
    pub conformance_applied: i64,
    pub conformance_refused: i64,
    pub conformance_drifted: i64,
    pub status: String,
}

impl From<BulkManifest> for GetResult {
    fn from(m: BulkManifest) -> Self {
        Self {
            sweep_id: m.sweep_id,
            project: m.project,
            scope_glob: m.scope_glob,
            lint_class: m.lint_class,
            fix_strategy: m.fix_strategy,
            blast_estimate: m.blast_estimate,
            expires_at: m.expires_at,
            conformance_applied: m.conformance_applied,
            conformance_refused: m.conformance_refused,
            conformance_drifted: m.conformance_drifted,
            status: m.status,
        }
    }
}

/// Fetch one manifest by `sweep_id`.
///
/// # Errors
///
/// Returns an RPC error if the database query fails.

