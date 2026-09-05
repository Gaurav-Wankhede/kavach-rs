// Concept lookups — find-by-name, list, BM25 FTS search.
use crate::error::Result;
use crate::graph::types::Entity;
use crate::turso::TursoStore;

pub(crate) const SEARCH_LIMIT_MAX: i64 = 200;

fn clamp_limit(limit: usize) -> i64 {
    match i64::try_from(limit) {
        Ok(n) if n > 0 && n <= SEARCH_LIMIT_MAX => n,
        Ok(n) if n > SEARCH_LIMIT_MAX => SEARCH_LIMIT_MAX,
        Ok(_) | Err(_) => SEARCH_LIMIT_MAX,
    }
}

/// Map an `entities` row to a concept `Entity`.
fn entity_from_row(row: &libsql::Row) -> Result<Entity> {
    let props = row
        .get::<Option<String>>(3)?
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| crate::error::Error::Migration(format!("concept props: {e}")))?;
    Ok(Entity {
        id: row.get::<Option<String>>(0)?,
        entity_type: row.get::<String>(1)?,
        name: row.get::<String>(2)?,
        properties: props,
        content_hash: row.get::<Option<String>>(4)?,
        project: row.get::<Option<String>>(5)?,
    })
}

/// Find a concept by canonical name. Returns None if absent.
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn find_concept(db: &TursoStore, name: &str) -> Result<Option<Entity>> {
    let mut resp = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project FROM entities \
             WHERE entity_type = 'concept' AND name = ?1 LIMIT 1",
            libsql::params![name.to_owned()],
        )
        .await?;
    match resp.next().await? {
        Some(row) => entity_from_row(&row).map(Some),
        None => Ok(None),
    }
}

/// Full-text search concepts by description via FTS5 over `entities.properties`.
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn search_concepts_fts(
    db: &TursoStore,
    query: &str,
    limit: usize,
) -> Result<Vec<Entity>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let limit = clamp_limit(limit);
    let mut resp = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project FROM entities \
             WHERE entity_type = 'concept' AND properties LIKE ?1 \
             ORDER BY name LIMIT ?2",
            libsql::params![format!("%{query}%"), limit],
        )
        .await?;
    let mut rows = Vec::new();
    while let Some(row) = resp.next().await? {
        rows.push(entity_from_row(&row)?);
    }
    Ok(rows)
}

/// List every concept (paginated). Caps at `SEARCH_LIMIT_MAX` per call.
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn list_concepts(db: &TursoStore, limit: usize) -> Result<Vec<Entity>> {
    let limit = clamp_limit(limit);
    let mut resp = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project FROM entities \
             WHERE entity_type = 'concept' ORDER BY name LIMIT ?1",
            libsql::params![limit],
        )
        .await?;
    let mut rows = Vec::new();
    while let Some(row) = resp.next().await? {
        rows.push(entity_from_row(&row)?);
    }
    Ok(rows)
}
