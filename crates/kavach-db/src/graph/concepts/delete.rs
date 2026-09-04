// Single-row + bulk-by-prefix delete on entity_type='concept'.
// SurrealDB DELETE has no RETURN COUNT; use RETURN BEFORE + .len() per
// https://surrealdb.com/docs/surrealql/statements/delete
use crate::error::Result;
use crate::graph::types::Entity;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

const BULK_DELETE_CAP: i64 = 5_000;

/// Delete a single concept by name.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn delete_concept(db: &Surreal<Db>, name: &str) -> Result<i64> {
    let q = "DELETE entity WHERE entity_type = 'concept' AND name = $name RETURN BEFORE";
    let mut resp = db.query(q).bind(("name", name.to_owned())).await?;
    let removed: Vec<Entity> = resp.take(0)?;
    // SOURCE: https://rust-lang.github.io/rust-clippy/rust-1.95.0/index.html#manual_unwrap_or
    let n = i64::try_from(removed.len()).unwrap_or(i64::MAX);
    Ok(n)
}

/// Delete all concepts whose name starts with the given prefix (up to 5,000).
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn delete_concepts_by_prefix(db: &Surreal<Db>, prefix: &str) -> Result<i64> {
    if prefix.is_empty() {
        return Ok(0);
    }
    // SOURCE: surrealdb.com/docs/surrealql/statements/delete — no LIMIT clause on DELETE.
    let q = "LET $ids = (SELECT VALUE id FROM entity \
             WHERE entity_type = 'concept' \
             AND string::starts_with(name, $prefix) LIMIT $cap); \
             DELETE $ids RETURN BEFORE;";
    let mut resp = db
        .query(q)
        .bind(("prefix", prefix.to_owned()))
        .bind(("cap", BULK_DELETE_CAP))
        .await?;
    let removed: Vec<Entity> = resp.take(1)?;
    // SOURCE: https://rust-lang.github.io/rust-clippy/rust-1.95.0/index.html#manual_unwrap_or
    let n = i64::try_from(removed.len()).unwrap_or(i64::MAX);
    Ok(n)
}
