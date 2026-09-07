//! Mistakes view — look up the anti-pattern hit count by name via
//! `mistake.hit_count`.

use axum::extract::Query;
use axum::response::Html;
use maud::{Markup, html};
use serde::Deserialize;
use serde_json::json;

use crate::error::render;
use crate::layout::{heading, shell};
use crate::rpc::{RpcError, call};

#[derive(Debug, Deserialize)]
pub struct NameQ {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct HitCount {
    name: String,
    hit_count: i64,
}

/// `GET /mistakes` — the lookup form.
pub async fn page() -> Html<String> {
    let inner = html! {
        (heading("Mistakes"))
        form.lookup hx-get="/mistakes/lookup" hx-target="#mistake-result" hx-swap="innerHTML" {
            input type="text" name="name" placeholder="anti-pattern name, e.g. loophole_uninterrogated" required;
            button type="submit" { "Look up" }
        }
        div #mistake-result {}
    };
    Html(shell("/mistakes", None, inner).into_string())
}

fn check_name(name: &str) -> Result<&str, &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Enter an anti-pattern name.");
    }
    if trimmed.len() > 128 {
        return Err("Anti-pattern name is too long (max 128 characters).");
    }
    Ok(trimmed)
}

/// `GET /mistakes/lookup?name=<name>` — the hit-count result fragment.
pub async fn lookup(Query(q): Query<NameQ>) -> Html<String> {
    match check_name(&q.name) {
        Ok(valid_name) => render(result(valid_name).await),
        Err(msg) => Html(format!("<p class=\"empty\">{msg}</p>")),
    }
}

async fn result(name: &str) -> Result<Markup, RpcError> {
    let r: HitCount = call("mistake.hit_count", json!({ "name": name })).await?;
    Ok(html! {
        div.panel {
            span.key { (r.name) }
            span.num.big { (r.hit_count) }
            span.muted { "recorded hits" }
        }
    })
}
