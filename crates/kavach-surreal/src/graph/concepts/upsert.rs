// Concept upsert — insert-or-update a global concept row in `entity`.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(SurrealValue)]
struct IdRow {
    id: RecordId,
}

/// Insert-or-update a global concept. `name` is the canonical id (`snake_case`).
/// `properties` is FLEXIBLE JSON: {display, description, tags[], sources[]}.
///
/// # Errors
/// Propagates `Error::Migration` if name is empty, or `Error::RecordNotFound` if the query returns no row.
pub async fn upsert_concept(
    db: &Surreal<Db>,
    name: &str,
    display: &str,
    description: &str,
    tags: &[String],
    sources: &[String],
) -> Result<RecordId> {
    if name.is_empty() {
        return Err(Error::Migration("concept name cannot be empty".into()));
    }
    let props = serde_json::json!({
        "display": display,
        "description": description,
        "tags": tags,
        "sources": sources,
    });
    let q = "UPSERT entity \
             SET entity_type = 'concept', name = $name, properties = $props, \
                 updated_at = time::now() \
             WHERE entity_type = 'concept' AND name = $name \
             RETURN id";
    let mut resp = db
        .query(q)
        .bind(("name", name.to_owned()))
        .bind(("props", props))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|r| r.id)
        .ok_or_else(|| Error::RecordNotFound(format!("concept upsert empty: {name}")))
}
