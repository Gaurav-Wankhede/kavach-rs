// SPEC: docs/architecture/session-occupancy-lease.md §Renewal (liveness janitor)
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::types::LEASE_TTL_SECS;
use crate::error::{Error, Result};
use surrealdb_types::SurrealValue;

/// The leased tables a renewal sweep covers. Mirrors `recovery::LEASED_TABLES`;
/// any table whose cards carry the `occupied_*` lease columns belongs here.
const LEASED_TABLES: &[&str] = &["roadmap", "decision", "app_spec"];

/// The renewal cadence: renew at one third of the TTL so two consecutive misses
/// (e.g. a paused daemon) are tolerated before a lease actually lapses. 300s TTL
/// → renew every 100s.
#[expect(
    clippy::integer_division,
    reason = "intentional floor: TTL/3 cadence, exact remainder irrelevant"
)]
pub const RENEW_INTERVAL_SECS: u64 = (LEASE_TTL_SECS as u64) / 3;

/// Extend the expiry of every lease that is STILL actively held and whose card
/// is STILL in progress, across all leased tables. Returns the number of leases
/// renewed this sweep.
///
/// This is the liveness counterpart to the auto-expiry TTL: a crashed holder's
/// lease still lapses (this sweep skips it once its card stops being
/// `in_progress`, or once the holder process is gone and the row's status is
/// closed), but a LIVE session working a card longer than the TTL keeps its
/// claim because each sweep pushes `occupied_until` forward.
///
/// The renewal is driven entirely by DB state — it holds no in-memory registry
/// of claims — so it is restart-safe by construction: a freshly spawned daemon
/// resumes renewing exactly the leases the DB says are live, with no re-arming
/// bookkeeping. The `WHERE` guarantees the three card requirements:
/// - stops renewing a finished card: `entry_status = 'in_progress'` excludes
///   `done`/`verified`/`todo`,
/// - stops renewing a released lease: `occupied_by != NONE` excludes unlocked
///   rows,
/// - never resurrects a lapsed-and-restolen lease: only rows whose CURRENT
///   `occupied_until` is in the future are pushed forward; a lease that already
///   expired is left for the legitimate next acquirer (it is NOT renewed back
///   to life under the old holder).
///
/// # Errors
/// Propagates `Error::Surreal` when a table's UPDATE fails, or `Error::Migration`
/// on expiry arithmetic overflow.
pub async fn renew_active_leases(db: &Surreal<Db>) -> Result<usize> {
    let now = Utc::now();
    let expires = now
        .checked_add_signed(Duration::seconds(LEASE_TTL_SECS))
        .ok_or_else(|| Error::Migration("lease renewal expiration overflow".to_owned()))?;
    let mut renewed = 0_usize;
    for table in LEASED_TABLES {
        renewed = renewed.saturating_add(renew_table(db, table, now, expires).await?);
    }
    Ok(renewed)
}

/// Renew the live leases of a single table. Counts the rows actually extended.
async fn renew_table(
    db: &Surreal<Db>,
    table: &str,
    now: chrono::DateTime<Utc>,
    expires: chrono::DateTime<Utc>,
) -> Result<usize> {
    let resp = db
        .query(
            "UPDATE type::table($t) SET occupied_until=$u, occupied_heartbeat=$h \
             WHERE occupied_by != NONE AND occupied_until != NONE AND occupied_until > $now \
                   AND entry_status = 'in_progress' \
             RETURN id",
        )
        .bind(("t", table.to_owned()))
        .bind(("u", expires))
        .bind(("h", now))
        .bind(("now", now))
        .await;
    // SurrealDB defers a "table does not exist" error to result extraction, not
    // the query future — so the tolerance check must wrap BOTH .await and
    // .take(). A leased table absent on a fresh store holds no renewable lease:
    // a 0-row no-op, not a sweep-aborting failure.
    let taken = match resp {
        Ok(mut r) => r.take::<Vec<RenewedIdRow>>(0),
        Err(e) => Err(e),
    };
    match taken {
        Ok(updated) => Ok(updated.len()),
        Err(e) if is_missing_table(&e) => Ok(0),
        Err(e) => Err(Error::Surreal(e)),
    }
}

/// True when the error is `SurrealDB`'s "table does not exist" — the only error
/// `renew_table` swallows (an absent leased table simply holds no renewable
/// lease). Matched on the rendered message because the typed `NotFound` variant
/// is not surfaced through the SDK error tree at this version.
fn is_missing_table(e: &surrealdb::Error) -> bool {
    // Display renders "The table 'X' does not exist"; Debug wraps it in
    // NotFound(Table{..}). Match the stable phrase common to both.
    let msg = e.to_string();
    msg.contains("does not exist")
}

#[derive(surrealdb_types::SurrealValue)]
struct RenewedIdRow {
    id: surrealdb_types::RecordId,
}

#[cfg(test)]
#[path = "renew_test.rs"]
mod tests;
