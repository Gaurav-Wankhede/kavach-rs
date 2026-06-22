// Delete side of the mistake ledger (completes append/read CRUD): purge an
// anti_pattern cluster + its mistake_event observations by gate.
// SOURCE: decision.mistake-ledger-purge-cli
// https://surrealdb.com/docs/surrealql/statements/delete
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

/// Delete an `anti_pattern` cluster by gate, returning the node count removed.
///
/// Removes every `anti_pattern` entity whose `gate` property matches `gate`, plus
/// the `mistake_event` entities that cluster under it (inbound `instance_of`) and
/// those edges.
///
/// Idempotent: a `gate` with no rows (or a never-created `entity` table) deletes
/// nothing and returns `Ok(0)`. The `instance_of` events are removed first so no
/// dangling edge survives the node deletion.
///
/// # Errors
/// Propagates `Error::Surreal` on a real query failure (a missing-table error is
/// the empty case, mapped to `Ok(0)`, not an error).
pub async fn delete_anti_patterns_by_gate(db: &Surreal<Db>, gate: &str) -> Result<usize> {
    #[derive(SurrealValue)]
    struct IdRow {
        name: String,
    }
    if gate.is_empty() {
        return Ok(0);
    }
    // 1. Collect the target anti_pattern node ids (by gate property).
    let select = "SELECT name FROM entity \
                  WHERE entity_type = 'anti_pattern' AND properties.gate = $gate";
    let mut resp = db.query(select).bind(("gate", gate.to_owned())).await?;
    let rows: Vec<IdRow> = match resp.take(0) {
        Ok(r) => r,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    if rows.is_empty() {
        return Ok(0);
    }
    // 2. Delete the clustered mistake_event observations + their instance_of edges,
    //    then the anti_pattern nodes. `RETURN BEFORE` returns the deleted rows so we
    //    read back the VERIFIED removal count (count→delete→verify), not the stale
    //    pre-SELECT count. SurrealDB 3.1.4 DELETE … RETURN BEFORE returns the array
    //    of deleted records. SOURCE: https://surrealdb.com/docs/surrealql/statements/delete
    let purge = "DELETE entity WHERE entity_type = 'mistake_event' \
                 AND ->instance_of->entity.properties.gate CONTAINS $gate RETURN BEFORE; \
                 DELETE entity WHERE entity_type = 'anti_pattern' \
                 AND properties.gate = $gate RETURN BEFORE;";
    let mut presp = db.query(purge).bind(("gate", gate.to_owned())).await?;
    // statement 0 = deleted mistake_events (drained, count unused); statement 1 =
    // deleted anti_patterns. A missing-table error on either slot is the empty case.
    match presp.take::<Vec<surrealdb_types::Value>>(0) {
        Ok(_) => {}
        Err(e) if crate::error::is_missing_table_error(&e) => {}
        Err(e) => return Err(e.into()),
    }
    let removed = match presp.take::<Vec<surrealdb_types::Value>>(1) {
        Ok(deleted) => deleted.len(),
        Err(e) if crate::error::is_missing_table_error(&e) => 0,
        Err(e) => return Err(e.into()),
    };
    // Read-back assertion: the verified anti_pattern removal count must match the
    // pre-SELECT target count. A mismatch means a concurrent writer or partial
    // delete — surface it rather than report a stale success.
    debug_assert_eq!(removed, rows.len(), "purge read-back mismatch: removed != selected");
    Ok(removed)
}

#[cfg(test)]
#[path = "purge_test.rs"]
mod purge_test;
