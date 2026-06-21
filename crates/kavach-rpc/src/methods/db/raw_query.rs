//! `db.raw_query` RPC method — run an operator-supplied read-only `SurrealQL` query.
//!
//! `SELECT`/`INFO` only; the read-only guard lives in `kavach_surreal::graph_raw_select`
//! (`SurrealDB` 3.x has no native readonly tx). SOURCE: decision.raw-select-cli.

use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::graph_raw_select;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RawQueryParams {
    /// The read-only `SurrealQL` to execute (`SELECT`/`INFO` only).
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
        .map_err(surreal_to_rpc)?;
    Ok(RawQueryResult { json })
}
