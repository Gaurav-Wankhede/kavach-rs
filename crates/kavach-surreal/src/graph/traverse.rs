use crate::error::Result;
use crate::graph::types::{Entity, RelationType};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::RecordId;

/// # Errors
/// Propagates `Error::Surreal` when the database query fails.
pub async fn forward(
    db: &Surreal<Db>,
    from: &RecordId,
    rel_type: RelationType,
) -> Result<Vec<Entity>> {
    let query = match rel_type {
        RelationType::Contains => "SELECT out.* AS entity FROM $node->contains->entity",
        RelationType::DependsOn => "SELECT out.* AS entity FROM $node->depends_on->entity",
        RelationType::Modifies => "SELECT out.* AS entity FROM $node->modifies->entity",
        RelationType::References => "SELECT out.* AS entity FROM $node->references->entity",
        RelationType::Mentions => "SELECT out.* AS entity FROM $node->mentions->entity",
        RelationType::WorksOn => "SELECT out.* AS entity FROM $node->works_on->entity",
        RelationType::Owns => "SELECT out.* AS entity FROM $node->owns->entity",
    };

    let mut response = db.query(query).bind(("node", from.clone())).await?;
    let entities: Vec<Entity> = response.take(0)?;
    Ok(entities)
}

/// # Errors
/// Propagates `Error::Surreal` when the database query fails.
pub async fn backward(
    db: &Surreal<Db>,
    to: &RecordId,
    rel_type: RelationType,
) -> Result<Vec<Entity>> {
    let query = match rel_type {
        RelationType::Contains => "SELECT in.* AS entity FROM $node<-contains<-entity",
        RelationType::DependsOn => "SELECT in.* AS entity FROM $node<-depends_on<-entity",
        RelationType::Modifies => "SELECT in.* AS entity FROM $node<-modifies<-entity",
        RelationType::References => "SELECT in.* AS entity FROM $node<-references<-entity",
        RelationType::Mentions => "SELECT in.* AS entity FROM $node<-mentions<-entity",
        RelationType::WorksOn => "SELECT in.* AS entity FROM $node<-works_on<-entity",
        RelationType::Owns => "SELECT in.* AS entity FROM $node<-owns<-entity",
    };

    let mut response = db.query(query).bind(("node", to.clone())).await?;
    let entities: Vec<Entity> = response.take(0)?;
    Ok(entities)
}
