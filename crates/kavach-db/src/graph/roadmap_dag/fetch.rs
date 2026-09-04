use super::model::{DagEdge, DagNode, RoadmapDag};
use crate::error::Result;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

#[derive(SurrealValue)]
struct IdRow {
    id: surrealdb_types::RecordId,
}

#[derive(Deserialize, SurrealValue)]
struct MetaRow {
    entry_key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    entry_status: String,
}

#[derive(Deserialize, SurrealValue, Default)]
struct RawRow {
    name: String,
    #[serde(default)]
    out_depends_on: Vec<String>,
    #[serde(default)]
    out_blocks: Vec<String>,
    #[serde(default)]
    out_supersedes: Vec<String>,
    #[serde(default)]
    out_references: Vec<String>,
    #[serde(default)]
    out_mentions: Vec<String>,
}

/// Fetch the roadmap DAG (nodes + typed edges) for `project_slug`.
///
/// Returns an empty `RoadmapDag` when the project does not exist.
///
/// # Errors
/// Propagates `Error::Surreal` from the project lookup, per-category meta
/// SELECT, or the edge SELECT.
pub async fn fetch(db: &Surreal<Db>, project_slug: &str) -> Result<RoadmapDag> {
    let prefix = format!("{project_slug}/");

    let proj_q = "SELECT id FROM project WHERE slug = $slug LIMIT 1";
    let mut p_resp = db
        .query(proj_q)
        .bind(("slug", project_slug.to_owned()))
        .await?;
    let proj_row: Option<IdRow> = p_resp.take(0)?;
    let Some(IdRow { id: pid }) = proj_row else {
        return Ok(RoadmapDag::default());
    };

    let categories = ["roadmap", "decision", "research", "pattern", "app_spec"];
    let mut meta: std::collections::HashMap<String, (String, String, String)> =
        std::collections::HashMap::new();
    for cat in &categories {
        let row_q = "SELECT entry_key, title, entry_status FROM type::table($table) \
                     WHERE project = $pid";
        let mut r = db
            .query(row_q)
            .bind(("table", (*cat).to_owned()))
            .bind(("pid", pid.clone()))
            .await?;
        let rows: Vec<MetaRow> = r.take(0)?;
        for m in rows {
            let qname = format!("{project_slug}/{cat}/{}", m.entry_key);
            meta.insert(qname, (m.title, m.entry_status, (*cat).to_owned()));
        }
    }

    let q = "SELECT name, \
                ->depends_on->entity.name AS out_depends_on, \
                ->blocks->entity.name     AS out_blocks, \
                ->supersedes->entity.name AS out_supersedes, \
                ->references->entity.name AS out_references, \
                ->mentions->entity.name   AS out_mentions \
             FROM entity \
             WHERE entity_type = 'memory' AND string::starts_with(name, $prefix);";
    let mut resp = db.query(q).bind(("prefix", prefix.clone())).await?;
    let raw: Vec<RawRow> = resp.take(0)?;

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

    let mut edges: Vec<DagEdge> = Vec::with_capacity(raw.len().saturating_mul(2));
    for row in raw {
        let src = row.name;
        if !meta.contains_key(&src) {
            continue;
        }
        let per_rel: [(&str, Vec<String>); 5] = [
            ("depends_on", row.out_depends_on),
            ("blocks", row.out_blocks),
            ("supersedes", row.out_supersedes),
            ("references", row.out_references),
            ("mentions", row.out_mentions),
        ];
        for (rel, targets) in per_rel {
            for tgt in targets {
                if meta.contains_key(&tgt) {
                    edges.push(DagEdge {
                        source: src.clone(),
                        target: tgt,
                        rel: rel.to_owned(),
                    });
                }
            }
        }
    }

    Ok(RoadmapDag { nodes, edges })
}
