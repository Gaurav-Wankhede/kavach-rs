use super::consts::{QUERIES_CONCEPTS_FOR_PROJECT, QUERIES_PROJECTS_FOR_CONCEPT};
use super::types::{BRIDGE_QUERY_LIMIT, BridgeHit, ConceptsRow, ProjectHit, ProjectsRow};
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

/// Retrieves all concepts bridged to a project.
///
/// # Errors
/// Propagates `Error` when the database query fails.
pub async fn concepts_for_project(db: &Surreal<Db>, project_slug: &str) -> Result<Vec<BridgeHit>> {
    let mut out: Vec<BridgeHit> =
        Vec::with_capacity(QUERIES_CONCEPTS_FOR_PROJECT.len().saturating_mul(8));
    for (table, edge, q) in QUERIES_CONCEPTS_FOR_PROJECT {
        let mut resp = db
            .query(*q)
            .bind(("slug", project_slug.to_owned()))
            .bind(("limit", BRIDGE_QUERY_LIMIT))
            .await?;
        let rows: Vec<ConceptsRow> = resp.take(0)?;
        for row in rows {
            for concept in row.concepts {
                out.push(BridgeHit {
                    concept,
                    edge: (*edge).to_owned(),
                    src_table: (*table).to_owned(),
                    src_key: row.entry_key.clone(),
                });
            }
        }
    }
    Ok(out)
}

/// Retrieves all projects bridged to a concept.
///
/// # Errors
/// Propagates `Error` when the database query fails.
pub async fn projects_for_concept(db: &Surreal<Db>, concept_name: &str) -> Result<Vec<ProjectHit>> {
    let mut out: Vec<ProjectHit> =
        Vec::with_capacity(QUERIES_PROJECTS_FOR_CONCEPT.len().saturating_mul(16));
    for (edge, q) in QUERIES_PROJECTS_FOR_CONCEPT {
        let mut resp = db
            .query(*q)
            .bind(("name", concept_name.to_owned()))
            .bind(("limit", BRIDGE_QUERY_LIMIT))
            .await?;
        let nested: Vec<Vec<ProjectsRow>> = resp.take(0)?;
        for inner in nested {
            for row in inner {
                out.push(ProjectHit {
                    edge: (*edge).to_owned(),
                    src_table: row.table,
                    src_key: row.key,
                    project_slug: row.slug,
                });
            }
        }
    }
    Ok(out)
}
