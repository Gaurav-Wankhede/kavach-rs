use crate::error::{internal, invalid_params, surreal_to_rpc};
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_db::{RunRecord, run_get, run_insert, run_list_by_project, run_update_status};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub project: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct RecordParams {
    pub project: String,
    pub entry_key: String,
    pub branch: Option<String>,
    pub status: String,
    pub command: Option<String>,
    pub pid: Option<i64>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct UpdateStatusParams {
    pub id: String,
    pub status: String,
    pub finished_at: Option<String>,
    pub exit_code: Option<i64>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct CancelParams {
    pub id: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct SpawnParams {
    pub project: String,
    pub entry_key: String,
    pub branch: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize)]
pub struct SpawnResult {
    pub id: String,
    pub pid: Option<u32>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize)]
pub struct IdResult {
    pub id: String,
}

/// A run row with a flat, JSON-friendly `table:key` string id.
///
/// The web/CLI clients consume this — the Turso `run` table keys by a plain
/// string id, so no RecordId conversion is needed.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize)]
pub struct RunDto {
    pub id: String,
    pub entry_key: String,
    pub branch: Option<String>,
    pub status: String,
    pub command: Option<String>,
    pub pid: Option<i64>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub exit_code: Option<i64>,
}

fn run_to_dto(r: RunRecord) -> RunDto {
    RunDto {
        id: r.id.unwrap_or_default(),
        entry_key: r.entry_key,
        branch: r.branch,
        status: r.status,
        command: r.command,
        pid: r.pid,
        started_at: r.started_at,
        finished_at: r.finished_at,
        exit_code: r.exit_code,
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize)]
pub struct SuccessResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Resolve a project SLUG (e.g. "kavach-rs") to its project id string (the
/// slug itself — the Turso `projects` table keys by slug). The web UI and CLI
/// pass slugs, not pre-formatted record ids — mirrors the `db.*` methods'
/// `resolve_project_id`. Use this for the `project` field; `parse_record_id`
/// stays for run-id strings (`run:…`) returned by `run.list`.
async fn resolve_project(state: &AppState, slug: &str) -> Result<String, ErrorObjectOwned> {
    let project = kavach_db::project_get_by_slug(&state.db, slug)
        .await
        .map_err(surreal_to_rpc)?;
    project
        .and_then(|p| p.id)
        .ok_or_else(|| invalid_params(format!("project not found: {slug}")))
}

/// Validate the `table:key` record-id shape and return it unchanged.
fn parse_record_id(s: &str) -> Result<String, ErrorObjectOwned> {
    match s.split_once(':') {
        Some((table, key)) if !table.is_empty() && !key.is_empty() => Ok(s.to_owned()),
        _ => Err(invalid_params(format!(
            "expected RecordId 'table:id', got: {s}"
        ))),
    }
}

/// List all runs for a project.
///
/// # Errors
///
/// Returns an error if the database operation fails or project cannot be resolved.
pub async fn list(state: &AppState, params: ListParams) -> Result<Vec<RunDto>, ErrorObjectOwned> {
    let project_id = resolve_project(state, &params.project).await?;
    let runs = run_list_by_project(&state.db, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(runs.into_iter().map(run_to_dto).collect())
}

/// Record a new run.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub async fn record(state: &AppState, params: RecordParams) -> Result<IdResult, ErrorObjectOwned> {
    let project_id = resolve_project(state, &params.project).await?;
    let run = RunRecord {
        id: None,
        project: Some(project_id),
        entry_key: params.entry_key,
        branch: params.branch,
        status: params.status,
        command: params.command,
        pid: params.pid,
        started_at: None,
        finished_at: None,
        exit_code: None,
        cost_usd: None,
    };
    let id = run_insert(&state.db, &run).await.map_err(surreal_to_rpc)?;
    Ok(IdResult { id })
}

/// Update a run's status.
///
/// # Errors
///
/// Returns an error if the database operation fails or id cannot be parsed.
pub async fn update_status(
    state: &AppState,
    params: UpdateStatusParams,
) -> Result<SuccessResult, ErrorObjectOwned> {
    let id = parse_record_id(&params.id)?;
    run_update_status(
        &state.db,
        &id,
        &params.status,
        params.finished_at,
        params.exit_code,
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(SuccessResult {
        success: true,
        error: None,
    })
}

/// Cancel a run by sending SIGTERM to its process.
///
/// # Errors
///
/// Returns an error if the run cannot be found, has no pid, or the operation fails.
pub async fn cancel(
    state: &AppState,
    params: CancelParams,
) -> Result<SuccessResult, ErrorObjectOwned> {
    let id = parse_record_id(&params.id)?;
    let run = run_get(&state.db, &id).await.map_err(surreal_to_rpc)?;

    let Some(run) = run else {
        return Ok(SuccessResult {
            success: false,
            error: Some("Run not found".to_owned()),
        });
    };

    let Some(pid) = run.pid else {
        return Ok(SuccessResult {
            success: false,
            error: Some("Run has no process id".to_owned()),
        });
    };

    match Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
    {
        Ok(_) => {
            run_update_status(&state.db, &id, "cancelled", None, None)
                .await
                .map_err(surreal_to_rpc)?;
            Ok(SuccessResult {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(SuccessResult {
            success: false,
            error: Some(format!("Failed to send SIGTERM: {e}")),
        }),
    }
}

/// Spawn a detached child process and record it as a run.
///
/// # Errors
///
/// Returns an error if parameter validation, process spawn, or database operation fails.
pub async fn spawn(state: &AppState, params: SpawnParams) -> Result<SpawnResult, ErrorObjectOwned> {
    if params.command.is_empty() {
        return Err(invalid_params("command cannot be empty"));
    }

    let project_id = resolve_project(state, &params.project).await?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut cmd = Command::new(&params.command);
    cmd.args(&params.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(cwd) = &params.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| internal(format!("failed to spawn process '{}': {e}", params.command)))?;

    let pid_u32 = child.id();

    let run = RunRecord {
        id: None,
        project: Some(project_id),
        entry_key: params.entry_key,
        branch: params.branch,
        status: "running".to_owned(),
        command: Some(params.command.clone()),
        pid: Some(i64::from(pid_u32)),
        started_at: Some(now.clone()),
        finished_at: None,
        exit_code: None,
        cost_usd: None,
    };

    let record_id = run_insert(&state.db, &run).await.map_err(surreal_to_rpc)?;

    let db = std::sync::Arc::clone(&state.db);
    let record_id_clone = record_id.clone();
    tokio::spawn(async move {
        let (status, exit_code) =
            tokio::task::block_in_place(|| child.wait()).map_or(("failed", None), |exit_status| {
                let st = if exit_status.success() {
                    "done"
                } else {
                    "failed"
                };
                (st, exit_status.code().map(i64::from))
            });
        let finished_now = chrono::Utc::now().to_rfc3339();
        run_update_status(&db, &record_id_clone, status, Some(finished_now), exit_code)
            .await
            .ok();
    });

    Ok(SpawnResult {
        id: record_id,
        pid: Some(pid_u32),
    })
}
