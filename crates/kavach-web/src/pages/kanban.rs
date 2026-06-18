//! Kanban board — `db.kanban` rendered as four status columns with a count
//! header, live-refreshed via the SSE `refresh` signal.

use axum::extract::Query;
use axum::response::Html;
use maud::{Markup, html};
use serde::Deserialize;
use serde_json::json;

use crate::error::render;
use crate::layout::{heading, live, shell};
use crate::pages::{ProjectQ, resolve_project};
use crate::rpc::{RpcError, call};

const COLUMNS: &[(&str, &str)] = &[
    ("todo", "To Do"),
    ("inprogress", "In Progress"),
    ("done", "Done"),
    ("verified", "Verified"),
];

#[derive(Debug, Deserialize)]
struct Item {
    key: String,
    title: String,
    status: String,
    #[serde(default)]
    category: String,
}

#[derive(Debug, Deserialize, Default)]
struct Counts {
    todo: usize,
    in_progress: usize,
    done: usize,
    verified: usize,
}

#[derive(Debug, Deserialize)]
struct KanbanResult {
    items: Vec<Item>,
    #[serde(default)]
    counts: Counts,
}

/// `GET /kanban?project=<slug>` — full page.
pub async fn page(Query(q): Query<ProjectQ>) -> Html<String> {
    render(body(q.project).await)
}

/// `GET /kanban/fragment?project=<slug>` — board only, for SSE swaps.
pub async fn fragment(Query(q): Query<ProjectQ>) -> Html<String> {
    render(section(q.project).await)
}

async fn body(requested: Option<String>) -> Result<Markup, RpcError> {
    let project = resolve_project(requested).await?;
    let frag = format!("/kanban/fragment?project={}", project.as_deref().unwrap_or(""));
    let inner = html! {
        (heading("Kanban"))
        (live(&frag, section(project.clone()).await?))
    };
    Ok(shell("/kanban", project.as_deref(), inner))
}

async fn section(project: Option<String>) -> Result<Markup, RpcError> {
    let Some(project) = project else {
        return Ok(html! { p.empty { "No project selected." } });
    };
    let r: KanbanResult = call("db.kanban", json!({ "project": project, "limit": 500 })).await?;
    Ok(board(&r))
}

fn board(r: &KanbanResult) -> Markup {
    html! {
        div.counts {
            span { "todo " b { (r.counts.todo) } }
            span { "in-progress " b { (r.counts.in_progress) } }
            span { "done " b { (r.counts.done) } }
            span { "verified " b { (r.counts.verified) } }
        }
        div.kanban {
            @for (key, label) in COLUMNS {
                div.column {
                    h3 { (label) }
                    @for it in r.items.iter().filter(|i| norm(&i.status) == *key) {
                        div.kard {
                            div.kard-title { (it.title) }
                            div.kard-meta { span.key { (it.key) } " · " (it.category) }
                        }
                    }
                }
            }
        }
    }
}

fn norm(status: &str) -> &str {
    match status {
        "in_progress" => "inprogress",
        other => other,
    }
}
