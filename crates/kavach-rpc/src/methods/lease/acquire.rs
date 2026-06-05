// SPEC: docs/architecture/session-occupancy-lease.md
// Lease acquire RPC handler — CAS via SurrealDB OCC.
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::lease::{AcquireOutcome, acquire as lease_acquire};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AcquireParams {
    pub table: String,
    pub key: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AcquireResult {
    pub acquired: bool,
    pub session_id: String,
    pub epoch: Option<i64>,
    pub expires_at: DateTime<Utc>,
}

/// Acquires a lease via optimistic concurrency control (OCC) on the `SurrealDB` backend.
///
/// # Errors
/// Returns `Err` if the `SurrealDB` operation fails (e.g., connection loss, transaction conflict).
pub async fn acquire(
    state: &AppState,
    p: AcquireParams,
) -> Result<AcquireResult, ErrorObjectOwned> {
    let outcome = lease_acquire(&state.db, &p.table, &p.key, &p.session_id)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(match outcome {
        AcquireOutcome::Acquired(l) => AcquireResult {
            acquired: true,
            session_id: l.session_id,
            epoch: Some(l.epoch),
            expires_at: l.expires_at,
        },
        AcquireOutcome::HeldBy {
            session_id,
            expires_at,
        } => AcquireResult {
            acquired: false,
            session_id,
            epoch: None,
            expires_at,
        },
    })
}
