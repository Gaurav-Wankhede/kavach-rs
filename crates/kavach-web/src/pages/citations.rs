//! Citations view — list a project's official-docs citations (`citation.list`)
//! and add one (`citation.add`), live-refreshed via the SSE `refresh` signal.

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

#[derive(Debug, Deserialize)]
struct CitationMeta {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    url: String,
}

#[derive(Debug, Deserialize)]
struct Citation {
    entry_key: String,
    name: String,
    #[serde(default)]
    metadata: Vec<CitationMeta>,
    #[serde(default)]
    access_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct AddForm {
    project: String,
    entry_key: String,
    name: String,
    slug: String,
    url: String,
}

/// `GET /citations?project=<slug>` — full page.
pub async fn page(Query(q): Query<ProjectQ>) -> Html<String> {
    render(body(q.project).await)
}

/// `GET /citations/fragment?project=<slug>` — list only, for SSE swaps.
pub async fn fragment(Query(q): Query<ProjectQ>) -> Html<String> {
    render(section(q.project).await)
}

fn is_http_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    let host = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    host.is_some_and(|h| !h.is_empty() && !h.starts_with('/'))
}

fn validate_citation(f: &AddForm) -> Result<(), &'static str> {
    if f.project.trim().is_empty() || f.project.len() > 128 {
        return Err("Project name must be between 1 and 128 characters.");
    }
    if f.entry_key.trim().is_empty() || f.entry_key.len() > 128 {
        return Err("Entry key must be between 1 and 128 characters.");
    }
    if f.name.trim().is_empty() || f.name.len() > 256 {
        return Err("Citation name must be between 1 and 256 characters.");
    }
    if f.slug.trim().is_empty() || f.slug.len() > 128 {
        return Err("Citation slug must be between 1 and 128 characters.");
    }
    if !is_http_url(&f.url) || f.url.len() > 2048 {
        return Err("Citation url must be valid http:// or https:// (max 2048 chars).");
    }
    Ok(())
}

/// `POST /citations/add` — upsert a citation, then return the refreshed list.
pub async fn add(Form(f): Form<AddForm>) -> Html<String> {
    if let Err(msg) = validate_citation(&f) {
        let panel = html! {
            div.panel.panel-error {
                h2 { "Request failed" }
                pre { (msg) }
            }
        };
        return Html(panel.into_string());
    }
    let params = json!({
        "project": f.project,
        "entry_key": f.entry_key,
        "name": f.name,
        "metadata": [{ "slug": f.slug, "url": f.url }],
    });
    match call::<_, serde_json::Value>("citation.add", params).await {
        Ok(_) => render(section(Some(f.project)).await),
        Err(e) => Html(crate::error::error_panel(&e).into_string()),
    }
}

async fn body(requested: Option<String>) -> Result<Markup, RpcError> {
    let project = resolve_project(requested).await?;
    let slug: &str = project.as_deref().unwrap_or_default();
    let frag = format!("/citations/fragment?project={slug}");
    let inner = html! {
        (heading("Citations"))
        (add_form(slug))
        (live(&frag, section(project.clone()).await?))
    };
    Ok(shell("/citations", project.as_deref(), inner))
}

async fn section(project: Option<String>) -> Result<Markup, RpcError> {
    let Some(project) = project else {
        return Ok(html! { p.empty { "No project selected." } });
    };
    let citations: Vec<Citation> = call("citation.list", json!({ "project": project })).await?;
    Ok(rows(&citations))
}

fn rows(citations: &[Citation]) -> Markup {
    html! {
        div #citation-list {
            @if citations.is_empty() {
                p.empty { "No citations." }
            } @else {
                @for c in citations {
                    div.citation {
                        span.key { (c.name) }
                        span.chip { (c.entry_key) }
                        span.muted { "access " (c.access_count) }
                        @for m in &c.metadata {
                            @if is_http_url(&m.url) {
                                a.url href=(m.url) target="_blank" rel="noopener noreferrer" { (m.slug) }
                            } @else if !m.slug.is_empty() {
                                span.url { (m.slug) }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn add_form(project: &str) -> Markup {
    html! {
        form.add-citation hx-post="/citations/add" hx-target="#citation-list" hx-swap="innerHTML" {
            input type="hidden" name="project" value=(project);
            input type="text" name="entry_key" placeholder="entry key (slug)" required;
            input type="text" name="name" placeholder="display name (e.g. Database)" required;
            input type="text" name="slug" placeholder="metadata slug (e.g. records)" required;
            input type="url" name="url" placeholder="official-docs URL" required;
            button type="submit" { "Add citation" }
        }
    }
}
