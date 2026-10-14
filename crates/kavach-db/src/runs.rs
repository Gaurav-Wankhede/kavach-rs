use crate::error::Result;
use crate::db::DbStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RunRecord is an API type that must be constructed in downstream crates"
)]
pub struct RunRecord {
    pub id: Option<String>,
    pub project: Option<String>,
    pub entry_key: String,
    pub branch: Option<String>,
    pub status: String,
    pub command: Option<String>,
    pub pid: Option<i64>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub exit_code: Option<i64>,
    pub cost_usd: Option<f64>,
}

/// Map a `run` row (column order: id, project, entry_key, branch, status,
/// command, pid, started_at, finished_at, exit_code, cost_usd) into a
/// [`RunRecord`].
fn run_from_row(r: &libsql::Row) -> Result<RunRecord> {
    Ok(RunRecord {
        // `run.id` is `INTEGER PRIMARY KEY AUTOINCREMENT`. Reading it as `String`
        // panics inside libsql (`unreachable!("invalid value type")`). Read the
        // declared type, then render.
        id: Some(r.get::<i64>(0)?.to_string()),
        project: r.get::<Option<String>>(1)?,
        entry_key: r.get::<String>(2)?,
        branch: r.get::<Option<String>>(3)?,
        status: r.get::<String>(4)?,
        command: r.get::<Option<String>>(5)?,
        pid: r.get::<Option<i64>>(6)?,
        started_at: r.get::<Option<String>>(7)?,
        finished_at: r.get::<Option<String>>(8)?,
        exit_code: r.get::<Option<i64>>(9)?,
        cost_usd: r.get::<Option<f64>>(10)?,
    })
}

/// Insert a new run record.
///
/// # Errors
/// Propagates `Error::Database` from the INSERT.
pub async fn run_insert(db: &DbStore, run: &RunRecord) -> Result<String> {
    let mut response = db
        .exec_params(
            "INSERT INTO run \
                 (project, entry_key, branch, status, command, pid, started_at, finished_at, exit_code, cost_usd) \
             VALUES ($project, $entry_key, $branch, $status, $command, $pid, $started_at, $finished_at, $exit_code, $cost_usd) \
             RETURNING id",
            libsql::params![
                run.project.clone(),
                run.entry_key.clone(),
                run.branch.clone(),
                run.status.clone(),
                run.command.clone(),
                run.pid,
                run.started_at.clone(),
                run.finished_at.clone(),
                run.exit_code,
                run.cost_usd
            ],
        )
        .await?;
    match response.next().await? {
        Some(r) => Ok(r.get::<i64>(0)?.to_string()),
        None => Err(crate::error::Error::SchemaViolation(
            "INSERT returned no id".into(),
        )),
    }
}

/// List all runs for a project, ordered by `started_at` descending.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn run_list_by_project(db: &DbStore, project_id: &str) -> Result<Vec<RunRecord>> {
    let mut rows = db
        .exec_params(
            "SELECT id, project, entry_key, branch, status, command, pid, started_at, \
                    finished_at, exit_code, cost_usd FROM run \
             WHERE project = $project_id \
             ORDER BY started_at DESC",
            libsql::params![project_id.to_owned()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(run_from_row(&r)?);
    }
    Ok(out)
}

/// Get a single run by ID.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn run_get(db: &DbStore, id: &str) -> Result<Option<RunRecord>> {
    let mut rows = db
        .exec_params(
            "SELECT id, project, entry_key, branch, status, command, pid, started_at, \
                    finished_at, exit_code, cost_usd FROM run WHERE id = $id LIMIT 1",
            libsql::params![id.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => Ok(Some(run_from_row(&r)?)),
        None => Ok(None),
    }
}

/// Update a run's status and optionally its finish time and exit code.
///
/// # Errors
/// Propagates `Error::Database` from the UPDATE.
pub async fn run_update_status(
    db: &DbStore,
    id: &str,
    status: &str,
    finished_at: Option<String>,
    exit_code: Option<i64>,
) -> Result<()> {
    db.exec_params(
        "UPDATE run SET status = $status, finished_at = $finished_at, exit_code = $exit_code \
         WHERE id = $id",
        libsql::params![status.to_owned(), finished_at, exit_code, id.to_owned()],
    )
    .await?;
    Ok(())
}

/// Mark all running runs as orphaned on startup (server crash recovery).
/// Called once during server initialization before serving requests.
///
/// # Errors
/// Propagates `Error::Database` from the UPDATE.
pub async fn run_reconcile_orphans(db: &DbStore) -> Result<u64> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut resp = db
        .exec_params(
            "UPDATE run SET status = 'orphaned', finished_at = $now WHERE status = 'running' \
             RETURNING id",
            libsql::params![now],
        )
        .await?;
    let mut count = 0u64;
    while resp.next().await?.is_some() {
        count = count.saturating_add(1);
    }
    Ok(count)
}
