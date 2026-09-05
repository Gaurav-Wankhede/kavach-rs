use crate::turso::TursoStore;
// SPEC: docs/architecture/session-occupancy-lease.md §Acquire (CAS via SQL UPDATE)
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
use super::types::{AcquireOutcome, LEASE_TTL_SECS, Lease, LeaseRow};
use crate::error::{Error, Result};
use chrono::{Duration, Utc};
/// Attempt to acquire a lease on the given key for the session.
///
/// # Errors
/// Propagates `Error::Turso` when the database query fails or `Error::RecordNotFound` if the record does not exist.
pub async fn acquire(
    db: &TursoStore,
    table: &str,
    key: &str,
    session_id: &str,
) -> Result<AcquireOutcome> {
    let now = Utc::now();
    let lease_until = now
        .checked_add_signed(Duration::seconds(LEASE_TTL_SECS))
        .ok_or_else(|| Error::Migration("lease expiration overflow".to_owned()))?;
    let now_s = now.to_rfc3339();
    let until_s = lease_until.to_rfc3339();
    // ATOMIC acquire. A single conditional UPDATE is the compare-and-set: the
    // claim is applied ONLY when the lease is unheld, already expired, or
    // already ours. The previous SELECT-then-UPDATE was a TOCTOU window — two
    // acquires could both read "free" and both write, last-writer-wins, each
    // returning a different epoch for the same key. Here SQLite evaluates the
    // WHERE against row state at write time, so only one racer's UPDATE matches.
    // `occupied_epoch = COALESCE(occupied_epoch, 0) + 1` bumps the fencing token
    // in the same statement; RETURNING gives back the post-write epoch.
    let mut updated = db
        .exec_params(
            "UPDATE memories SET \
             occupied_by = $s, occupied_until = $u, \
             occupied_epoch = COALESCE(occupied_epoch, 0) + 1, occupied_heartbeat = $h \
             WHERE category = $t AND entry_key = $k \
               AND (occupied_by IS NULL OR occupied_until IS NULL OR occupied_until < $now \
                    OR occupied_by = $s) \
             RETURNING occupied_by, occupied_until, occupied_epoch",
            libsql::params![
                session_id.to_owned(),
                until_s,
                now_s.clone(),
                table.to_owned(),
                key.to_owned(),
                now_s
            ],
        )
        .await?;
    let won = match updated.next().await? {
        Some(row) => Some(LeaseRow {
            occupied_by: row.get::<Option<String>>(0)?,
            occupied_until: row.get::<Option<String>>(1)?,
            occupied_epoch: row.get::<Option<i64>>(2)?,
        }),
        None => None,
    };
    if let Some(won) = won {
        return Ok(AcquireOutcome::Acquired(Lease {
            session_id: session_id.to_owned(),
            epoch: won.occupied_epoch.unwrap_or(1),
            expires_at: won.expires_at().unwrap_or(lease_until),
        }));
    }
    // The CAS matched no row: either the record is absent, or it is validly held
    // by another session. Read once to disambiguate and report the true holder.
    let mut cur = db
        .exec_params(
            "SELECT occupied_by, occupied_until, occupied_epoch FROM memories \
             WHERE category = $t AND entry_key = $k",
            libsql::params![table.to_owned(), key.to_owned()],
        )
        .await?;
    let row = match cur.next().await? {
        Some(row) => LeaseRow {
            occupied_by: row.get::<Option<String>>(0)?,
            occupied_until: row.get::<Option<String>>(1)?,
            occupied_epoch: row.get::<Option<i64>>(2)?,
        },
        None => return Err(Error::RecordNotFound(format!("{table}:{key}"))),
    };
    // The row exists but the CAS did not match — it must be validly held, so
    // `occupied_by` is non-NULL. A NULL here is schema corruption or a torn write;
    // returning an empty session_id would forge a fake holder that no lease check
    // matches. Fail closed so the corruption surfaces instead of leaking a bogus lease.
    let expires_at = row.expires_at().unwrap_or(now);
    let holder = row.occupied_by.ok_or_else(|| {
        Error::SchemaViolation(format!("{table}:{key} exists but occupied_by is NULL"))
    })?;
    Ok(AcquireOutcome::HeldBy {
        session_id: holder,
        expires_at,
    })
}
#[cfg(test)]
#[path = "acquire_test.rs"]
mod tests;
