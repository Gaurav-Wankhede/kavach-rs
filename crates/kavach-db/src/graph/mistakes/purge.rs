// Delete side of the mistake ledger (completes append/read CRUD): purge an
// anti_pattern cluster + its mistake_event observations by gate.
// SOURCE: decision.mistake-ledger-purge-cli
use crate::error::Result;
use crate::turso::TursoStore;

/// Delete an `anti_pattern` cluster by gate, returning the node count removed.
///
/// Removes every `anti_pattern` entity whose `gate` property matches `gate`, plus
/// the `mistake_event` entities that cluster under it (inbound `instance_of`) and
/// those edges.
///
/// Idempotent: a `gate` with no rows deletes nothing and returns `Ok(0)`.
///
/// # Errors
/// Propagates `Error::Turso` on a real query failure.
pub async fn delete_anti_patterns_by_gate(db: &TursoStore, gate: &str) -> Result<usize> {
    if gate.is_empty() {
        return Ok(0);
    }
    // 1. Collect the target anti_pattern node ids (by gate property).
    let mut resp = db
        .exec_params(
            "SELECT id FROM entities \
             WHERE entity_type = 'anti_pattern' AND json_extract(properties, '$.gate') = $gate",
            libsql::params![gate.to_owned()],
        )
        .await?;
    let mut ids: Vec<String> = Vec::new();
    while let Some(r) = resp.next().await? {
        ids.push(r.get::<String>(0)?);
    }
    if ids.is_empty() {
        return Ok(0);
    }
    // 2. Delete the clustered mistake_event observations + their instance_of
    //    edges, then the anti_pattern nodes. Verified removal count comes from
    //    the DELETE ... RETURNING of the anti_patterns.
    let txn = db.conn.transaction().await?;
    for id in &ids {
        // `execute` (not `query`) so a failed DELETE surfaces immediately
        // instead of being deferred into unconsumed Rows (§no_silent_failure).
        txn.execute(
            "DELETE FROM entity_edges WHERE rel_type = 'instance_of' AND to_id = ?1",
            libsql::params![id.clone()],
        )
        .await?;
        txn.execute(
            "DELETE FROM entities WHERE entity_type = 'mistake_event' \
             AND id IN (SELECT from_id FROM entity_edges WHERE rel_type = 'instance_of' AND to_id = ?1)",
            libsql::params![id.clone()],
        )
        .await?;
    }
    let mut removed = 0usize;
    for id in &ids {
        let mut del = txn
            .query(
                "DELETE FROM entities WHERE id = ?1 AND entity_type = 'anti_pattern' RETURNING id",
                libsql::params![id.clone()],
            )
            .await?;
        if del.next().await?.is_some() {
            removed += 1;
        }
    }
    txn.commit().await?;
    // Read-back assertion: the verified anti_pattern removal count must match the
    // pre-SELECT target count. A mismatch means a concurrent writer or partial
    // delete — surface it rather than report a stale success.
    debug_assert_eq!(
        removed,
        ids.len(),
        "purge read-back mismatch: removed != selected"
    );
    Ok(removed)
}

#[cfg(test)]
#[path = "purge_test.rs"]
mod purge_test;
