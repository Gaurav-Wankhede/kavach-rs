// SPEC: docs/architecture/session-occupancy-lease.md — read current lease state.
// SOURCE: https://surrealdb.com/3.0
use chrono::Utc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::types::{Lease, LeaseRow};
use crate::error::{Error, Result};

/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn status(db: &Surreal<Db>, table: &str, key: &str) -> Result<Option<Lease>> {
    let row: Option<LeaseRow> = db
        .query("SELECT occupied_by, occupied_until, occupied_epoch FROM type::thing($t, $k)")
        .bind(("t", table.to_owned()))
        .bind(("k", key.to_owned()))
        .await
        .map_err(Error::Surreal)?
        .take(0)
        .map_err(Error::Surreal)?;
    let now = Utc::now();
    Ok(row.and_then(
        |r| match (r.occupied_by, r.occupied_until, r.occupied_epoch) {
            (Some(s), Some(u), Some(e)) if u > now => Some(Lease {
                session_id: s,
                epoch: e,
                expires_at: u,
            }),
            _ => None,
        },
    ))
}
