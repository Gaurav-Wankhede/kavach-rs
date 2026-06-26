//! Concepts view — list/search concepts (`concept.list` / `concept.search`) and
//! add a new one (`concept.add`, which enforces an evidence source URL).

use axum::Form;
use axum::extract::Query;
use axum::response::Html;
use maud::{Markup, html};
use serde::Deserialize;
use serde_json::json;

use crate::error::render;
use crate::layout::{heading, live, shell};
use crate::rpc::{RpcError, call};

/// Loose view over a concept Entity row — only the fields we render.
#[derive(Debug, Deserialize)]
struct Concept {
    name: String,
    #[serde(default)]
    entity_type: String,
    #[serde(default)]
    properties: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQ {
    #[serde(default)]
    q: String,
}

#[derive(Debug, Deserialize)]
pub struct AddForm {
    name: String,
    display: String,
    desc: String,
    sources: String,
}

/// `GET /concepts` — search box, add form, and the live concept list.
pub async fn page() -> Html<String> {
    render(body().await)
}

/// `GET /concepts/fragment` — the list only, for SSE swaps.
pub async fn fragment() -> Html<String> {
    render(list_section().await)
}

/// `GET /concepts/search?q=<query>` — search results fragment.
pub async fn search(Query(q): Query<SearchQ>) -> Html<String> {
    if q.q.trim().is_empty() {
        return render(list_section().await);
    }
    let params = json!({ "query": q.q, "limit": 100 });
    render(
        call::<_, Vec<Concept>>("concept.search", params)
            .await
            .map(|c| rows(&c)),
    )
}

/// `POST /concepts/add` — create a concept, then return the refreshed list.
pub async fn add(Form(f): Form<AddForm>) -> Html<String> {
    let sources: Vec<String> = f
        .sources
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    let params =
        json!({ "name": f.name, "display": f.display, "desc": f.desc, "sources": sources });
    match call::<_, serde_json::Value>("concept.add", params).await {
        Ok(_) => render(list_section().await),
        Err(e) => Html(crate::error::error_panel(&e).into_string()),
    }
}

async fn body() -> Result<Markup, RpcError> {
    let inner = html! {
        (heading("Concepts"))
        (add_form())
        input.search type="search" name="q" placeholder="search concepts…"
            hx-get="/concepts/search" hx-target="#concept-list" hx-swap="innerHTML"
            hx-trigger="input changed delay:300ms, search";
        (live("/concepts/fragment", list_section().await?))
    };
    Ok(shell("/concepts", None, inner))
}

async fn list_section() -> Result<Markup, RpcError> {
    let concepts: Vec<Concept> = call("concept.list", json!({ "limit": 100 })).await?;
    Ok(rows(&concepts))
}

fn rows(concepts: &[Concept]) -> Markup {
    html! {
        div #concept-list {
            @if concepts.is_empty() {
                p.empty { "No concepts." }
            } @else {
                @for c in concepts {
                    div.concept {
                        span.key { (c.name) }
                        @if !c.entity_type.is_empty() { span.chip { (c.entity_type) } }
                        @if let Some(p) = &c.properties { div.muted { (p.to_string()) } }
                    }
                }
            }
        }
    }
}

fn add_form() -> Markup {
    html! {
        form.add-concept hx-post="/concepts/add" hx-target="#concept-list" hx-swap="innerHTML" {
            input type="text" name="name" placeholder="name (slug)" required;
            input type="text" name="display" placeholder="display name" required;
            input type="text" name="desc" placeholder="description" required;
            input type="text" name="sources" placeholder="source URL(s), comma-separated" required;
            button type="submit" { "Add concept" }
        }
    }
}
