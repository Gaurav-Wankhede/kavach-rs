// split: intentional - project lookup helpers (find_by_path, get_by_slug, list_all)
// sql-safe: queries use static literals + .bind() for params, no user input concatenation
use crate::error::Result;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct Project {
    pub id: Option<RecordId>,
    pub slug: String,
    pub display: String,
    pub workdir: Option<String>,
    pub stack: Option<String>,
    pub aliases: Option<Vec<String>>,
    #[serde(default)]
    pub parent: Option<RecordId>,
}

const PROJECT_FIELDS: &str = "id, slug, display, workdir, stack, aliases, parent";

const ANCESTRY_MAX_DEPTH: usize = 7;

/// Walk the parent chain from a project id; returns [child, parent, grandparent, ...]
/// up to `ANCESTRY_MAX_DEPTH` (7) levels. Iterative — no recursive CTE needed.
///
/// # Errors
/// Propagates `Error::Surreal` from any per-step SELECT.
pub async fn get_ancestry(db: &Surreal<Db>, start_id: &RecordId) -> Result<Vec<Project>> {
    let mut chain: Vec<Project> = Vec::new();
    let mut cursor: Option<RecordId> = Some(start_id.clone());
    for _ in 0..ANCESTRY_MAX_DEPTH {
        let Some(id) = cursor.take() else { break };
        let q = "SELECT id, slug, display, workdir, stack, aliases, parent FROM project \
                 WHERE id = $id LIMIT 1";
        let mut response = db.query(q).bind(("id", id)).await?;
        let row: Option<Project> = response.take(0)?;
        let Some(proj) = row else { break };
        cursor.clone_from(&proj.parent);
        chain.push(proj);
    }
    Ok(chain)
}

/// Fetch a project row by its unique slug.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn get_by_slug(db: &Surreal<Db>, slug: &str) -> Result<Option<Project>> {
    // BUG-FIX [silent-read-drop]: `parent` was omitted from this SELECT, so the
    // #[serde(default)] field always deserialized to None — making the hierarchy
    // invisible to every get_by_slug caller despite the column existing in schema.
    let query = "SELECT id, slug, display, workdir, stack, aliases, parent FROM project \
                 WHERE slug = $slug LIMIT 1";
    let mut response = db.query(query).bind(("slug", slug.to_owned())).await?;
    let project: Option<Project> = response.take(0)?;
    Ok(project)
}

/// Find a project where `path` matches `workdir` exactly or is in `aliases`.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn find_by_path(db: &Surreal<Db>, path: &str) -> Result<Option<Project>> {
    // BUG-FIX [silent-read-drop]: include `parent` (see get_by_slug).
    let query = "SELECT id, slug, display, workdir, stack, aliases, parent FROM project \
                 WHERE workdir = $path OR aliases CONTAINS $path LIMIT 1";
    let mut response = db.query(query).bind(("path", path.to_owned())).await?;
    let project: Option<Project> = response.take(0)?;
    Ok(project)
}

/// List all projects ordered by slug.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_all(db: &Surreal<Db>) -> Result<Vec<Project>> {
    // BUG-FIX [silent-read-drop]: include `parent` (see get_by_slug).
    let query = "SELECT id, slug, display, workdir, stack, aliases, parent FROM project \
                 ORDER BY slug";
    let mut response = db.query(query).await?;
    let projects: Vec<Project> = response.take(0)?;
    Ok(projects)
}

/// Set (or clear) a project's parent by slug. Idempotent.
///
/// Rejects self-parenting and cycles up to `ANCESTRY_MAX_DEPTH`: a project may
/// not be its own ancestor. Pass `parent_slug = None` to detach to top-level.
///
/// # Errors
/// - `Error::RecordNotFound` if either slug is unknown.
/// - `Error::InvalidHierarchy` if the link would create a self-loop or cycle.
/// - `Error::Surreal` on the UPDATE.
pub async fn set_parent(
    db: &Surreal<Db>,
    child_slug: &str,
    parent_slug: Option<&str>,
) -> Result<()> {
    let child = get_by_slug(db, child_slug)
        .await?
        .ok_or_else(|| crate::error::Error::RecordNotFound(format!("project {child_slug}")))?;
    let child_id = child
        .id
        .ok_or_else(|| crate::error::Error::RecordNotFound(format!("project id {child_slug}")))?;

    let parent_id = match parent_slug {
        None => None,
        Some(ps) => {
            let parent = get_by_slug(db, ps)
                .await?
                .ok_or_else(|| crate::error::Error::RecordNotFound(format!("project {ps}")))?;
            let pid = parent
                .id
                .ok_or_else(|| crate::error::Error::RecordNotFound(format!("project id {ps}")))?;
            if pid == child_id {
                return Err(crate::error::Error::InvalidHierarchy(format!(
                    "project {child_slug} cannot be its own parent"
                )));
            }
            // Cycle guard: child must not already be an ancestor of the proposed parent.
            for ancestor in get_ancestry(db, &pid).await? {
                if ancestor.id.as_ref() == Some(&child_id) {
                    return Err(crate::error::Error::InvalidHierarchy(format!(
                        "linking {child_slug} -> {ps} would create a cycle"
                    )));
                }
            }
            Some(pid)
        }
    };

    let query = "UPDATE $id SET parent = $parent, updated_at = time::now()";
    db.query(query)
        .bind(("id", child_id))
        .bind(("parent", parent_id))
        .await?;
    Ok(())
}

/// A node in the project hierarchy tree: a project plus its child subtrees.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ProjectNode {
    /// The project at this node.
    pub project: Project,
    /// Child subtrees, ordered by slug.
    pub children: Vec<Self>,
}

/// Recursive helper for `assemble_forest`: build the node rooted at `id`.
fn build_node(
    id: &str,
    by_id: &std::collections::BTreeMap<String, Project>,
    children_of: &std::collections::BTreeMap<String, Vec<String>>,
) -> Option<ProjectNode> {
    let project = by_id.get(id)?.clone();
    let mut children: Vec<ProjectNode> = children_of
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|cid| build_node(cid, by_id, children_of))
        .collect();
    children.sort_by(|a, b| a.project.slug.cmp(&b.project.slug));
    Some(ProjectNode { project, children })
}

/// Build the full project forest (roots = projects with no parent), each root
/// carrying its nested children. Single `list_all` read; tree assembled in-memory.
///
/// Orphans (parent set to a missing id) are surfaced as roots so nothing is hidden.
///
/// # Errors
/// Propagates `Error::Surreal` from `list_all`.
pub async fn build_forest(db: &Surreal<Db>) -> Result<Vec<ProjectNode>> {
    let projects = list_all(db).await?;
    Ok(assemble_forest(&projects))
}

/// Pure tree assembly from a flat project list — extracted for unit testing
/// without a live DB.
#[must_use]
pub fn assemble_forest(projects: &[Project]) -> Vec<ProjectNode> {
    use std::collections::BTreeMap;

    // RecordId has no Display; key maps on its stable Debug form (same convention
    // as parts.rs upsert). Local helper keeps the call sites readable.
    fn key(id: &RecordId) -> String {
        format!("{id:?}")
    }

    // Map id-string -> children id-strings, and id-string -> project.
    let by_id: BTreeMap<String, Project> = projects
        .iter()
        .filter_map(|p| p.id.as_ref().map(|id| (key(id), p.clone())))
        .collect();
    let mut children_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots: Vec<String> = Vec::new();

    for p in projects {
        let Some(id) = p.id.as_ref().map(key) else {
            continue;
        };
        match p.parent.as_ref().map(key) {
            // Parent exists in the set -> nest under it; otherwise treat as root (orphan-safe).
            Some(pid) if by_id.contains_key(&pid) => {
                children_of.entry(pid).or_default().push(id);
            }
            _ => roots.push(id),
        }
    }

    let mut forest: Vec<ProjectNode> = roots
        .iter()
        .filter_map(|id| build_node(id, &by_id, &children_of))
        .collect();
    forest.sort_by(|a, b| a.project.slug.cmp(&b.project.slug));
    forest
}

/// Derive the path of `child_workdir` relative to `parent_workdir`.
///
/// Returns `None` when `child` is not actually under `parent` (defensive — the
/// caller decides whether to fall back to the absolute path for display).
#[must_use]
pub fn relative_to_parent(parent_workdir: &str, child_workdir: &str) -> Option<String> {
    let parent = std::path::Path::new(parent_workdir);
    let child = std::path::Path::new(child_workdir);
    child
        .strip_prefix(parent)
        .ok()
        .map(|rel| rel.to_string_lossy().into_owned())
}

/// Helper to provide the SELECT field list for projects queries elsewhere.
#[must_use]
pub const fn project_fields() -> &'static str {
    PROJECT_FIELDS
}

/// Register or update a project. Uses UPSERT to be idempotent on slug.
///
/// # Errors
/// `Error::Surreal` on UPSERT failure; `Error::Migration` if the response
/// shape is malformed.
pub async fn register(
    db: &Surreal<Db>,
    slug: &str,
    display: &str,
    workdir: &str,
    stack: Option<&str>,
) -> Result<RecordId> {
    // FIX: [dependency_skew] SurrealDB 3.0 renamed SurrealQL type::thing()
    // -> type::record(). The 2->3 SDK migration fixed the Rust API but
    // not query-string builtin names; type::thing parse-errors at runtime.
    let query = "UPSERT type::record('project', $slug) \
                 SET slug = $slug, display = $display, workdir = $workdir, stack = $stack, \
                     updated_at = time::now() \
                 RETURN AFTER";
    let mut response = db
        .query(query)
        .bind(("slug", slug.to_owned()))
        .bind(("display", display.to_owned()))
        .bind(("workdir", workdir.to_owned()))
        .bind(("stack", stack.map(ToOwned::to_owned)))
        .await?;
    let result: Option<Project> = response.take(0)?;
    match result {
        Some(p) => p
            .id
            .ok_or_else(|| crate::error::Error::RecordNotFound(format!("project upsert {slug}"))),
        None => Err(crate::error::Error::RecordNotFound(format!(
            "project upsert returned empty for {slug}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{Project, assemble_forest, relative_to_parent};
    use surrealdb_types::RecordId;

    fn proj(slug: &str, workdir: &str, parent: Option<&str>) -> Project {
        Project {
            id: Some(RecordId::new("project", slug)),
            slug: slug.to_owned(),
            display: slug.to_owned(),
            workdir: Some(workdir.to_owned()),
            stack: None,
            aliases: None,
            parent: parent.map(|p| RecordId::new("project", p)),
        }
    }

    #[test]
    fn forest_nests_child_under_parent() {
        let projects = vec![
            proj("backend", "/root/nicole/Backend", Some("nicole-carpenter")),
            proj("nicole-carpenter", "/root/nicole", None),
            proj("kavach-rs", "/root/kavach", None),
        ];
        let forest = assemble_forest(&projects);
        // Two roots, sorted by slug: kavach-rs, nicole-carpenter.
        assert_eq!(forest.len(), 2);
        assert_eq!(forest[0].project.slug, "kavach-rs");
        assert_eq!(forest[1].project.slug, "nicole-carpenter");
        // backend is a child of nicole-carpenter, NOT a root.
        assert_eq!(forest[1].children.len(), 1);
        assert_eq!(forest[1].children[0].project.slug, "backend");
        assert!(forest[0].children.is_empty());
    }

    #[test]
    fn orphan_with_missing_parent_surfaces_as_root() {
        // parent points at a slug not present -> must not vanish.
        let projects = vec![proj("backend", "/root/nicole/Backend", Some("ghost"))];
        let forest = assemble_forest(&projects);
        assert_eq!(forest.len(), 1);
        assert_eq!(forest[0].project.slug, "backend");
    }

    #[test]
    fn relative_path_strips_parent_prefix() {
        assert_eq!(
            relative_to_parent("/root/nicole", "/root/nicole/Backend").as_deref(),
            Some("Backend")
        );
    }

    #[test]
    fn relative_path_none_when_not_nested() {
        assert_eq!(
            relative_to_parent("/root/nicole", "/elsewhere/Backend"),
            None
        );
    }
}
