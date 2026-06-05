// split: intentional - project-part lookup helpers (find_by_path, list_by_project, upsert)
// sql-safe: queries use static literals + .bind() for params, no user input concatenation
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug, Clone, SurrealValue)]
#[non_exhaustive]
pub struct Part {
    pub id: Option<RecordId>,
    pub project: RecordId,
    pub part_name: String,
    pub part_path: String,
    pub part_type: String,
    pub stack: Option<String>,
    pub description: Option<String>,
}

/// Longest-prefix match: find the part whose path is a prefix of `file_path`,
/// preferring the longest match (deepest registered part wins).
///
/// Uses field alias workaround for ORDER BY function (`SurrealDB` issue #1525).
///
/// # Errors
///
/// Propagates `Error::Surreal` when the query fails.
pub async fn find_by_path(db: &Surreal<Db>, file_path: &str) -> Result<Option<Part>> {
    let query = "SELECT id, project, part_name, part_path, part_type, stack, description, \
                 string::len(part_path) AS path_len \
                 FROM part \
                 WHERE string::starts_with($path, part_path) \
                 ORDER BY path_len DESC \
                 LIMIT 1";
    let mut response = db.query(query).bind(("path", file_path.to_owned())).await?;
    let part: Option<Part> = response.take(0)?;
    Ok(part)
}

/// # Errors
///
/// Propagates `Error::Surreal` when the query fails.
pub async fn list_by_project(db: &Surreal<Db>, project_id: &RecordId) -> Result<Vec<Part>> {
    let query = "SELECT id, project, part_name, part_path, part_type, stack, description \
                 FROM part WHERE project = $project";
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .await?;
    let parts: Vec<Part> = response.take(0)?;
    Ok(parts)
}

/// Upsert a part. Idempotent on (project, `part_name`).
///
/// # Errors
///
/// Propagates `Error::Surreal` when the query fails or if the part ID is missing from the result.
pub async fn upsert(
    db: &Surreal<Db>,
    project_id: &RecordId,
    part_name: &str,
    part_path: &str,
    part_type: &str,
    stack: Option<&str>,
    description: Option<&str>,
) -> Result<RecordId> {
    // FIX: [dependency_skew] SurrealDB 3.0: type::thing() -> type::record().
    let query = "UPSERT type::record('part', string::concat($project_slug, ':', $part_name)) \
                 SET project = $project, part_name = $part_name, part_path = $part_path, \
                     part_type = $part_type, stack = $stack, description = $description, \
                     updated_at = time::now() \
                 RETURN AFTER";
    let project_slug = format!("{:?}", &project_id.key);
    let mut response = db
        .query(query)
        .bind(("project", project_id.clone()))
        .bind(("project_slug", project_slug))
        .bind(("part_name", part_name.to_owned()))
        .bind(("part_path", part_path.to_owned()))
        .bind(("part_type", part_type.to_owned()))
        .bind(("stack", stack.map(ToOwned::to_owned)))
        .bind(("description", description.map(ToOwned::to_owned)))
        .await?;
    let result: Option<Part> = response.take(0)?;
    match result {
        Some(p) => p
            .id
            .ok_or_else(|| crate::error::Error::RecordNotFound(format!("part upsert {part_name}"))),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "part upsert returned empty for {part_name}"
        ))),
    }
}
