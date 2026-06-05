// SPEC: docs/architecture/session-occupancy-lease.md
// RPC handler exposing the lease-state read primitive.
// SOURCE: https://surrealdb.com/3.0
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::lease::status as lease_status;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StatusParams {
    pub table: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StatusResult {
    pub held: bool,
    pub session_id: Option<String>,
    pub epoch: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Returns the current lease status for a given table and key.
///
/// # Errors
///
/// Returns an error if the lease status query fails in the database.
pub async fn status(state: &AppState, p: StatusParams) -> Result<StatusResult, ErrorObjectOwned> {
    let lease = lease_status(&state.db, &p.table, &p.key)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(match lease {
        Some(l) => StatusResult {
            held: true,
            session_id: Some(l.session_id),
            epoch: Some(l.epoch),
            expires_at: Some(l.expires_at),
        },
        None => StatusResult {
            held: false,
            session_id: None,
            epoch: None,
            expires_at: None,
        },
    })
}
