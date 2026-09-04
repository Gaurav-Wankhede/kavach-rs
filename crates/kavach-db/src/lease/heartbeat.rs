// SPEC: docs/architecture/session-occupancy-lease.md §Heartbeat (epoch-guarded fencing-token check)
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

use super::types::{LEASE_TTL_SECS, Lease, LeaseRow};
use crate::error::{Error, Result};

/// Refreshes a lease heartbeat and expiration time if the epoch and session match.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails, or `Error::Migration` if the lease was preempted by another session.
pub async fn heartbeat(db: &Surreal<Db>, table: &str, key: &str, lease: &Lease) -> Result<Lease> {
    let now = Utc::now();
    let expires = now
        .checked_add_signed(Duration::seconds(LEASE_TTL_SECS))
        .ok_or_else(|| Error::Migration("lease expiration overflow".to_owned()))?;
    let updated: Option<LeaseRow> = db
        .query(
            "UPDATE type::record($t, $k) SET occupied_heartbeat=$h, occupied_until=$u \
             WHERE occupied_by=$s AND occupied_epoch=$e \
             RETURN occupied_by, occupied_until, occupied_epoch",
        )
        .bind(("t", table.to_owned()))
        .bind(("k", key.to_owned()))
        .bind(("h", now))
        .bind(("u", expires))
        .bind(("s", lease.session_id.clone()))
        .bind(("e", lease.epoch))
        .await
        .map_err(Error::Surreal)?
        .take(0)
        .map_err(Error::Surreal)?;
    match updated {
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
