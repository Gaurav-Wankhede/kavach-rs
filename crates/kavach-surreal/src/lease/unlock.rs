// SPEC: docs/architecture/session-occupancy-lease.md — clear lease fields when holder is done.
// SOURCE: https://medium.com/@Modexa/7-lease-based-locks-that-dont-deadlock-d6de4a0562c9
// SOURCE: https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

use super::types::Lease;
use crate::error::{Error, Result};

/// Clears lease fields for the given key when the holder is done.
///
/// # Errors
/// Propagates `Error::Surreal` when the database query fails.
pub async fn unlock(db: &Surreal<Db>, table: &str, key: &str, lease: &Lease) -> Result<()> {
    db.query(
        "UPDATE type::record($t, $k) SET \
         occupied_by=NONE, occupied_until=NONE, occupied_heartbeat=NONE \
         WHERE occupied_by=$s AND occupied_epoch=$e",
    )
    .bind(("t", table.to_owned()))
    .bind(("k", key.to_owned()))
    .bind(("s", lease.session_id.clone()))
    .bind(("e", lease.epoch))
    .await
    .map_err(Error::Surreal)?;
    Ok(())
}
