// Concept lookups — find-by-name, list, BM25 FTS search.
use crate::error::Result;
use crate::graph::types::Entity;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

pub(crate) const SEARCH_LIMIT_MAX: i64 = 200;

fn clamp_limit(limit: usize) -> i64 {
    match i64::try_from(limit) {
        Ok(n) if n > 0 && n <= SEARCH_LIMIT_MAX => n,
        Ok(n) if n > SEARCH_LIMIT_MAX => SEARCH_LIMIT_MAX,
        Ok(_) | Err(_) => SEARCH_LIMIT_MAX,
    }
}

/// Find a concept by canonical name. Returns None if absent.
///
/// # Errors
///
/// Propagates `Error::Surreal` when the query fails.
pub async fn find_concept(db: &Surreal<Db>, name: &str) -> Result<Option<Entity>> {
    let q = "SELECT id, entity_type, name, properties, content_hash, project FROM entity \
             WHERE entity_type = 'concept' AND name = $name LIMIT 1";
    let mut resp = db.query(q).bind(("name", name.to_owned())).await?;
    let row: Option<Entity> = resp.take(0)?;
    Ok(row)
}

/// Full-text search concepts by description via `SurrealDB` BM25 analyzer.
/// Index: `idx_concept_fts` (schema.rs).
///
/// # Errors
///
/// Propagates `Error::Surreal` when the query fails.
pub async fn search_concepts_fts(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
) -> Result<Vec<Entity>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let limit = clamp_limit(limit);
    let q = "SELECT id, entity_type, name, properties, content_hash, project FROM entity \
             WHERE entity_type = 'concept' AND properties.description @@ $q \
             LIMIT $limit";
    let mut resp = db
        .query(q)
        .bind(("q", query.to_owned()))
        .bind(("limit", limit))
        .await?;
    let rows: Vec<Entity> = resp.take(0)?;
    Ok(rows)
}

/// List every concept (paginated). Caps at `SEARCH_LIMIT_MAX` per call.
///
/// # Errors
///
/// Propagates `Error::Surreal` when the query fails.
pub async fn list_concepts(db: &Surreal<Db>, limit: usize) -> Result<Vec<Entity>> {
    let limit = clamp_limit(limit);
    let q = "SELECT id, entity_type, name, properties, content_hash, project FROM entity \
             WHERE entity_type = 'concept' ORDER BY name LIMIT $limit";
    let mut resp = db.query(q).bind(("limit", limit)).await?;
    let rows: Vec<Entity> = resp.take(0)?;
    Ok(rows)
}
