// SPEC: docs/architecture/session-occupancy-lease.md — batch all-or-nothing acquire.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::lease::{AcquireSetOutcome, acquire_set as lease_acquire_set};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AcquireSetParams {
    pub table: String,
    pub keys: Vec<String>,
    pub session_id: String,
}

/// One acquired lease in a successful batch, in input order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AcquiredLease {
    pub epoch: i64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AcquireSetResult {
    pub all_acquired: bool,
    pub session_id: String,
    pub leases: Vec<AcquiredLease>,
    /// First contended key + its holder when `all_acquired` is false; empty otherwise.
    pub conflict_key: String,
    pub held_by: String,
}

/// Atomically reserve a SET of keys for one session (all-or-nothing).
///
/// # Errors
/// Returns `Err` if the `SurrealDB` operation fails.
pub async fn acquire_set(
    state: &AppState,
    p: AcquireSetParams,
) -> Result<AcquireSetResult, ErrorObjectOwned> {
    let refs: Vec<&str> = p.keys.iter().map(String::as_str).collect();
    let outcome = lease_acquire_set(&state.db, &p.table, &refs, &p.session_id)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(match outcome {
        AcquireSetOutcome::AllAcquired(leases) => AcquireSetResult {
            all_acquired: true,
            session_id: p.session_id,
            leases: leases
                .into_iter()
                .map(|l| AcquiredLease {
                    epoch: l.epoch,
                    expires_at: l.expires_at,
                })
                .collect(),
            conflict_key: String::new(),
            held_by: String::new(),
        },
        AcquireSetOutcome::Conflict {
            conflict_key,
            held_by,
        } => AcquireSetResult {
            all_acquired: false,
            session_id: p.session_id,
            leases: Vec::new(),
            conflict_key,
            held_by,
        },
    })
}
