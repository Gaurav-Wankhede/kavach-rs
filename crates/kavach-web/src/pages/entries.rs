//! Shared entry-list machinery for the Roadmap and Decisions pages, both of
//! which are `db.query` over a category, rendered as a status-tagged table.

use maud::{Markup, html};
use serde::Deserialize;
use serde_json::json;

use crate::rpc::{RpcError, call};

/// One row of `db.query`. Field names mirror the RPC `QueryEntry`.
#[derive(Debug, Deserialize)]
pub struct Entry {
    pub key: String,
    pub title: String,
    pub category: String,
    pub status: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub access_count: i64,
}

#[derive(Debug, Deserialize)]
struct QueryResult {
    entries: Vec<Entry>,
}

/// Fetch every entry in `category` for `project` (`all=true` → includes
/// verified rows).
///
/// # Errors
/// Propagates any RPC failure.
pub async fn fetch(project: &str, category: &str) -> Result<Vec<Entry>, RpcError> {
    let params = json!({ "project": project, "category": category, "all": true });
    let r: QueryResult = call("db.query", params).await?;
    Ok(r.entries)
}

/// Render entries as a table. `project` + `category` thread into per-row edit
/// links so the editor can prefill from the same query.
#[must_use]
pub fn table(project: &str, category: &str, entries: &[Entry]) -> Markup {
    html! {
        @if entries.is_empty() {
            p.empty { "No entries." }
        } @else {
            table.entries {
                thead { tr { th { "Status" } th { "Key" } th { "Title" } th { "Hits" } th {} } }
                tbody {
                    @for e in entries {
                        tr {
                            td { span.status.(status_class(&e.status)) { (e.status) } }
                            td.key { (e.key) }
                            td { (e.title) }
                            td.num { (e.access_count) }
                            td {
                                a.btn-sm href=(edit_url(project, category, &e.key)) { "edit" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn edit_url(project: &str, category: &str, key: &str) -> String {
    format!("/entries/edit?project={project}&category={category}&key={key}")
}

fn status_class(status: &str) -> &'static str {
    match status {
        "done" => "s-done",
        "verified" => "s-verified",
        "inprogress" | "in_progress" => "s-prog",
        "onhold" | "on_hold" => "s-hold",
        _ => "s-todo",
    }
}
