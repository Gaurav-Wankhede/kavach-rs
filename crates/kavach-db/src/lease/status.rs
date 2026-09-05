use crate::turso::TursoStore;
// SPEC: docs/architecture/session-occupancy-lease.md — read current lease state.
use chrono::Utc;

use super::types::{Lease, LeaseRow};
use crate::error::Result;

/// # Errors
/// Propagates `Error::Turso` when the query fails.
pub async fn status(db: &TursoStore, table: &str, key: &str) -> Result<Option<Lease>> {
    let mut rows = db
        .exec_params(
            "SELECT occupied_by, occupied_until, occupied_epoch FROM memories \
             WHERE category = $t AND entry_key = $k",
            libsql::params![table.to_owned(), key.to_owned()],
        )
        .await?;
    let row = match rows.next().await? {
        Some(row) => LeaseRow {
            occupied_by: row.get::<Option<String>>(0)?,
            occupied_until: row.get::<Option<String>>(1)?,
            occupied_epoch: row.get::<Option<i64>>(2)?,
        },
        None => return Ok(None),
    };
    let now = Utc::now();
    let expires_at = row.expires_at();
    Ok(match (row.occupied_by, expires_at, row.occupied_epoch) {
        (Some(s), Some(u), Some(e)) if u > now => Some(Lease {
            session_id: s,
            epoch: e,
            expires_at: u,
        }),
        _ => None,
    })
}
