//! `db.raw_query` RPC method — run an operator-supplied read-only `DatabaseQL` query.
//!
//! `SELECT`/`INFO` only; the read-only guard lives in `kavach_db::graph_raw_select`
//! (`Database` 3.x has no native readonly tx). SOURCE: decision.raw-select-cli.

use crate::error::db_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_db::graph_raw_select;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RawQueryParams {
    /// The read-only `DatabaseQL` to execute (`SELECT`/`INFO` only).
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RawQueryResult {
    /// Result rows as a pretty-printed JSON string.
    pub json: String,
}

/// # Errors
/// Returns an RPC error if the query is non-read-only, empty, or fails.
pub async fn raw_query(
    ctx: &AppState,
    params: RawQueryParams,
) -> Result<RawQueryResult, ErrorObjectOwned> {
    let json = graph_raw_select(&ctx.db, &params.query)
        .await
        .map_err(db_to_rpc)?;
    Ok(RawQueryResult { json })
}
