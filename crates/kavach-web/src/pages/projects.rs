//! Projects landing page — lists every known project as a card linking into the
//! project-scoped views.

use axum::response::Html;
use maud::{Markup, html};

use crate::error::render;
use crate::layout::{heading, shell};
use crate::pages::{ProjectRow, list_projects};
use crate::rpc::RpcError;

/// `GET /` — the projects overview.
pub async fn page() -> Html<String> {
    render(body().await)
}

async fn body() -> Result<Markup, RpcError> {
    let projects = list_projects().await?;
    let inner = html! {
        (heading("Projects"))
        @if projects.is_empty() {
            p.empty { "No projects registered yet." }
        } @else {
            div.cards {
                @for p in &projects {
                    (card(p))
                }
            }
        }
    };
    Ok(shell("/", None, inner))
}

fn card(p: &ProjectRow) -> Markup {
    html! {
        div.card {
            h3 { (p.slug) }
            @if let Some(w) = &p.workdir { div.muted { (w) } }
            @if let Some(s) = &p.stack { div.chip { (s) } }
            div.card-links {
                a href=(format!("/kanban?project={}", p.slug)) { "Kanban" }
                a href=(format!("/roadmap?project={}", p.slug)) { "Roadmap" }
                a href=(format!("/decisions?project={}", p.slug)) { "Decisions" }
            }
        }
    }
}
