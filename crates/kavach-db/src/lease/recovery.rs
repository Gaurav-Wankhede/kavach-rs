// SPEC: docs/architecture/session-occupancy-lease.md
use crate::error::Result;
use crate::db::DbStore;

const LEASED_TABLES: &[&str] = &["roadmap", "decision", "app_spec"];

/// Clears stale lease records for a session across all leased tables.
///
/// # Errors
/// Propagates `Error::Database` when the database query fails.
pub async fn clear_stale_for_session(db: &DbStore, session_id: &str) -> Result<()> {
    for table in LEASED_TABLES {
        clear_table(db, table, session_id).await?;
    }
    Ok(())
}

async fn clear_table(db: &DbStore, table: &str, session_id: &str) -> Result<()> {
    db.exec_params(
        "UPDATE memories SET occupied_by = NULL, occupied_until = NULL, \
             occupied_heartbeat = NULL \
         WHERE category = $t AND occupied_by = $s",
        libsql::params![table.to_owned(), session_id.to_owned()],
    )
    .await?;
    Ok(())
}
