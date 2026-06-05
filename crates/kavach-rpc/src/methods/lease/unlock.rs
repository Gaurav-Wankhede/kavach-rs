// SPEC: docs/architecture/session-occupancy-lease.md
// Lease unlock RPC — clear lease fields when holder is done.
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::lease::{Lease, unlock as lease_unlock};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnlockParams {
    pub table: String,
    pub key: String,
    pub session_id: String,
    pub epoch: i64,
    pub expires_at: DateTime<Utc>,
}

impl fmt::Debug for UnlockParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnlockParams")
            .field("table", &self.table)
            .field("key", &"<redacted>")
            .field("session_id", &"<redacted>")
            .field("epoch", &self.epoch)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnlockResult {
    pub ok: bool,
}

/// Unlock a lease for a session.
///
/// # Errors
///
/// Returns an error if the lease unlock operation fails in the database.
pub async fn unlock(state: &AppState, p: UnlockParams) -> Result<UnlockResult, ErrorObjectOwned> {
    let lease = Lease {
        session_id: p.session_id,
        epoch: p.epoch,
        expires_at: p.expires_at,
    };
    lease_unlock(&state.db, &p.table, &p.key, &lease)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(UnlockResult { ok: true })
}
