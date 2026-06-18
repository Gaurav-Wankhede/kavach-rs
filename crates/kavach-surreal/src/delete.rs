// split: intentional - granular delete operations (single key or category-wide)
// Preferred over wipe_project for surgical record removal.
use crate::error::{Error, Result};
use crate::projects::get_by_slug;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug)]
#[non_exhaustive]
pub struct DeleteReport {
    pub project_slug: String,
    pub category: String,
    pub key: Option<String>,
    pub count: usize,
}

#[derive(surrealdb_types::SurrealValue)]
struct CountRow {
    count: i64,
}

const VALID_CATEGORIES: &[&str] = &["decision", "research", "pattern", "roadmap", "app_spec"];

fn validate_category(category: &str) -> Result<()> {
    if VALID_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(Error::RecordNotFound(format!(
            "invalid category '{category}' — must be one of: {}",
            VALID_CATEGORIES.join(", ")
        )))
    }
}

async fn get_project_id(db: &Surreal<Db>, project_slug: &str) -> Result<RecordId> {
    let project = get_by_slug(db, project_slug).await?;
    let project =
        project.ok_or_else(|| Error::RecordNotFound(format!("project '{project_slug}'")))?;
    project
        .id
        .ok_or_else(|| Error::RecordNotFound(format!("project '{project_slug}' has no id")))
}

/// Delete a single record by key — atomically removes the document row,
/// the graph entity (`entity_type`='memory'), and connected edges in a
/// single `SurrealDB` transaction.
///
/// `SurrealDB` auto-cleans edges when an endpoint entity is deleted.
/// SOURCE: <https://surrealdb.com/docs/surrealdb/models/graph>
/// SOURCE: <https://surrealdb.com/docs/surrealql/statements/delete>
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn delete_by_key(
    db: &Surreal<Db>,
    project_slug: &str,
    category: &str,
    key: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let pid = get_project_id(db, project_slug).await?;
    let qualified_name = format!("{project_slug}/{category}/{key}");

    // Atomic: document row + graph entity in single BEGIN/COMMIT.
    // Static table-name selection avoids format!() with user-controlled SQL.
    let table_delete = match category {
        "decision" => "DELETE decision WHERE project = $pid AND entry_key = $key;",
        "research" => "DELETE research WHERE project = $pid AND entry_key = $key;",
        "pattern" => "DELETE pattern WHERE project = $pid AND entry_key = $key;",
        "roadmap" => "DELETE roadmap WHERE project = $pid AND entry_key = $key;",
        "app_spec" => "DELETE app_spec WHERE project = $pid AND entry_key = $key;",
        other => return Err(Error::RecordNotFound(format!("unknown category: {other}"))),
    };

    let q = format!(
        "BEGIN TRANSACTION;\n\
         {table_delete}\n\
         DELETE entity WHERE entity_type = 'memory' AND name = $qname;\n\
         COMMIT TRANSACTION;"
    );

    db.query(q)
        .bind(("pid", pid))
        .bind(("key", key.to_owned()))
        .bind(("qname", qualified_name))
        .await?;

    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(key.to_owned()),
        count: 1,
    })
}

/// Preview delete of a single record (dry-run).
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn preview_delete_by_key(
    db: &Surreal<Db>,
    project_slug: &str,
    category: &str,
    key: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let pid = get_project_id(db, project_slug).await?;

    let count: usize = match category {
        "decision" => {
            let mut row_result = db.query("SELECT count() FROM decision WHERE project = $pid AND entry_key = $key GROUP ALL")
                .bind(("pid", pid))
                .bind(("key", key.to_owned()))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "research" => {
            let mut row_result = db.query("SELECT count() FROM research WHERE project = $pid AND entry_key = $key GROUP ALL")
                .bind(("pid", pid))
                .bind(("key", key.to_owned()))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "pattern" => {
            let mut row_result = db.query("SELECT count() FROM pattern WHERE project = $pid AND entry_key = $key GROUP ALL")
                .bind(("pid", pid))
                .bind(("key", key.to_owned()))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "roadmap" => {
            let mut row_result = db.query("SELECT count() FROM roadmap WHERE project = $pid AND entry_key = $key GROUP ALL")
                .bind(("pid", pid))
                .bind(("key", key.to_owned()))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "app_spec" => {
            let mut row_result = db.query("SELECT count() FROM app_spec WHERE project = $pid AND entry_key = $key GROUP ALL")
                .bind(("pid", pid))
                .bind(("key", key.to_owned()))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        other => return Err(Error::RecordNotFound(format!("unknown category: {other}"))),
    };

    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(key.to_owned()),
        count,
    })
}

/// Delete all records in a category for a project.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn delete_category(
    db: &Surreal<Db>,
    project_slug: &str,
    category: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let pid = get_project_id(db, project_slug).await?;

    // ALGO: prefix_match_via_string_starts_with
    // PROBLEM_CLASS: string_match (entity name namespace cleanup)
    // REJECTED: [{"name":"trie","reason":"single-pass DB scan; trie adds index overhead"},{"name":"regex","reason":"more expensive on a literal prefix"},{"name":"per-row delete loop","reason":"N round-trips vs single TX"}]
    // TIME: O(n) where n=entity table size | SPACE: O(1)
    // YEAR: 2026 | SEARCHED: 2026-05
    // TRADEOFF: full table scan; acceptable for category-level cleanup (rare op)
    // BENCHMARK: https://surrealdb.com/docs/surrealql/datamodel/strings#startswith
    let (count_q, table_delete) = match category {
        "decision" => (
            "SELECT count() FROM decision WHERE project = $pid GROUP ALL",
            "DELETE decision WHERE project = $pid;",
        ),
        "research" => (
            "SELECT count() FROM research WHERE project = $pid GROUP ALL",
            "DELETE research WHERE project = $pid;",
        ),
        "pattern" => (
            "SELECT count() FROM pattern WHERE project = $pid GROUP ALL",
            "DELETE pattern WHERE project = $pid;",
        ),
        "roadmap" => (
            "SELECT count() FROM roadmap WHERE project = $pid GROUP ALL",
            "DELETE roadmap WHERE project = $pid;",
        ),
        "app_spec" => (
            "SELECT count() FROM app_spec WHERE project = $pid GROUP ALL",
            "DELETE app_spec WHERE project = $pid;",
        ),
        other => return Err(Error::RecordNotFound(format!("unknown category: {other}"))),
    };

    let mut row_result = db.query(count_q).bind(("pid", pid.clone())).await?;
    let row: Option<CountRow> = row_result.take(0)?;
    let count = usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0);

    // Atomic bulk delete: document rows + matching entities in single TX.
    // Entity name pattern is "<project_slug>/<category>/<entry_key>" so
    // string::starts_with($prefix) cleans every entity in this category.
    // SurrealDB auto-cleans edges when entity endpoints vanish.
    let prefix = format!("{project_slug}/{category}/");
    let q = format!(
        "BEGIN TRANSACTION;\n\
         {table_delete}\n\
         DELETE entity WHERE entity_type = 'memory' AND string::starts_with(name, $prefix);\n\
         COMMIT TRANSACTION;"
    );
    db.query(q)
        .bind(("pid", pid))
        .bind(("prefix", prefix))
        .await?;

    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: None,
        count,
    })
}

/// Per-category `(count_query, delete_statement)` for a prefix-bound delete. The
/// table name is statically selected (never interpolated); `entry_key`/`project`
/// are bound parameters, so no caller input reaches the SQL text.
fn prefix_queries(category: &str) -> Result<(&'static str, &'static str)> {
    let pair = match category {
        "decision" => (
            "SELECT count() FROM decision WHERE project = $pid AND string::starts_with(entry_key, $kp) GROUP ALL",
            "DELETE decision WHERE project = $pid AND string::starts_with(entry_key, $kp);",
        ),
        "research" => (
            "SELECT count() FROM research WHERE project = $pid AND string::starts_with(entry_key, $kp) GROUP ALL",
            "DELETE research WHERE project = $pid AND string::starts_with(entry_key, $kp);",
        ),
        "pattern" => (
            "SELECT count() FROM pattern WHERE project = $pid AND string::starts_with(entry_key, $kp) GROUP ALL",
            "DELETE pattern WHERE project = $pid AND string::starts_with(entry_key, $kp);",
        ),
        "roadmap" => (
            "SELECT count() FROM roadmap WHERE project = $pid AND string::starts_with(entry_key, $kp) GROUP ALL",
            "DELETE roadmap WHERE project = $pid AND string::starts_with(entry_key, $kp);",
        ),
        "app_spec" => (
            "SELECT count() FROM app_spec WHERE project = $pid AND string::starts_with(entry_key, $kp) GROUP ALL",
            "DELETE app_spec WHERE project = $pid AND string::starts_with(entry_key, $kp);",
        ),
        other => return Err(Error::RecordNotFound(format!("unknown category: {other}"))),
    };
    Ok(pair)
}

/// Count records in `category` whose `entry_key` starts with `key_prefix`, for the
/// shared count step of both the preview and the apply path.
async fn count_by_key_prefix(
    db: &Surreal<Db>,
    pid: &RecordId,
    count_q: &str,
    key_prefix: &str,
) -> Result<usize> {
    let mut row_result = db
        .query(count_q)
        .bind(("pid", pid.clone()))
        .bind(("kp", key_prefix.to_owned()))
        .await?;
    let row: Option<CountRow> = row_result.take(0)?;
    Ok(usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0))
}

/// Preview a prefix delete (dry-run): report how many rows in `category` have an
/// `entry_key` starting with `key_prefix`, without deleting anything.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn preview_delete_by_key_prefix(
    db: &Surreal<Db>,
    project_slug: &str,
    category: &str,
    key_prefix: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let pid = get_project_id(db, project_slug).await?;
    let (count_q, _) = prefix_queries(category)?;
    let count = count_by_key_prefix(db, &pid, count_q, key_prefix).await?;
    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(format!("{key_prefix}*")),
        count,
    })
}

/// Delete every record in a category whose `entry_key` starts with `key_prefix`.
///
/// E.g. purge all `heal.incident.loophole-*` cards in one transaction. Atomic:
/// document rows + matching graph entities in a single `BEGIN`/`COMMIT`.
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn delete_by_key_prefix(
    db: &Surreal<Db>,
    project_slug: &str,
    category: &str,
    key_prefix: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let pid = get_project_id(db, project_slug).await?;
    let (count_q, table_delete) = prefix_queries(category)?;
    let count = count_by_key_prefix(db, &pid, count_q, key_prefix).await?;

    // Matching graph entities are named "<project>/<category>/<entry_key>", so the
    // entity prefix is the category prefix + the key prefix.
    let entity_prefix = format!("{project_slug}/{category}/{key_prefix}");
    let q = format!(
        "BEGIN TRANSACTION;\n\
         {table_delete}\n\
         DELETE entity WHERE entity_type = 'memory' AND string::starts_with(name, $eprefix);\n\
         COMMIT TRANSACTION;"
    );
    db.query(q)
        .bind(("pid", pid))
        .bind(("kp", key_prefix.to_owned()))
        .bind(("eprefix", entity_prefix))
        .await?;

    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: Some(format!("{key_prefix}*")),
        count,
    })
}

/// Preview delete of all records in a category (dry-run).
///
/// # Errors
/// Propagates `Error::RecordNotFound` if the category is invalid or the project is not found.
pub async fn preview_delete_category(
    db: &Surreal<Db>,
    project_slug: &str,
    category: &str,
) -> Result<DeleteReport> {
    validate_category(category)?;
    let pid = get_project_id(db, project_slug).await?;

    let count: usize = match category {
        "decision" => {
            let mut row_result = db
                .query("SELECT count() FROM decision WHERE project = $pid GROUP ALL")
                .bind(("pid", pid))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "research" => {
            let mut row_result = db
                .query("SELECT count() FROM research WHERE project = $pid GROUP ALL")
                .bind(("pid", pid))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "pattern" => {
            let mut row_result = db
                .query("SELECT count() FROM pattern WHERE project = $pid GROUP ALL")
                .bind(("pid", pid))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "roadmap" => {
            let mut row_result = db
                .query("SELECT count() FROM roadmap WHERE project = $pid GROUP ALL")
                .bind(("pid", pid))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        "app_spec" => {
            let mut row_result = db
                .query("SELECT count() FROM app_spec WHERE project = $pid GROUP ALL")
                .bind(("pid", pid))
                .await?;
            let row: Option<CountRow> = row_result.take(0)?;
            usize::try_from(row.map_or(0, |r| r.count)).unwrap_or(0)
        }
        other => return Err(Error::RecordNotFound(format!("unknown category: {other}"))),
    };

    Ok(DeleteReport {
        project_slug: project_slug.to_owned(),
        category: category.to_owned(),
        key: None,
        count,
    })
}
