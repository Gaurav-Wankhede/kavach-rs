// split: intentional - project lookup helpers (find_by_path, get_by_slug, list_all)
// sql-safe: queries use static literals + .bind() for params, no user input concatenation
use crate::error::{Error, Result};
use crate::db::DbStore;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Project {
    pub id: Option<String>,
    pub slug: String,
    pub display: String,
    pub workdir: Option<String>,
    pub stack: Option<String>,
    pub aliases: Option<Vec<String>>,
    #[serde(default)]
    pub parent: Option<String>,
}

const ANCESTRY_MAX_DEPTH: usize = 7;

/// Walk the parent chain from a project slug; returns [child, parent, grandparent, ...]
/// up to `ANCESTRY_MAX_DEPTH` (7) levels. Iterative — no recursive CTE needed.
///
/// # Errors
/// Propagates `Error::Database` from any per-step SELECT.
pub async fn get_ancestry(db: &DbStore, start_id: &str) -> Result<Vec<Project>> {
    let mut chain: Vec<Project> = Vec::new();
    let mut cursor: Option<String> = Some(start_id.to_owned());
    for _ in 0..ANCESTRY_MAX_DEPTH {
        let Some(id) = cursor.take() else { break };
        let q = "SELECT slug, display, workdir, path, stack, aliases, parent FROM projects WHERE slug = ? LIMIT 1";
        let mut rows = db
            .exec_params(q, [id.clone()])
            .await
            .map_err(Error::Db)?;
        let Some(row) = rows.next().await.map_err(|e| Error::Db(e.into()))? else {
            break;
        };
        let proj = project_from_row(&row);
        cursor.clone_from(&proj.parent);
        chain.push(proj);
    }
    Ok(chain)
}

/// Fetch a project row by its unique slug.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn get_by_slug(db: &DbStore, slug: &str) -> Result<Option<Project>> {
    let q = "SELECT slug, display, workdir, path, stack, aliases, parent FROM projects WHERE slug = ? LIMIT 1";
    let mut rows = db
        .exec_params(q, [slug.to_owned()])
        .await
        .map_err(Error::Db)?;
    let Some(row) = rows.next().await.map_err(|e| Error::Db(e.into()))? else {
        return Ok(None);
    };
    Ok(Some(project_from_row(&row)))
}

/// Find a project where `path` matches `workdir`/`path` exactly or is in `aliases`.
/// Both path columns are checked: pre-K-6 rows may carry only one of them.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn find_by_path(db: &DbStore, path: &str) -> Result<Option<Project>> {
    let q = "SELECT slug, display, workdir, path, stack, aliases, parent FROM projects WHERE workdir = ? OR path = ? OR aliases LIKE '%' || ? || '%' LIMIT 1";
    let mut rows = db
        .exec_params(q, [path.to_owned(), path.to_owned(), path.to_owned()])
        .await
        .map_err(Error::Db)?;
    let Some(row) = rows.next().await.map_err(|e| Error::Db(e.into()))? else {
        return Ok(None);
    };
    Ok(Some(project_from_row(&row)))
}

/// List all projects ordered by slug.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn list_all(db: &DbStore) -> Result<Vec<Project>> {
    let q =
        "SELECT slug, display, workdir, path, stack, aliases, parent FROM projects ORDER BY slug";
    let mut rows = db.exec_params(q, ()).await.map_err(Error::Db)?;
    let mut projects = Vec::new();
    while let Some(row) = rows.next().await.map_err(|e| Error::Db(e.into()))? {
        projects.push(project_from_row(&row));
    }
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
/// - `Error::Database` on the UPDATE.
pub async fn set_parent(
    db: &DbStore,
    child_slug: &str,
    parent_slug: Option<&str>,
) -> Result<()> {
    let child = get_by_slug(db, child_slug)
        .await?
        .ok_or_else(|| Error::RecordNotFound(format!("project {child_slug}")))?;
    let child_id = child.slug.clone();

    let parent_id = match parent_slug {
        None => None,
        Some(ps) => {
            let parent = get_by_slug(db, ps)
                .await?
                .ok_or_else(|| Error::RecordNotFound(format!("project {ps}")))?;
            let pid = parent.slug.clone();
            if pid == child_id {
                return Err(Error::InvalidHierarchy(format!(
                    "project {child_slug} cannot be its own parent"
                )));
            }
            // Cycle guard: child must not already be an ancestor of the proposed parent.
            for ancestor in get_ancestry(db, &pid).await? {
                if ancestor.id.as_deref() == Some(&child_id) {
                    return Err(Error::InvalidHierarchy(format!(
                        "linking {child_slug} -> {ps} would create a cycle"
                    )));
                }
            }
            Some(pid)
        }
    };

    let q = "UPDATE projects SET parent = ?, updated_at = datetime('now') WHERE slug = ?";
    db.exec_params(q, [parent_id.clone().unwrap_or_default(), child_id])
        .await
        .map_err(Error::Db)?;
    Ok(())
}

/// Build a `Project` from a `libsql::Row` over [`PROJECT_COLS`].
/// `workdir` falls back to the canonical `path` column so path-only rows
/// (written by the pre-K-6 direct register path) still resolve.
fn project_from_row(row: &libsql::Row) -> Project {
    let slug: String = row.get::<String>(0).unwrap_or_default();
    let display: String = row.get::<String>(1).unwrap_or_default();
    let workdir_col: Option<String> = row.get::<Option<String>>(2).unwrap_or_default();
    let path_col: Option<String> = row.get::<Option<String>>(3).unwrap_or_default();
    let workdir = workdir_col.or(path_col);
    let stack: Option<String> = row.get::<Option<String>>(4).unwrap_or_default();
    let aliases_str: Option<String> = row.get::<Option<String>>(5).unwrap_or_default();
    let aliases: Option<Vec<String>> =
        aliases_str.and_then(|s| serde_json::from_str(&s).unwrap_or_default());
    let parent_raw: Option<String> = row.get::<Option<String>>(6).unwrap_or_default();
    // Guard against the pre-K-6 self-parent rows (`parent = slug`): a project
    // is never its own parent, or forest assembly recurses forever.
    let parent = parent_raw.filter(|p| !p.is_empty() && p != &slug);
    Project {
        id: Some(slug.clone()),
        slug,
        display,
        workdir,
        stack,
        aliases,
        parent,
    }
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
/// Propagates `Error::Database` from `list_all`.
pub async fn build_forest(db: &DbStore) -> Result<Vec<ProjectNode>> {
    let projects = list_all(db).await?;
    Ok(assemble_forest(&projects))
}

/// Pure tree assembly from a flat project list — extracted for unit testing
/// without a live DB.
#[must_use]
pub fn assemble_forest(projects: &[Project]) -> Vec<ProjectNode> {
    use std::collections::BTreeMap;

    // Map id-string -> children id-strings, and id-string -> project.
    let by_id: BTreeMap<String, Project> = projects
        .iter()
        .filter_map(|p| p.id.as_ref().map(|id| (id.clone(), p.clone())))
        .collect();
    let mut children_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots: Vec<String> = Vec::new();

    for p in projects {
        let Some(id) = p.id.as_ref() else {
            continue;
        };
        match p.parent.as_ref() {
            // Parent exists in the set -> nest under it; otherwise treat as root (orphan-safe).
            Some(pid) if by_id.contains_key(pid) => {
                children_of.entry(pid.clone()).or_default().push(id.clone());
            }
            _ => roots.push(id.clone()),
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

/// Register or update a project. Uses UPSERT to be idempotent on slug.
///
/// Writes BOTH `path` (canonical) and `workdir` (legacy) so every reader —
/// slug lookup, path lookup, forest — sees the same row (K-6 split-brain fix).
/// New rows are parentless (NULL); the old self-parent default is gone.
///
/// # Errors
/// `Error::Database` on UPSERT failure.
pub async fn register(
    db: &DbStore,
    slug: &str,
    display: &str,
    workdir: &str,
    stack: Option<&str>,
) -> Result<String> {
    let q = "INSERT INTO projects (slug, display, workdir, path, stack, aliases, parent, updated_at) VALUES (?, ?, ?, ?, ?, ?, NULL, datetime('now')) ON CONFLICT(slug) DO UPDATE SET display = excluded.display, workdir = excluded.workdir, path = excluded.path, stack = excluded.stack, updated_at = datetime('now')";
    db.exec_params(
        q,
        [
            slug.to_owned(),
            display.to_owned(),
            workdir.to_owned(),
            workdir.to_owned(),
            stack.map(ToOwned::to_owned).unwrap_or_default(),
            String::new(),
        ],
    )
    .await
    .map_err(Error::Db)?;
    Ok(slug.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Project, assemble_forest, relative_to_parent};

    fn proj(slug: &str, workdir: &str, parent: Option<&str>) -> Project {
        Project {
            id: Some(slug.to_owned()),
            slug: slug.to_owned(),
            display: slug.to_owned(),
            workdir: Some(workdir.to_owned()),
            stack: None,
            aliases: None,
            parent: parent.map(ToOwned::to_owned),
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

    #[tokio::test]
    async fn register_persists_path_and_workdir_together() {
        // K-6: both register paths must leave `path` AND `workdir` populated
        // so slug lookups and path lookups agree.
        let db = crate::db::DbStore::open_memory()
            .await
            .expect("mem db");
        super::register(&db, "proj", "proj", "/tmp/proj", None)
            .await
            .expect("register");
        let got = super::get_by_slug(&db, "proj")
            .await
            .expect("lookup")
            .expect("row present");
        assert_eq!(got.workdir.as_deref(), Some("/tmp/proj"));
        let by_path = super::find_by_path(&db, "/tmp/proj")
            .await
            .expect("path lookup")
            .expect("path resolves");
        assert_eq!(by_path.slug, "proj");
        // Raw column check: `path` itself must not stay null.
        let mut rows = db
            .exec_params("SELECT path FROM projects WHERE slug = 'proj'", ())
            .await
            .expect("raw read");
        let row = rows.next().await.expect("row iter").expect("row present");
        assert_eq!(
            row.get::<Option<String>>(0)
                .expect("path decodes")
                .as_deref(),
            Some("/tmp/proj")
        );
    }

    #[tokio::test]
    async fn register_leaves_no_self_parent() {
        // K-6: the old default wrote `parent = slug`, a self-loop that makes
        // forest assembly recurse forever. New rows are parentless roots.
        let db = crate::db::DbStore::open_memory()
            .await
            .expect("mem db");
        super::register(&db, "solo", "solo", "/tmp/solo", None)
            .await
            .expect("register");
        let got = super::get_by_slug(&db, "solo")
            .await
            .expect("lookup")
            .expect("row present");
        assert_eq!(got.parent, None, "no self-parent");
        let forest = super::build_forest(&db).await.expect("forest");
        assert!(forest.iter().any(|n| n.project.slug == "solo"));
    }

    #[tokio::test]
    async fn entry_store_register_is_path_compatible() {
        // The direct CLI fallback (`EntryStore::register_project`) must produce
        // rows the slug/path readers resolve identically.
        use crate::db::entries::EntryStore;
        let db = crate::db::DbStore::open_memory()
            .await
            .expect("mem db");
        db.register_project("direct", "/tmp/direct", None)
            .await
            .expect("direct register");
        let got = super::get_by_slug(&db, "direct")
            .await
            .expect("lookup")
            .expect("row present");
        assert_eq!(got.workdir.as_deref(), Some("/tmp/direct"));
        assert!(
            super::find_by_path(&db, "/tmp/direct")
                .await
                .expect("path lookup")
                .is_some()
        );
    }
}
