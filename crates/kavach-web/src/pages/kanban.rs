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
    #[serde(default)]
    content: String,
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
    let r: KanbanResult = call("db.kanban", json!({ "project": &project, "limit": 500 })).await?;
    Ok(board(&project, &r))
}

fn board(project: &str, r: &KanbanResult) -> Markup {
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
                        (kard(project, it))
                    }
                }
            }
        }
    }
}

fn kard(project: &str, it: &Item) -> Markup {
    let edit = crate::pages::entries::edit_url(project, &it.category, &it.key);
    html! {
        details.kard {
            summary.kard-summary {
                span.kard-title { (it.title) }
                span.kard-meta { span.key { (it.key) } " · " (it.category) }
            }
            div.kard-body {
                @if it.content.is_empty() {
                    p.muted { "(no content)" }
                } @else {
                    pre.kard-content { (it.content) }
                }
                (inline_edit(project, it))
                a.btn-sm href=(edit) { "open full editor ↗" }
            }
        }
    }
}

fn inline_edit(project: &str, it: &Item) -> Markup {
    let result_id = format!("kres-{}", it.key);
    let target = format!("#{result_id}");
    html! {
        form.kard-edit hx-post="/entries/save" hx-target=(target) hx-swap="innerHTML" {
            input type="hidden" name="project" value=(project);
            input type="hidden" name="category" value=(it.category);
            input type="hidden" name="key" value=(it.key);
            input.kard-title-input type="text" name="title" value=(it.title) required;
            textarea name="content" rows="6" { (it.content) }
            div.kard-actions { button type="submit" { "Save" } }
        }
        form.kard-status hx-post="/entries/status" hx-target=(target) hx-swap="innerHTML" {
            input type="hidden" name="project" value=(project);
            input type="hidden" name="category" value=(it.category);
            input type="hidden" name="key" value=(it.key);
            select name="status" hx-trigger="change" hx-post="/entries/status"
                hx-target=(target) hx-swap="innerHTML" hx-include="closest form" {
                @for s in ["todo", "inprogress", "done", "verified", "onhold"] {
                    option value=(s) selected[norm(&it.status) == s] { (s) }
                }
            }
        }
        div #(result_id) {}
    }
}

fn norm(status: &str) -> &str {
    match status {
        "in_progress" => "inprogress",
        other => other,
    }
}
