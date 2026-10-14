use crate::db::DbStore;
// SPEC: docs/architecture/session-occupancy-lease.md — clear lease fields when holder is done.
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html

use super::types::Lease;
use crate::error::Result;

/// Clears lease fields for the given key when the holder is done.
///
/// # Errors
/// Propagates `Error::Database` when the database query fails.
pub async fn unlock(db: &DbStore, table: &str, key: &str, lease: &Lease) -> Result<()> {
    db.exec_params(
        "UPDATE memories SET occupied_by = NULL, occupied_until = NULL, \
             occupied_heartbeat = NULL \
         WHERE category = $t AND entry_key = $k \
           AND occupied_by = $s AND occupied_epoch = $e",
        libsql::params![
            table.to_owned(),
            key.to_owned(),
            lease.session_id.clone(),
            lease.epoch
        ],
    )
    .await?;
    Ok(())
}
