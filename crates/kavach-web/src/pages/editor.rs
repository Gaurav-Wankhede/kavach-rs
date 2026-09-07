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
use kavach_types::MemoryStatus;

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

fn validate_edit_q(q: &EditQ) -> Result<(), &'static str> {
    if q.project.trim().is_empty() || q.project.len() > 256 {
        return Err("Invalid project.");
    }
    if q.category.trim().is_empty() || !["roadmap", "decision"].contains(&q.category.as_str()) {
        return Err("Invalid category.");
    }
    if q.key.trim().is_empty() || q.key.len() > 128 {
        return Err("Invalid key.");
    }
    Ok(())
}

/// `GET /entries/edit?project=&category=&key=` — the prefilled edit form.
pub async fn edit(Query(q): Query<EditQ>) -> Html<String> {
    if let Err(msg) = validate_edit_q(&q) {
        return Html(format!("<div class=\"panel panel-error\"><h2>Validation error</h2><pre>{msg}</pre></div>"));
    }
    render(form(q).await)
}

/// `POST /entries/save` — persist title/content via `db.write` (update mode).
pub async fn save(Form(f): Form<SaveForm>) -> Html<String> {
    // Server-side validation: required fields within limits.
    if f.project.is_empty() || f.project.len() > 256 {
        return Html("Invalid project.".to_string());
    }
    if f.category.is_empty() || !["roadmap", "decision"].contains(&f.category.as_str()) {
        return Html("Invalid category.".to_string());
    }
    if f.key.is_empty() || f.key.len() > 128 {
        return Html("Invalid key.".to_string());
    }
    if f.title.is_empty() || f.title.len() > 512 {
        return Html("Invalid title.".to_string());
    }
    // Content can be empty (explicit erase) but cap at 1MB.
    if f.content.len() > 1_000_000 {
        return Html("Content too large (max 1MB).".to_string());
    }

    let params = json!({
        "project": f.project, "category": f.category, "key": f.key,
        "title": f.title, "content": f.content, "new": false, "update_key": f.key,
    });
    Html(result_panel(
        call::<_, WriteResult>("db.write", params).await,
        &f.project,
        &f.category,
    ))
}

/// `POST /entries/status` — change a card's status via `db.status_update`.
pub async fn status(Form(f): Form<StatusForm>) -> Html<String> {
    // Server-side validation.
    if f.project.is_empty() || f.project.len() > 256 {
        return Html("Invalid project.".to_string());
    }
    if f.category.is_empty() || !["roadmap", "decision"].contains(&f.category.as_str()) {
        return Html("Invalid category.".to_string());
    }
    if f.key.is_empty() || f.key.len() > 128 {
        return Html("Invalid key.".to_string());
    }
    if f.status.is_empty() || f.status.len() > 32 {
        return Html("Invalid status.".to_string());
    }

    let params = json!({
        "project": f.project, "category": f.category, "key": f.key, "status": f.status,
    });
    Html(result_panel(
        call::<_, WriteResult>("db.status_update", params).await,
        &f.project,
        &f.category,
    ))
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
        Ok(w) => {
            let err_msg = match w.error {
                Some(err) => err,
                None => "write failed".to_string(),
            };
            html! { div.panel.panel-error { (err_msg) } }
                .into_string()
        }
        Err(e) => error_panel(&e).into_string(),
    }
}

fn list_path(category: &str) -> &'static str {
    if category == "decision" {
        "decisions"
    } else {
        "roadmap"
    }
}

async fn form(q: EditQ) -> Result<Markup, RpcError> {
    let entries = fetch(&q.project, &q.category).await?;
    let found = entries.into_iter().find(|e| e.key == q.key);
    let (title, content) = match found {
        Some(e) => (e.title, e.content.unwrap_or_default()),
        None => (String::new(), String::new()),
    };
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
                @for status in MemoryStatus::all() {
                    option value=(status.as_ref()) { (status.as_ref()) }
                }
            }
            button type="submit" { "Set status" }
        }
    }
}
