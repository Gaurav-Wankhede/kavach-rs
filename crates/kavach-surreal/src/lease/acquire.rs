// SPEC: docs/architecture/session-occupancy-lease.md §Acquire (CAS via SurrealDB OCC)
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

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
    let now = Utc::now();
    let lease_until = now
        .checked_add_signed(Duration::seconds(LEASE_TTL_SECS))
        .ok_or_else(|| Error::Migration("lease expiration overflow".to_owned()))?;
    // ATOMIC acquire. A single conditional UPDATE is the compare-and-set: the
    // claim is applied ONLY when the lease is unheld, already expired, or
    // already ours. The previous SELECT-then-UPDATE was a TOCTOU window — two
    // acquires could both read "free" and both write, last-writer-wins, each
    // returning a different epoch for the same key. Here SurrealDB evaluates the
    // WHERE against row state at write time, so only one racer's UPDATE matches.
    // `occupied_epoch = (occupied_epoch ?? 0) + 1` bumps the fencing token in the
    // same statement; RETURN gives back the post-write epoch for the winner.
    let updated: Option<LeaseRow> = db
        .query(
            "UPDATE type::record($t, $k) SET \
             occupied_by=$s, occupied_until=$u, \
             occupied_epoch=(occupied_epoch ?? 0) + 1, occupied_heartbeat=$h \
             WHERE occupied_by=NONE OR occupied_until=NONE OR occupied_until < $now \
                   OR occupied_by=$s \
             RETURN occupied_by, occupied_until, occupied_epoch",
        )
        .bind(("t", table.to_owned()))
        .bind(("k", key.to_owned()))
        .bind(("s", session_id.to_owned()))
        .bind(("u", lease_until))
        .bind(("h", now))
        .bind(("now", now))
        .await
        .map_err(Error::Surreal)?
        .take(0)
        .map_err(Error::Surreal)?;
    if let Some(won) = updated {
        return Ok(AcquireOutcome::Acquired(Lease {
            session_id: session_id.to_owned(),
            epoch: won.occupied_epoch.unwrap_or(1),
            expires_at: won.occupied_until.unwrap_or(lease_until),
        }));
    }
    // The CAS matched no row: either the record is absent, or it is validly held
    // by another session. Read once to disambiguate and report the true holder.
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
    // The row exists but the CAS did not match — it must be validly held, so
    // `occupied_by` is non-NULL. A NULL here is schema corruption or a torn write;
    // returning an empty session_id would forge a fake holder that no lease check
    // matches. Fail closed so the corruption surfaces instead of leaking a bogus lease.
    let holder = row.occupied_by.ok_or_else(|| {
        Error::SchemaViolation(format!("{table}:{key} exists but occupied_by is NULL"))
    })?;
    Ok(AcquireOutcome::HeldBy {
        session_id: holder,
        expires_at: row.occupied_until.unwrap_or(now),
    })
}

#[cfg(test)]
#[path = "acquire_test.rs"]
mod tests;
