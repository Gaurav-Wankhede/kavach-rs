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
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// `POST /citations/add` — upsert a citation, then return the refreshed list.
pub async fn add(Form(f): Form<AddForm>) -> Html<String> {
    if !is_http_url(&f.url) {
        let panel = html! {
            div.panel.panel-error {
                h2 { "Request failed" }
                pre { "citation url must be http:// or https://" }
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
    let slug = project.as_deref().unwrap_or("");
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
            input type="text" name="name" placeholder="display name (e.g. SurrealDB)" required;
            input type="text" name="slug" placeholder="metadata slug (e.g. records)" required;
            input type="url" name="url" placeholder="official-docs URL" required;
            button type="submit" { "Add citation" }
        }
    }
}
