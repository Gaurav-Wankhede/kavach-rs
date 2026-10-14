use super::model::{DagEdge, DagNode, RoadmapDag};
use crate::error::Result;
use crate::db::DbStore;

/// Fetch the roadmap DAG (nodes + typed edges) for `project_slug`.
///
/// Returns an empty `RoadmapDag` when the project does not exist.
///
/// # Errors
/// Propagates `Error::Database` from the project lookup, per-category meta
/// SELECT, or the edge SELECT.
pub async fn fetch(db: &DbStore, project_slug: &str) -> Result<RoadmapDag> {
    let prefix = format!("{project_slug}/");

    let proj_q = "SELECT slug FROM projects WHERE slug = $slug LIMIT 1";
    let mut p_resp = db
        .exec_params(proj_q, libsql::params![project_slug.to_owned()])
        .await?;
    if p_resp.next().await?.is_none() {
        return Ok(RoadmapDag::default());
    }

    let categories = ["roadmap", "decision", "research", "pattern", "app_spec"];
    let mut meta: std::collections::HashMap<String, (String, String, String)> =
        std::collections::HashMap::new();
    for cat in &categories {
        let row_q = "SELECT entry_key, title, entry_status FROM memories \
                     WHERE project = $pid AND category = $cat";
        let mut r = db
            .exec_params(
                row_q,
                libsql::params![project_slug.to_owned(), (*cat).to_owned()],
            )
            .await?;
        while let Some(row) = r.next().await? {
            let entry_key = row.get::<String>(0)?;
            let title = row.get::<String>(1)?;
            let entry_status = row.get::<Option<String>>(2)?.unwrap_or_default();
            let qname = format!("{project_slug}/{cat}/{entry_key}");
            meta.insert(qname, (title, entry_status, (*cat).to_owned()));
        }
    }

    // Typed edges among memory entities in this project.
    let mut resp = db
        .exec_params(
            "SELECT e.from_id, e.rel_type, e.to_id \
             FROM entity_edges e \
             WHERE e.from_id LIKE $prefix AND e.rel_type IN \
                 ('depends_on','blocks','supersedes','references','mentions')",
            libsql::params![format!("{prefix}%")],
        )
        .await?;
    let mut raw: Vec<(String, String, String)> = Vec::new();
    while let Some(row) = resp.next().await? {
        raw.push((
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<String>(2)?,
        ));
    }

    let mut nodes: Vec<DagNode> = Vec::with_capacity(meta.len());
    for (qname, (title, status, category)) in &meta {
        nodes.push(DagNode {
            id: qname.clone(),
            entry_key: qname
                .rsplit('/')
                .next()
                .unwrap_or(qname.as_str())
                .to_owned(),
            title: title.clone(),
            entry_status: status.clone(),
            category: category.clone(),
        });
    }

    let mut edges: Vec<DagEdge> = Vec::with_capacity(raw.len());
    for (src, rel, tgt) in raw {
        if meta.contains_key(&src) && meta.contains_key(&tgt) {
            edges.push(DagEdge {
                source: src,
                target: tgt,
                rel,
            });
        }
    }

    Ok(RoadmapDag { nodes, edges })
}
