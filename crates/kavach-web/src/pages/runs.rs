//! Runs view — `run.list(project=<id>)` rendered as a run table,
//! live-refreshed via the SSE `refresh` signal.

use axum::Form;
use axum::extract::Query;
use axum::response::Html;
use maud::{Markup, html};
use serde::Deserialize;
use serde_json::json;

use crate::error::render;
use crate::layout::{heading, live, shell};
use crate::pages::{ProjectQ, resolve_project};
use crate::rpc::{RpcError, call};

/// One run row from `run.list`. Field names mirror the RPC `RunRecord`.
#[derive(Debug, Deserialize)]
pub struct RunRow {
    pub id: String,
    pub entry_key: String,
    #[serde(default)]
    pub branch: Option<String>,
    pub status: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub pid: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CancelForm {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct SpawnForm {
    pub project: String,
    pub entry_key: String,
    pub command: String,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SuccessResult {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[expect(dead_code)]
struct SpawnResult {
    id: String,
    #[serde(default)]
    pid: Option<u32>,
}

/// Fetch all runs for a project via `run.list`.
///
/// # Errors
/// Propagates any RPC failure.
async fn fetch(project: &str) -> Result<Vec<RunRow>, RpcError> {
    let params = json!({ "project": project });
    // `run.list` returns a JSON array of run rows directly (RunDto), not a
    // wrapper object — deserialize straight into Vec<RunRow>.
    let runs: Vec<RunRow> = call("run.list", params).await?;
    Ok(runs)
}

/// Render the spawn form + runs table.
#[must_use]
fn spawn_form(project: &str) -> Markup {
    html! {
        form.spawn-form method="post" action="/runs/spawn" {
            input type="hidden" name="project" value=(project);
            div {
                label { "Entry key:" }
                input type="text" name="entry_key" required;
            }
            div {
                label { "Command:" }
                input type="text" name="command" required;
            }
            div {
                label { "Args (space or comma separated):" }
                input type="text" name="args" placeholder="arg1 arg2";
            }
            div {
                label { "Branch (optional):" }
                input type="text" name="branch";
            }
            button.btn type="submit" { "Spawn" }
        }
    }
}

/// Render runs as a table.
#[must_use]
fn table(runs: &[RunRow]) -> Markup {
    html! {
        @if runs.is_empty() {
            p.empty { "No runs." }
        } @else {
            table.runs {
                thead { tr { th { "Status" } th { "Entry" } th { "Branch" } th { "Started" } th { "PID" } th {} } }
                tbody {
                    @for r in runs {
                        tr {
                            td { span.status.(status_class(&r.status)) { (r.status) } }
                            td.key { (r.entry_key) }
                            td { (r.branch.as_deref().unwrap_or("—")) }
                            td.time { (r.started_at.as_deref().unwrap_or("—")) }
                            td.num { (r.pid.map(|p| p.to_string()).as_deref().unwrap_or("—")) }
                            td {
                                @if let Some(_pid) = r.pid {
                                    @if r.status != "cancelled" && r.status != "done" {
                                        form.inline-form method="post" action="/runs/cancel" {
                                            input type="hidden" name="id" value=(r.id);
                                            button.btn-sm type="submit" { "cancel" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "done" => "s-done",
        "cancelled" => "s-hold",
        "running" => "s-prog",
        _ => "s-todo",
    }
}

/// `GET /runs?project=<slug>` — full page.
pub async fn page(Query(q): Query<ProjectQ>) -> Html<String> {
    render(body(q.project).await)
}

/// `GET /runs/fragment?project=<slug>` — just the table, for SSE swaps.
pub async fn fragment(Query(q): Query<ProjectQ>) -> Html<String> {
    render(section(q.project).await)
}

/// `POST /runs/cancel` — cancel a run by ID.
pub async fn cancel(Form(form): Form<CancelForm>) -> Html<String> {
    let result = match cancel_run(&form.id).await {
        Ok(resp) => {
            if resp.success {
                "Run cancelled successfully.".to_string()
            } else {
                resp.error
                    .unwrap_or_else(|| "Failed to cancel run.".to_string())
            }
        }
        Err(_) => "Failed to cancel run.".to_string(),
    };
    Html(result)
}

/// `POST /runs/spawn` — spawn a new process run.
pub async fn spawn(Form(form): Form<SpawnForm>) -> Html<String> {
    let args: Vec<String> = form
        .args
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect();

    let result = match spawn_run(
        &form.project,
        &form.entry_key,
        &form.command,
        args,
        form.branch,
    )
    .await
    {
        Ok(_resp) => "Run spawned successfully.".to_string(),
        Err(e) => format!("Failed to spawn run: {e}"),
    };
    Html(result)
}

async fn cancel_run(id: &str) -> Result<SuccessResult, RpcError> {
    let params = json!({ "id": id });
    call("run.cancel", params).await
}

async fn spawn_run(
    project: &str,
    entry_key: &str,
    command: &str,
    args: Vec<String>,
    branch: Option<String>,
) -> Result<SpawnResult, RpcError> {
    let params = json!({
        "project": project,
        "entry_key": entry_key,
        "command": command,
        "args": args,
        "branch": branch,
    });
    call("run.spawn", params).await
}

async fn body(requested: Option<String>) -> Result<Markup, RpcError> {
    let project = resolve_project(requested).await?;
    let frag = format!(
        "/runs/fragment?project={}",
        project.as_deref().unwrap_or("")
    );
    let inner = html! {
        (heading("Runs"))
        (live(&frag, section(project.clone()).await?))
    };
    Ok(shell("/runs", project.as_deref(), inner))
}

async fn section(project: Option<String>) -> Result<Markup, RpcError> {
    let Some(project) = project else {
        return Ok(html! { p.empty { "No project selected." } });
    };
    let runs = fetch(&project).await?;
    Ok(html! {
        (spawn_form(&project))
        (table(&runs))
    })
}
