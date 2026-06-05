// bulk.sweep_create — register a new manifest binding N edits to one RCA.
// SOURCE: roadmap.unit.kavach-bulk-mode; pattern: methods/lease/acquire.rs.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::bulk_manifest::{BulkManifest, CreateParams, create as bm_create};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreateRpcParams {
    pub sweep_id: String,
    pub project: String,
    pub root_rca: String,
    pub scope_glob: String,
    pub lint_class: String,
    pub fix_strategy: String,
    pub blast_estimate: i64,
    pub signed_by_session: String,
    pub approved_by: String,
    pub ttl_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreateResult {
    pub sweep_id: String,
    pub project: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub blast_estimate: i64,
}

/// Registers a new bulk manifest in the database.
///
/// # Errors
///
/// Returns an error if the database query fails or if manifest creation is rejected.
pub async fn create(
    state: &AppState,
    p: CreateRpcParams,
) -> Result<CreateResult, ErrorObjectOwned> {
    let m: BulkManifest = bm_create(
        &state.db,
        CreateParams {
            sweep_id: &p.sweep_id,
            project: &p.project,
            root_rca: &p.root_rca,
            scope_glob: &p.scope_glob,
            lint_class: &p.lint_class,
            fix_strategy: &p.fix_strategy,
            blast_estimate: p.blast_estimate,
            signed_by_session: &p.signed_by_session,
            approved_by: &p.approved_by,
            ttl_seconds: p.ttl_seconds,
        },
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(CreateResult {
        sweep_id: m.sweep_id,
        project: m.project,
        status: m.status,
        expires_at: m.expires_at,
        blast_estimate: m.blast_estimate,
    })
}
