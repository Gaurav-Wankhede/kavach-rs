use crate::error::{Error, Result};
use crate::graph::types::{Edge, Entity, RelateParams, RelationType};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::RecordId;

/// # Errors
/// Propagates `Error::RecordNotFound` if the entity creation fails or the returned record lacks an ID.
pub async fn create_entity(db: &Surreal<Db>, entity: Entity) -> Result<RecordId> {
    let created: Option<Entity> = db
        .create("entity")
        .content(entity)
        .await?
        .into_iter()
        .next();
    match created {
        Some(e) => Ok(e.id.ok_or_else(|| Error::RecordNotFound("entity".into()))?),
        None => Err(Error::RecordNotFound("entity creation failed".into())),
    }
}

/// # Errors
/// Propagates `Error` if the query fails.
pub async fn get_entity(db: &Surreal<Db>, table: &str, id: &str) -> Result<Option<Entity>> {
    let entity: Option<Entity> = db.select((table, id)).await?;
    Ok(entity)
}

/// # Errors
/// Propagates `Error::RecordNotFound` if the edge creation fails or the returned record lacks an ID.
pub async fn relate(db: &Surreal<Db>, params: &RelateParams) -> Result<RecordId> {
    let weight = params.weight.unwrap_or(1.0);
    let rel_name = params.rel_type.as_str();

    let query = match params.rel_type {
        RelationType::Contains => "RELATE $from->contains->$to SET weight = $weight",
        RelationType::DependsOn => "RELATE $from->depends_on->$to SET weight = $weight",
        RelationType::Modifies => "RELATE $from->modifies->$to SET weight = $weight",
        RelationType::References => "RELATE $from->references->$to SET weight = $weight",
        RelationType::Mentions => "RELATE $from->mentions->$to SET weight = $weight",
        RelationType::WorksOn => "RELATE $from->works_on->$to SET weight = $weight",
        RelationType::Owns => "RELATE $from->owns->$to SET weight = $weight",
    };

    let mut response = db
        .query(query)
        .bind(("from", params.from_id.clone()))
        .bind(("to", params.to_id.clone()))
        .bind(("weight", weight))
        .await?;

    let edge: Option<Edge> = response.take(0)?;
    match edge {
        Some(e) => Ok(e.id.ok_or_else(|| Error::RecordNotFound(rel_name.into()))?),
        None => Err(Error::RecordNotFound("edge creation failed".into())),
    }
}

/// # Errors
/// Propagates `Error` if the deletion query fails.
pub async fn delete_edge(db: &Surreal<Db>, table: &str, id: &str) -> Result<()> {
    let _: Option<Edge> = db.delete((table, id)).await?;
    Ok(())
}
