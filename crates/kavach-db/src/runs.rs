use crate::error::Result;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RunRecord is an API type that must be constructed in downstream crates"
)]
pub struct RunRecord {
    pub id: Option<RecordId>,
    pub project: Option<RecordId>,
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

/// Insert a new run record.
///
/// # Errors
/// Propagates `Error::Surreal` from the INSERT.
pub async fn run_insert(db: &Surreal<Db>, run: &RunRecord) -> Result<RecordId> {
    // No `RETURN id`: the default INSERT response is the full inserted record
    // (id + every field), which deserializes back into RunRecord. A `RETURN id`
    // projection would yield rows with only `id`, failing RunRecord's required
    // non-Option fields. Extract statement 0 (the INSERT) via take(0) — `take`
    // keys on statement index, not a "$result" string (the prior bug returned
    // an empty vec → spurious "INSERT returned no id").
    let query = "INSERT INTO run \
                 (project, entry_key, branch, status, command, pid, started_at, finished_at, exit_code, cost_usd) \
                 VALUES ($project, $entry_key, $branch, $status, $command, $pid, $started_at, $finished_at, $exit_code, $cost_usd)";
    let mut response = db
        .query(query)
        .bind(("project", run.project.clone()))
        .bind(("entry_key", run.entry_key.clone()))
        .bind(("branch", run.branch.clone()))
        .bind(("status", run.status.clone()))
        .bind(("command", run.command.clone()))
        .bind(("pid", run.pid))
        .bind(("started_at", run.started_at.clone()))
        .bind(("finished_at", run.finished_at.clone()))
        .bind(("exit_code", run.exit_code))
        .bind(("cost_usd", run.cost_usd))
        .await?;
    let records: Vec<RunRecord> = response.take(0)?;
    records
        .into_iter()
        .next()
        .and_then(|r| r.id)
        .ok_or_else(|| crate::error::Error::SchemaViolation("INSERT returned no id".into()))
}

/// List all runs for a project, ordered by `started_at` descending.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn run_list_by_project(
    db: &Surreal<Db>,
    project_id: &RecordId,
) -> Result<Vec<RunRecord>> {
    let query = "SELECT id, project, entry_key, branch, status, command, pid, started_at, \
                 finished_at, exit_code, cost_usd FROM run \
                 WHERE project = $project_id \
                 ORDER BY started_at DESC";
    let mut response = db
        .query(query)
        .bind(("project_id", project_id.clone()))
        .await?;
    let runs: Vec<RunRecord> = response.take(0)?;
    Ok(runs)
}

/// Get a single run by ID.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn run_get(db: &Surreal<Db>, id: &RecordId) -> Result<Option<RunRecord>> {
    let query = "SELECT id, project, entry_key, branch, status, command, pid, started_at, \
                 finished_at, exit_code, cost_usd FROM run WHERE id = $id LIMIT 1";
    let mut response = db.query(query).bind(("id", id.clone())).await?;
    let run: Option<RunRecord> = response.take(0)?;
    Ok(run)
}

/// Update a run's status and optionally its finish time and exit code.
///
/// # Errors
/// Propagates `Error::Surreal` from the UPDATE.
pub async fn run_update_status(
    db: &Surreal<Db>,
    id: &RecordId,
    status: &str,
    finished_at: Option<String>,
    exit_code: Option<i64>,
) -> Result<()> {
    let query =
        "UPDATE $id SET status = $status, finished_at = $finished_at, exit_code = $exit_code";
    db.query(query)
        .bind(("id", id.clone()))
        .bind(("status", status.to_owned()))
        .bind(("finished_at", finished_at))
        .bind(("exit_code", exit_code))
        .await?;
    Ok(())
}

/// Mark all running runs as orphaned on startup (server crash recovery).
/// Called once during server initialization before serving requests.
///
/// # Errors
/// Propagates `Error::Surreal` from the UPDATE.
pub async fn run_reconcile_orphans(db: &Surreal<Db>) -> Result<u64> {
    let now = chrono::Utc::now().to_rfc3339();
    let query = "UPDATE run SET status = 'orphaned', finished_at = $now WHERE status = 'running'";
    let _response = db.query(query).bind(("now", now)).await?;
    // For now, just return 0 to indicate reconciliation happened.
    // The actual count is available via db.query("SELECT COUNT(*) FROM run WHERE status = 'orphaned'")
    // but orphaned reconciliation is best-effort for startup recovery.
    Ok(0)
}
