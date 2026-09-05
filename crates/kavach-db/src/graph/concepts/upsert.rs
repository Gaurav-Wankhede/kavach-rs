// Concept upsert — insert-or-update a global concept row in `entities`.
use crate::error::{Error, Result};
use crate::turso::TursoStore;

/// Insert-or-update a global concept. `name` is the canonical id (`snake_case`).
/// `properties` is FLEXIBLE JSON: {display, description, tags[], sources[]}.
///
/// # Errors
/// Propagates `Error::Migration` if name is empty, or `Error::RecordNotFound` if the query returns no row.
pub async fn upsert_concept(
    db: &TursoStore,
    name: &str,
    display: &str,
    description: &str,
    tags: &[String],
    sources: &[String],
) -> Result<String> {
    if name.is_empty() {
        return Err(Error::Migration("concept name cannot be empty".into()));
    }
    let props = serde_json::json!({
        "display": display,
        "description": description,
        "tags": tags,
        "sources": sources,
    });
    let props_json = serde_json::to_string(&props)?;
    let id = format!("concept:{name}");
    let mut resp = db
        .exec_params(
            "INSERT INTO entities (id, entity_type, name, properties) \
             VALUES ($id, 'concept', $name, $props) \
             ON CONFLICT(id) DO UPDATE SET \
                 name = excluded.name, properties = excluded.properties",
            libsql::params![id.clone(), name.to_owned(), props_json],
        )
        .await?;
    match resp.next().await? {
        Some(_) => Ok(id),
        None => Err(Error::RecordNotFound(format!(
            "concept upsert empty: {name}"
        ))),
    }
}
