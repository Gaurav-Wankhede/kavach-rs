// JSON-RPC handlers for durable harness runtime state (session_runtime table).
// Used by kavach-session load/save so SessionState survives /clear + context
// compaction, keyed by session_id — no cross-session rehydration drift.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct SessionGetParams {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct SessionUpsertParams {
    pub session_id: String,
    pub workdir: String,
    /// The full `SessionState` serialized as one string (the existing INI
    /// text) — the schema stays stable however many fields `SessionState` grows.
    pub state_blob: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct SessionGetResult {
    pub session_id: String,
    pub workdir: String,
    pub state_blob: String,
}

/// Fetch the runtime row for exactly this `session_id`.
/// `None` ⇒ no row — the caller starts fresh, never inherits another session.
///
/// # Errors
///
/// Returns an error if the database query fails (connection, timeout, or query execution error).
pub async fn get(
    state: &AppState,
    params: SessionGetParams,
) -> Result<Option<SessionGetResult>, ErrorObjectOwned> {
    let row = kavach_surreal::session_get_by_id(&state.db, &params.session_id)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(row.map(|r| SessionGetResult {
        session_id: r.session_id,
        workdir: r.workdir,
        state_blob: r.state_blob,
    }))
}

/// Idempotent write-through of a session's runtime state (one row per
/// `session_id` via the UNIQUE index).
///
/// # Errors
///
/// Returns an error if the database upsert operation fails (connection, timeout, or query execution error).
pub async fn upsert(
    state: &AppState,
    params: SessionUpsertParams,
) -> Result<bool, ErrorObjectOwned> {
    kavach_surreal::session_upsert(
        &state.db,
        &params.session_id,
        &params.workdir,
        &params.state_blob,
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(true)
}
