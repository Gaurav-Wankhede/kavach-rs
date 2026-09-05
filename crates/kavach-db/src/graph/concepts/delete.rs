// Single-row + bulk-by-prefix delete on entity_type='concept'.
use crate::error::Result;
use crate::turso::TursoStore;

const BULK_DELETE_CAP: i64 = 5_000;

/// Delete a single concept by name. Returns the number of rows removed.
///
/// # Errors
/// Propagates `Error::Turso` when the query fails.
pub async fn delete_concept(db: &TursoStore, name: &str) -> Result<i64> {
    let mut resp = db
        .exec_params(
            "DELETE FROM entities WHERE entity_type = 'concept' AND name = ?1 RETURNING id",
            libsql::params![name.to_owned()],
        )
        .await?;
    let mut n: i64 = 0;
    while resp.next().await?.is_some() {
        n = n.saturating_add(1);
    }
    Ok(n)
}

/// Delete all concepts whose name starts with the given prefix (up to 5,000).
///
/// # Errors
/// Propagates `Error::Turso` when the query fails.
pub async fn delete_concepts_by_prefix(db: &TursoStore, prefix: &str) -> Result<i64> {
    if prefix.is_empty() {
        return Ok(0);
    }
    let mut resp = db
        .exec_params(
            "DELETE FROM entities WHERE entity_type = 'concept' AND name LIKE ?1 \
             RETURNING id LIMIT ?2",
            libsql::params![format!("{prefix}%"), BULK_DELETE_CAP],
        )
        .await?;
    let mut n: i64 = 0;
    while resp.next().await?.is_some() {
        n = n.saturating_add(1);
    }
    Ok(n)
}
