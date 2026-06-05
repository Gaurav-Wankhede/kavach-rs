// ALGO: LinearProjection
// PROBLEM_CLASS: map
// REJECTED: [{"name":"stream","reason":"single sync RPC response, no streaming"},{"name":"chunked","reason":"page sizes < 1k, no benefit"}]
// TIME: O(n) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: full materialization; bounded by RPC limit (default 50)
// BENCHMARK: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map
// SOURCE: https://docs.rs/serde/latest/serde/
use serde::{Deserialize, Serialize};

use crate::rpc_client::{Error as RpcError, rpc};
use crate::state::{EntryRef, status_from_str};

#[derive(Debug, Serialize)]
struct QueryParams<'a> {
    project: &'a str,
    category: Option<&'a str>,
    all: bool,
}

#[derive(Debug, Serialize)]
struct DeleteParams<'a> {
    project: &'a str,
    category: &'a str,
    key: Option<&'a str>,
    all: bool,
    dry_run: bool,
    confirm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryEntryDto {
    key: String,
    title: String,
    category: String,
    status: String,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryResultDto {
    entries: Vec<QueryEntryDto>,
}

#[derive(Debug, Deserialize)]
struct DeleteResultDto {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum LoadState {
    Ok(Vec<EntryRef>),
    DaemonOffline,
}

pub fn load(slug: &str) -> LoadState {
    let res = rpc::<QueryParams<'_>, QueryResultDto>(
        "db.query",
        QueryParams {
            project: slug,
            category: Some("roadmap"),
            all: true,
        },
    );
    match res {
        Ok(r) => LoadState::Ok(
            r.entries
                .into_iter()
                .map(|e| EntryRef {
                    project_slug: slug.to_owned(),
                    category: e.category,
                    status: status_from_str(&e.status),
                    key: e.key,
                    title: e.title,
                    content: e.content.unwrap_or_default(),
                })
                .collect(),
        ),
        Err(RpcError::DaemonOffline(_)) => LoadState::DaemonOffline,
        Err(e) => {
            tracing::error!(error = %e, "db.query roadmap failed");
            LoadState::Ok(Vec::new())
        }
    }
}

pub fn delete(target: &EntryRef) {
    // Target-bound confirmation the daemon requires; mirrors
    // kavach_rpc::methods::db::delete_confirm_phrase (single-key form).
    let confirm = format!(
        "delete {}/{}/{}",
        target.project_slug, target.category, target.key
    );
    let res = rpc::<DeleteParams<'_>, DeleteResultDto>(
        "db.delete",
        DeleteParams {
            project: &target.project_slug,
            category: &target.category,
            key: Some(&target.key),
            all: false,
            dry_run: false,
            confirm: Some(confirm),
        },
    );
    match res {
        Ok(r) if !r.success => tracing::error!(error = ?r.error, "db.delete returned !success"),
        Ok(_) => {}
        Err(e) => tracing::error!(error = %e, "db.delete failed"),
    }
}
