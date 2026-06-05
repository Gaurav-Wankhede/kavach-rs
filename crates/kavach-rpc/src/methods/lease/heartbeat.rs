// SPEC: docs/architecture/session-occupancy-lease.md
// Lease heartbeat RPC — epoch-guarded TTL renewal (fencing-token check).
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::lease::{Lease, heartbeat as lease_heartbeat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HeartbeatParams {
    pub table: String,
    pub key: String,
    pub session_id: String,
    pub epoch: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HeartbeatResult {
    pub renewed: bool,
    pub expires_at: DateTime<Utc>,
}

/// Renews a lease heartbeat with epoch-guarded TTL.
///
/// # Errors
///
/// Returns an RPC error if the database operation fails or the lease cannot be renewed.
pub async fn heartbeat(
    state: &AppState,
    p: HeartbeatParams,
) -> Result<HeartbeatResult, ErrorObjectOwned> {
    let lease = Lease {
        session_id: p.session_id,
        epoch: p.epoch,
        expires_at: p.expires_at,
    };
    let renewed = lease_heartbeat(&state.db, &p.table, &p.key, &lease)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(HeartbeatResult {
        renewed: true,
        expires_at: renewed.expires_at,
    })
}
