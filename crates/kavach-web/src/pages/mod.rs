//! Page + fragment handlers. Each module owns one sidebar view.
//!
//! Full-page handlers (`page`) return the whole shell; `fragment`/`data`
//! handlers return just the inner section HTMX swaps on `sse:refresh`.

pub mod citations;
pub mod concepts;
pub mod decisions;
pub mod editor;
pub mod entries;
pub mod kanban;
pub mod knowledge;
pub mod mistakes;
pub mod projects;
pub mod roadmap;
pub mod runs;

use serde::Deserialize;

use crate::rpc::{RpcError, call_no_params};

/// Query string shared by project-scoped pages: `?project=<slug>`.
#[derive(Debug, Deserialize, Default)]
pub struct ProjectQ {
    /// Selected project slug; when absent the first known project is used.
    pub project: Option<String>,
}

/// One row of `db.list_projects`. Field names mirror the RPC `ProjectRow`.
#[derive(Debug, Deserialize)]
pub struct ProjectRow {
    pub slug: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub stack: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListProjects {
    projects: Vec<ProjectRow>,
}

/// Fetch all projects via `db.list_projects`.
///
/// # Errors
/// Propagates any RPC failure.
pub async fn list_projects() -> Result<Vec<ProjectRow>, RpcError> {
    let r: ListProjects = call_no_params("db.list_projects").await?;
    Ok(r.projects)
}

/// Resolve the effective project: the requested slug if given, else the first
/// known project. Returns `None` only when no projects exist at all.
///
/// # Errors
/// Propagates any RPC failure from listing projects.
pub async fn resolve_project(requested: Option<String>) -> Result<Option<String>, RpcError> {
    if let Some(slug) = requested {
        return Ok(Some(slug));
    }
    Ok(list_projects().await?.into_iter().next().map(|p| p.slug))
}
