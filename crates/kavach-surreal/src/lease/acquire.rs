// SPEC: docs/architecture/session-occupancy-lease.md §Acquire (CAS via SurrealDB OCC)
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::types::{AcquireOutcome, LEASE_TTL_SECS, Lease, LeaseRow};
use crate::error::{Error, Result};

/// Attempt to acquire a lease on the given key for the session.
///
/// # Errors
/// Propagates `Error::Surreal` when the database query fails or `Error::RecordNotFound` if the record does not exist.
pub async fn acquire(
    db: &Surreal<Db>,
    table: &str,
    key: &str,
    session_id: &str,
) -> Result<AcquireOutcome> {
    let cur: Option<LeaseRow> = db
        .query("SELECT occupied_by, occupied_until, occupied_epoch FROM type::record($t, $k)")
        .bind(("t", table.to_owned()))
        .bind(("k", key.to_owned()))
        .await
        .map_err(Error::Surreal)?
        .take(0)
        .map_err(Error::Surreal)?;
    let Some(row) = cur else {
        return Err(Error::RecordNotFound(format!("{table}:{key}")));
    };
    let now = Utc::now();
    let prev_epoch: i64 = row.occupied_epoch.map_or(0, |e| e);
    let expired = row.occupied_until.is_none_or(|t| t < now);
    let mine = row.occupied_by.as_deref() == Some(session_id);
    if let Some(holder) = row.occupied_by.as_ref()
        && !expired
        && !mine
    {
        let held_until = row.occupied_until.unwrap_or(now);
        return Ok(AcquireOutcome::HeldBy {
            session_id: holder.clone(),
            expires_at: held_until,
        });
    }
    let next_epoch = prev_epoch.saturating_add(1);
    let lease_until = now
        .checked_add_signed(Duration::seconds(LEASE_TTL_SECS))
        .ok_or_else(|| Error::Migration("lease expiration overflow".to_owned()))?;
    db.query(
        "UPDATE type::record($t, $k) SET \
         occupied_by=$s, occupied_until=$u, occupied_epoch=$e, occupied_heartbeat=$h",
    )
    .bind(("t", table.to_owned()))
    .bind(("k", key.to_owned()))
    .bind(("s", session_id.to_owned()))
    .bind(("u", lease_until))
    .bind(("e", next_epoch))
    .bind(("h", now))
    .await
    .map_err(Error::Surreal)?;
    Ok(AcquireOutcome::Acquired(Lease {
        session_id: session_id.to_owned(),
        epoch: next_epoch,
        expires_at: lease_until,
    }))
}
