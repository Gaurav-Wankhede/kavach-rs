use crate::error::Result;
use crate::graph::types::{Entity, RelateParams};
use crate::turso::TursoStore;

/// # Errors
/// Propagates `Error::RecordNotFound` if the entity creation fails or the
/// returned record lacks an ID.
pub async fn create_entity(db: &TursoStore, entity: Entity) -> Result<String> {
    let id = entity
        .id
        .clone()
        .unwrap_or_else(|| format!("entity:{}", entity.name));
    let properties = entity
        .properties
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    db.exec_params(
        "INSERT INTO entities (id, entity_type, name, properties, content_hash, project) \
         VALUES ($id, $etype, $name, $props, $hash, $project) \
         ON CONFLICT(id) DO UPDATE SET \
             entity_type = excluded.entity_type, name = excluded.name, \
             properties = excluded.properties, content_hash = excluded.content_hash, \
             project = excluded.project",
        libsql::params![
            id.clone(),
            entity.entity_type,
            entity.name,
            properties,
            entity.content_hash,
            entity.project,
        ],
    )
    .await?;
    Ok(id)
}

/// # Errors
/// Propagates `Error` if the query fails.
pub async fn get_entity(db: &TursoStore, table: &str, id: &str) -> Result<Option<Entity>> {
    let _ = table; // entities is the single entity table; `table` kept for API parity.
    let mut rows = db
        .exec_params(
            "SELECT id, entity_type, name, properties, content_hash, project \
             FROM entities WHERE id = $id LIMIT 1",
            libsql::params![id.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => Ok(Some(Entity {
            id: Some(r.get::<String>(0)?),
            entity_type: r.get::<String>(1)?,
            name: r.get::<String>(2)?,
            properties: r
                .get::<Option<String>>(3)?
                .and_then(|s| serde_json::from_str(&s).ok()),
            content_hash: r.get::<Option<String>>(4)?,
            project: r.get::<Option<String>>(5)?,
        })),
        None => Ok(None),
    }
}

/// # Errors
/// Propagates `Error::RecordNotFound` if the edge creation fails or the
/// returned record lacks an ID.
pub async fn relate(db: &TursoStore, params: &RelateParams) -> Result<String> {
    let weight = params.weight.unwrap_or(1.0);
    let rel_name = params.rel_type.as_str();
    let now = crate::turso::now_rfc3339();
    db.exec_params(
        "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
         VALUES ($from, $rel, $to, $weight, $now) \
         ON CONFLICT(from_id, rel_type, to_id) DO UPDATE SET weight = excluded.weight",
        libsql::params![
            params.from_id.clone(),
            rel_name.to_owned(),
            params.to_id.clone(),
            weight,
            now
        ],
    )
    .await?;
    Ok(format!("{}:{}:{}", params.from_id, rel_name, params.to_id))
}

/// # Errors
/// Propagates `Error` if the deletion query fails.
pub async fn delete_edge(db: &TursoStore, table: &str, id: &str) -> Result<()> {
    let _ = table; // entity_edges is the single edge table; `table` kept for API parity.
    db.exec_params(
        "DELETE FROM entity_edges WHERE from_id = $id OR to_id = $id",
        libsql::params![id.to_owned()],
    )
    .await?;
    Ok(())
}
