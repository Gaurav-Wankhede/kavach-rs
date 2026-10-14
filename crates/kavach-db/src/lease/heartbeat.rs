use crate::db::DbStore;
// SPEC: docs/architecture/session-occupancy-lease.md §Heartbeat (epoch-guarded fencing-token check)
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
use chrono::{Duration, Utc};

use super::types::{LEASE_TTL_SECS, Lease};
use crate::error::{Error, Result};

/// Refreshes a lease heartbeat and expiration time if the epoch and session match.
///
/// # Errors
/// Propagates `Error::Database` when the query fails, or `Error::Migration` if the lease was preempted by another session.
pub async fn heartbeat(db: &DbStore, table: &str, key: &str, lease: &Lease) -> Result<Lease> {
    let now = Utc::now();
    let expires = now
        .checked_add_signed(Duration::seconds(LEASE_TTL_SECS))
        .ok_or_else(|| Error::Migration("lease expiration overflow".to_owned()))?;
    let now_s = now.to_rfc3339();
    let expires_s = expires.to_rfc3339();
    let mut updated = db
        .exec_params(
            "UPDATE memories SET occupied_heartbeat = $h, occupied_until = $u \
             WHERE category = $t AND entry_key = $k \
               AND occupied_by = $s AND occupied_epoch = $e \
             RETURNING occupied_by, occupied_until, occupied_epoch",
            libsql::params![
                now_s,
                expires_s,
                table.to_owned(),
                key.to_owned(),
                lease.session_id.clone(),
                lease.epoch
            ],
        )
        .await?;
    match updated.next().await? {
        Some(_) => Ok(Lease {
            session_id: lease.session_id.clone(),
            epoch: lease.epoch,
            expires_at: expires,
        }),
        None => Err(Error::Migration(format!(
            "lease preempted: {}:{} epoch {}",
            table, key, lease.epoch
        ))),
    }
}
