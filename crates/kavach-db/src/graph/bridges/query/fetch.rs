use super::consts::{QUERIES_CONCEPTS_FOR_PROJECT, QUERIES_PROJECTS_FOR_CONCEPT};
use super::types::{BRIDGE_QUERY_LIMIT, BridgeHit, ProjectHit};
use crate::error::Result;
use crate::graph::types::Entity;
use crate::turso::TursoStore;

/// Map a joined `(entry_key, entity...)` row to a concept `Entity`.
fn entity_from_row(row: &libsql::Row) -> Result<Entity> {
    let props = row
        .get::<Option<String>>(4)?
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| crate::error::Error::Migration(format!("bridge concept props: {e}")))?;
    Ok(Entity {
        id: row.get::<Option<String>>(1)?,
        entity_type: row.get::<String>(2)?,
        name: row.get::<String>(3)?,
        properties: props,
        content_hash: row.get::<Option<String>>(5)?,
        project: row.get::<Option<String>>(6)?,
    })
}

/// Retrieves all concepts bridged to a project.
///
/// # Errors
/// Propagates `Error` when the database query fails.
pub async fn concepts_for_project(db: &TursoStore, project_slug: &str) -> Result<Vec<BridgeHit>> {
    let mut out: Vec<BridgeHit> =
        Vec::with_capacity(QUERIES_CONCEPTS_FOR_PROJECT.len().saturating_mul(8));
    for (table, edge, q) in QUERIES_CONCEPTS_FOR_PROJECT {
        let mut resp = db
            .exec_params(
                q,
                libsql::params![
                    (*table).to_owned(),
                    (*edge).to_owned(),
                    project_slug.to_owned(),
                    BRIDGE_QUERY_LIMIT
                ],
            )
            .await?;
        while let Some(row) = resp.next().await? {
            let entry_key = row.get::<String>(0)?;
            let concept = entity_from_row(&row)?;
            out.push(BridgeHit {
                concept,
                edge: (*edge).to_owned(),
                src_table: (*table).to_owned(),
                src_key: entry_key,
            });
        }
    }
    Ok(out)
}

/// Retrieves all projects bridged to a concept.
///
/// # Errors
/// Propagates `Error` when the database query fails.
pub async fn projects_for_concept(db: &TursoStore, concept_name: &str) -> Result<Vec<ProjectHit>> {
    let mut out: Vec<ProjectHit> =
        Vec::with_capacity(QUERIES_PROJECTS_FOR_CONCEPT.len().saturating_mul(16));
    for (edge, q) in QUERIES_PROJECTS_FOR_CONCEPT {
        let mut resp = db
            .exec_params(
                q,
                libsql::params![
                    concept_name.to_owned(),
                    (*edge).to_owned(),
                    BRIDGE_QUERY_LIMIT
                ],
            )
            .await?;
        while let Some(row) = resp.next().await? {
            out.push(ProjectHit {
                edge: (*edge).to_owned(),
                src_table: row.get::<String>(0)?,
                src_key: row.get::<String>(1)?,
                project_slug: row.get::<String>(2)?,
            });
        }
    }
    Ok(out)
}
