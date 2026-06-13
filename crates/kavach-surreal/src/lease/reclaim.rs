// SPEC: docs/architecture/session-occupancy-lease.md §Reclaim (crash-orphan recovery)
// Closes harness-loop loophole L1: a session that CLAIMS a card (todo->in_progress)
// then CRASHES leaves the card stuck `in_progress` forever — `clear_stale_for_session`
// only fires on a CLEAN exit (it needs the dead session's id), and `renew_active_leases`
// merely STOPS renewing the lapsed lease without resetting the card. Nothing returns the
// orphan to the dispatchable `todo` pool. This time-based sweep does, with NO session_id:
// an `in_progress` card whose lease has lapsed (or was never set) is reclaimed to `todo`.
use chrono::Utc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};
use surrealdb_types::SurrealValue;

/// Tables whose cards carry the `occupied_*` lease columns. Mirrors
/// `recovery::LEASED_TABLES` / `renew::LEASED_TABLES`.
const LEASED_TABLES: &[&str] = &["roadmap", "decision", "app_spec"];

/// Reclaim crash-orphaned cards across all leased tables.
///
/// Any card still `in_progress` whose lease has LAPSED (`occupied_until <= now`)
/// or was never recorded (`occupied_until = NONE`) is reset to `todo` and its
/// lease fields cleared, returning it to the dispatchable pool. Returns the count.
///
/// Time-based and holder-agnostic by design — that is what closes the crash gap:
/// the dead session never runs its clean-exit `clear_stale_for_session`, so the
/// only safe signal that its work was abandoned is the lapsed TTL. A LIVE session
/// is protected because `renew_active_leases` keeps pushing `occupied_until` into
/// the future, so its card never matches `occupied_until <= now`.
///
/// Idempotent: a second sweep finds the just-reclaimed cards already `todo` and
/// skips them (the `entry_status = 'in_progress'` guard no longer matches).
///
/// # Errors
/// Propagates `Error::Surreal` when a table's UPDATE fails.
pub async fn reclaim_orphaned_in_progress(db: &Surreal<Db>) -> Result<usize> {
    let now = Utc::now();
    let mut reclaimed = 0_usize;
    for table in LEASED_TABLES {
        reclaimed = reclaimed.saturating_add(reclaim_table(db, table, now).await?);
    }
    Ok(reclaimed)
}

/// Reclaim orphaned cards in one table. Counts the rows actually reset.
async fn reclaim_table(db: &Surreal<Db>, table: &str, now: chrono::DateTime<Utc>) -> Result<usize> {
    // The lapsed-OR-absent lease test is the crux: `occupied_until = NONE`
    // catches a card claimed via the pure status-CAS path (`claim_card`), which
    // never writes a lease at all — without this arm such a card could never be
    // reclaimed after a crash, since it has no TTL to lapse.
    let resp = db
        .query(
            "UPDATE type::table($t) \
             SET entry_status='todo', occupied_by=NONE, occupied_until=NONE, \
                 occupied_heartbeat=NONE \
             WHERE entry_status='in_progress' \
                   AND (occupied_until=NONE OR occupied_until<=$now) \
             RETURN id",
        )
        .bind(("t", table.to_owned()))
        .bind(("now", now))
        .await;
    // SurrealDB defers "table does not exist" to result extraction, so the
    // tolerance check wraps both .await and .take() (mirrors renew_table): an
    // absent leased table on a fresh store holds no orphan — a 0-row no-op.
    let taken = match resp {
        Ok(mut r) => r.take::<Vec<ReclaimedIdRow>>(0),
        Err(e) => Err(e),
    };
    match taken {
        Ok(reset) => Ok(reset.len()),
        Err(e) if is_missing_table(&e) => Ok(0),
        Err(e) => Err(Error::Surreal(e)),
    }
}

/// True when the error is `SurrealDB`'s "table does not exist" — the only error
/// `reclaim_table` swallows (mirrors `renew::is_missing_table`).
fn is_missing_table(e: &surrealdb::Error) -> bool {
    e.to_string().contains("does not exist")
}

#[derive(surrealdb_types::SurrealValue)]
struct ReclaimedIdRow {
    id: surrealdb_types::RecordId,
}

#[cfg(test)]
#[path = "reclaim_test.rs"]
mod tests;
