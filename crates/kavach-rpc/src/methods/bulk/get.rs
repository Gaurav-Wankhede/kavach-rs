// GetResult — manifest DTO shared by bulk.sweep_list_active (list.rs).
use chrono::{DateTime, Utc};
use kavach_surreal::bulk_manifest::BulkManifest;
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
