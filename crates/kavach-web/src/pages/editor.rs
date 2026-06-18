//! Entry editor — prefill from `db.query`, write back via `db.write`, and change
//! status via `db.status_update`. Full-parity write path for roadmap/decision
//! rows.

use axum::Form;
use axum::extract::Query;
use axum::response::Html;
use maud::{Markup, html};
use serde::Deserialize;
use serde_json::json;

use crate::error::{error_panel, render};
use crate::layout::{heading, shell};
use crate::pages::entries::fetch;
use crate::rpc::{RpcError, call};

#[derive(Debug, Deserialize)]
pub struct EditQ {
    project: String,
    category: String,
    key: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveForm {
    project: String,
    category: String,
    key: String,
    title: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusForm {
    project: String,
    category: String,
    key: String,
    status: String,
}

/// `GET /entries/edit?project=&category=&key=` — the prefilled edit form.
pub async fn edit(Query(q): Query<EditQ>) -> Html<String> {
    render(form(q).await)
}

/// `POST /entries/save` — persist title/content via `db.write` (update mode).
pub async fn save(Form(f): Form<SaveForm>) -> Html<String> {
    let params = json!({
        "project": f.project, "category": f.category, "key": f.key,
        "title": f.title, "content": f.content, "new": false, "update_key": f.key,
    });
    Html(result_panel(call::<_, WriteResult>("db.write", params).await, &f.project, &f.category))
}

/// `POST /entries/status` — change a card's status via `db.status_update`.
pub async fn status(Form(f): Form<StatusForm>) -> Html<String> {
    let params = json!({
        "project": f.project, "category": f.category, "key": f.key, "status": f.status,
    });
    Html(result_panel(call::<_, WriteResult>("db.status_update", params).await, &f.project, &f.category))
}

#[derive(Debug, Deserialize)]
struct WriteResult {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

fn result_panel(r: Result<WriteResult, RpcError>, project: &str, category: &str) -> String {
    let back = format!("/{}?project={project}", list_path(category));
    match r {
        Ok(w) if w.success => html! {
            div.panel.panel-ok { "Saved." a.btn-sm href=(back) { "back to list" } }
        }
        .into_string(),
        Ok(w) => html! { div.panel.panel-error { (w.error.unwrap_or_else(|| "write failed".into())) } }
            .into_string(),
        Err(e) => error_panel(&e).into_string(),
    }
}

fn list_path(category: &str) -> &'static str {
    if category == "decision" { "decisions" } else { "roadmap" }
}

async fn form(q: EditQ) -> Result<Markup, RpcError> {
    let entries = fetch(&q.project, &q.category).await?;
    let found = entries.into_iter().find(|e| e.key == q.key);
    let (title, content) = found
        .map(|e| (e.title, e.content.unwrap_or_default()))
        .unwrap_or_default();
    let inner = html! {
        (heading("Edit Entry"))
        div.muted { (q.category) " · " (q.key) }
        form.editor hx-post="/entries/save" hx-target="#editor-result" hx-swap="innerHTML" {
            input type="hidden" name="project" value=(q.project);
            input type="hidden" name="category" value=(q.category);
            input type="hidden" name="key" value=(q.key);
            label { "Title" input type="text" name="title" value=(title) required; }
            label { "Content" textarea name="content" rows="16" { (content) } }
            button type="submit" { "Save" }
        }
        (status_form(&q))
        div #editor-result {}
    };
    Ok(shell("/roadmap", Some(&q.project), inner))
}

fn status_form(q: &EditQ) -> Markup {
    html! {
        form.status-form hx-post="/entries/status" hx-target="#editor-result" hx-swap="innerHTML" {
            input type="hidden" name="project" value=(q.project);
            input type="hidden" name="category" value=(q.category);
            input type="hidden" name="key" value=(q.key);
            select name="status" {
                @for s in ["todo", "inprogress", "done", "verified", "onhold"] {
                    option value=(s) { (s) }
                }
            }
            button type="submit" { "Set status" }
        }
    }
}
