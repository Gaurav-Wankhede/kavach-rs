// split: intentional - project-part lookup helpers (find_by_path, list_by_project, upsert)
// sql-safe: queries use static literals + params, no user input concatenation
use crate::error::Result;
use crate::turso::TursoStore;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Part {
    pub id: Option<String>,
    pub project: String,
    pub part_name: String,
    pub part_path: String,
    pub part_type: String,
    pub stack: Option<String>,
    pub description: Option<String>,
}

/// Map a `parts` row (column order: project, part_name, part_path, part_type,
/// stack, description) into a [`Part`]. The id is the composite
/// `project:part_name` key.
fn part_from_row(r: &libsql::Row) -> Result<Part> {
    let project = r.get::<String>(0)?;
    let part_name = r.get::<String>(1)?;
    Ok(Part {
        id: Some(format!("{project}:{part_name}")),
        project,
        part_name,
        part_path: r.get::<String>(2)?,
        part_type: r.get::<String>(3)?,
        stack: r.get::<Option<String>>(4)?,
        description: r.get::<Option<String>>(5)?,
    })
}

/// Longest-prefix match: find the part whose path is a prefix of `file_path`,
/// preferring the longest match (deepest registered part wins).
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn find_by_path(db: &TursoStore, file_path: &str) -> Result<Option<Part>> {
    let mut rows = db
        .exec_params(
            "SELECT project, part_name, part_path, part_type, stack, description \
             FROM parts \
             WHERE $path LIKE part_path || '%' \
             ORDER BY length(part_path) DESC \
             LIMIT 1",
            libsql::params![file_path.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => Ok(Some(part_from_row(&r)?)),
        None => Ok(None),
    }
}

/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn list_by_project(db: &TursoStore, project_id: &str) -> Result<Vec<Part>> {
    let mut rows = db
        .exec_params(
            "SELECT project, part_name, part_path, part_type, stack, description \
             FROM parts WHERE project = $project ORDER BY part_path",
            libsql::params![project_id.to_owned()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(part_from_row(&r)?);
    }
    Ok(out)
}

/// Upsert a part. Idempotent on (project, `part_name`).
///
/// # Errors
///
/// Propagates `Error::Turso` when the query fails.
pub async fn upsert(
    db: &TursoStore,
    project_id: &str,
    part_name: &str,
    part_path: &str,
    part_type: &str,
    stack: Option<&str>,
    description: Option<&str>,
) -> Result<String> {
    let now = crate::turso::now_rfc3339();
    db.exec_params(
        "INSERT INTO parts (project, part_name, part_path, part_type, stack, description, updated_at) \
         VALUES ($project, $part_name, $part_path, $part_type, $stack, $description, $now) \
         ON CONFLICT(project, part_name) DO UPDATE SET \
             part_path = excluded.part_path, part_type = excluded.part_type, \
             stack = excluded.stack, description = excluded.description, \
             updated_at = excluded.updated_at",
        libsql::params![
            project_id.to_owned(),
            part_name.to_owned(),
            part_path.to_owned(),
            part_type.to_owned(),
            stack.map(ToOwned::to_owned),
            description.map(ToOwned::to_owned),
            now
        ],
    )
    .await?;
    Ok(format!("{project_id}:{part_name}"))
}
