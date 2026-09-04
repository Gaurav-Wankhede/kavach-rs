// SPEC: docs/architecture/session-occupancy-lease.md
// SOURCE: https://surrealdb.com/docs/surrealql/statements/update
// SOURCE: https://docs.rs/crate/surrealdb/latest
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

use crate::error::{Error, Result};

const LEASED_TABLES: &[&str] = &["roadmap", "decision", "app_spec"];

/// Clears stale lease records for a session across all leased tables.
///
/// # Errors
/// Propagates `Error::Surreal` when the database query fails.
pub async fn clear_stale_for_session(db: &Surreal<Db>, session_id: &str) -> Result<()> {
    for table in LEASED_TABLES {
        clear_table(db, table, session_id).await?;
    }
    Ok(())
}

async fn clear_table(db: &Surreal<Db>, table: &str, session_id: &str) -> Result<()> {
    db.query(
        "UPDATE type::table($t) SET \
         occupied_by=NONE, occupied_until=NONE, occupied_heartbeat=NONE \
         WHERE occupied_by=$s",
    )
    .bind(("t", table.to_owned()))
    .bind(("s", session_id.to_owned()))
    .await
    .map_err(Error::Surreal)?;
    Ok(())
}
