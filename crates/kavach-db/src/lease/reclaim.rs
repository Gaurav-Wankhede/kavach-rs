use crate::db::DbStore;
// SPEC: docs/architecture/session-occupancy-lease.md §Reclaim (crash-orphan recovery)
// Closes harness-loop loophole L1: a session that CLAIMS a card (todo->in_progress)
// then CRASHES leaves the card stuck `in_progress` forever — `clear_stale_for_session`
// only fires on a CLEAN exit (it needs the dead session's id), and `renew_active_leases`
// merely STOPS renewing the lapsed lease without resetting the card. Nothing returns the
// orphan to the dispatchable `todo` pool. This time-based sweep does, with NO session_id:
// an `in_progress` card whose lease has lapsed (or was never set) is reclaimed to `todo`.
use crate::error::Result;
use chrono::Utc;
/// Tables whose cards carry the `occupied_*` lease columns. Mirrors
/// `recovery::LEASED_TABLES` / `renew::LEASED_TABLES`.
const LEASED_TABLES: &[&str] = &["roadmap", "decision", "app_spec"];
/// Reclaim crash-orphaned cards across all leased tables.
///
/// Any card still `in_progress` whose lease has LAPSED (`occupied_until <= now`)
/// or was never recorded (`occupied_until IS NULL`) is reset to `todo` and its
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
/// Propagates `Error::Database` when a table's UPDATE fails.
pub async fn reclaim_orphaned_in_progress(db: &DbStore) -> Result<usize> {
    let now = Utc::now();
    let mut reclaimed = 0_usize;
    for table in LEASED_TABLES {
        reclaimed = reclaimed.saturating_add(reclaim_table(db, table, now).await?);
    }
    Ok(reclaimed)
}
/// Reclaim orphaned cards in one table. Counts the rows actually reset.
async fn reclaim_table(db: &DbStore, table: &str, now: chrono::DateTime<Utc>) -> Result<usize> {
    // The lapsed-OR-absent lease test is the crux: `occupied_until IS NULL`
    // catches a card claimed via the pure status-CAS path (`claim_card`), which
    // never writes a lease at all — without this arm such a card could never be
    // reclaimed after a crash, since it has no TTL to lapse.
    let now_s = now.to_rfc3339();
    let mut resp = db
        .exec_params(
            "UPDATE memories \
             SET entry_status = 'todo', occupied_by = NULL, occupied_until = NULL, \
                 occupied_heartbeat = NULL \
             WHERE category = $t AND entry_status = 'in_progress' \
                   AND (occupied_until IS NULL OR occupied_until <= $now) \
             RETURNING entry_key",
            libsql::params![table.to_owned(), now_s],
        )
        .await?;
    let mut reclaimed = 0usize;
    while resp.next().await?.is_some() {
        reclaimed = reclaimed.saturating_add(1);
    }
    Ok(reclaimed)
}
#[cfg(test)]
#[path = "reclaim_test.rs"]
mod tests;
