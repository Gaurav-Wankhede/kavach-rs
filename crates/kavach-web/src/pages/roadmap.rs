//! Roadmap view — `db.query(category="roadmap")` rendered as an entry table,
//! live-refreshed via the SSE `refresh` signal.

use axum::extract::Query;
use axum::response::Html;
use maud::{Markup, html};

use crate::error::render;
use crate::layout::{heading, live, shell};
use crate::pages::entries::{fetch, table};
use crate::pages::{ProjectQ, resolve_project};
use crate::rpc::RpcError;

const CATEGORY: &str = "roadmap";

/// `GET /roadmap?project=<slug>` — full page.
pub async fn page(Query(q): Query<ProjectQ>) -> Html<String> {
    render(body(q.project).await)
}

/// `GET /roadmap/fragment?project=<slug>` — just the table, for SSE swaps.
pub async fn fragment(Query(q): Query<ProjectQ>) -> Html<String> {
    render(section(q.project).await)
}

async fn body(requested: Option<String>) -> Result<Markup, RpcError> {
    let project = resolve_project(requested).await?;
    let frag = format!(
        "/roadmap/fragment?project={}",
        project.as_deref().unwrap_or("")
    );
    let inner = html! {
        (heading("Roadmap"))
        (live(&frag, section(project.clone()).await?))
    };
    Ok(shell("/roadmap", project.as_deref(), inner))
}

async fn section(project: Option<String>) -> Result<Markup, RpcError> {
    let Some(project) = project else {
        return Ok(html! { p.empty { "No project selected." } });
    };
    let entries = fetch(&project, CATEGORY).await?;
    Ok(table(&project, CATEGORY, &entries))
}
