use crate::error::Result;
use crate::graph::types::{Entity, RelationType};
use crate::turso::TursoStore;

fn entity_from_row(r: &libsql::Row) -> Result<Entity> {
    Ok(Entity {
        id: Some(r.get::<String>(0)?),
        entity_type: r.get::<String>(1)?,
        name: r.get::<String>(2)?,
        properties: r
            .get::<Option<String>>(3)?
            .and_then(|s| serde_json::from_str(&s).ok()),
        content_hash: r.get::<Option<String>>(4)?,
        project: r.get::<Option<String>>(5)?,
    })
}

/// # Errors
/// Propagates `Error::Turso` when the database query fails.
pub async fn forward(db: &TursoStore, from: &str, rel_type: RelationType) -> Result<Vec<Entity>> {
    let mut rows = db
        .exec_params(
            "SELECT e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
             FROM entity_edges ed \
             JOIN entities e ON e.id = ed.to_id \
             WHERE ed.from_id = $from AND ed.rel_type = $rel \
             ORDER BY e.name",
            libsql::params![from.to_owned(), rel_type.as_str().to_owned()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(entity_from_row(&r)?);
    }
    Ok(out)
}

/// # Errors
/// Propagates `Error::Turso` when the database query fails.
pub async fn backward(db: &TursoStore, to: &str, rel_type: RelationType) -> Result<Vec<Entity>> {
    let mut rows = db
        .exec_params(
            "SELECT e.id, e.entity_type, e.name, e.properties, e.content_hash, e.project \
             FROM entity_edges ed \
             JOIN entities e ON e.id = ed.from_id \
             WHERE ed.to_id = $to AND ed.rel_type = $rel \
             ORDER BY e.name",
            libsql::params![to.to_owned(), rel_type.as_str().to_owned()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(entity_from_row(&r)?);
    }
    Ok(out)
}
