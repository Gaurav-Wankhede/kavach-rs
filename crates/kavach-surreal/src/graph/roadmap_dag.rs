// split: DAG types + their cohesive topo method + the single fetch that
// produces them — one graph-projection module, not mixed service concerns.
// Native-SurrealDB project-scoped concept graph fetch.
//
// Returns ALL memory entities for a project (across roadmap / decision /
// research / pattern / app_spec) plus every inter-entry edge of the 5
// supported relation types.
//
// ARCH: ProjectGraphFetch
// PATTERN: single-roundtrip-graph-projection
// SCOPE: scale (read path; 5s polling from desktop app)
// CAPACITY: <= 2000 nodes/project, <= 10000 edges/project (current max ~1500/424)
// QPS: 0.2 (single-user desktop) | LATENCY: <100ms p99 local RocksDB
// CONSISTENCY: snapshot-on-fetch; eventual consistency with concurrent writers
// FAILURE_MODE: project missing -> empty default; edge query failure -> nodes only
// OBSERVABILITY: caller (knowledge.rs) tracing::error on Err
//           one giant join; clearer types and small constant factor accepted.
//
//   {"name":"per_node_get_related","reason":"O(n) round-trips; rejected"},
//   {"name":"category_partition_then_union","reason":"5 separate edge queries -> 5x RTT"}
// ]
// TIME: O(n + e) | SPACE: O(n + e)
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: https://surrealdb.com/docs/learn/data-models/graph/overview
// SOURCE: https://surrealdb.com/docs/surrealql/statements/select
use crate::error::Result;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by kavach-engine scheduler tests + fetch(); non_exhaustive => E0639"
)]
pub struct DagNode {
    pub id: String,
    pub entry_key: String,
    pub title: String,
    pub entry_status: String,
    pub category: String,
}

#[derive(Debug, Clone, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by kavach-engine scheduler tests + fetch(); non_exhaustive => E0639"
)]
pub struct DagEdge {
    pub source: String,
    pub target: String,
    pub rel: String,
}

#[derive(Debug, Clone, Default, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by kavach-engine scheduler tests + fetch(); non_exhaustive => E0639"
)]
pub struct RoadmapDag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
}

/// Topological-sort outcome for the dependency DAG.
///
/// A cycle is a deadlock: no node in the cycle can ever become ready, so the
/// scheduler must reject the dispatch and name the offending keys rather than
/// spin forever.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched cross-crate by kavach-engine DagScheduler; non_exhaustive => E0004"
)]
pub enum TopoOrder {
    /// Dependency-respecting order: prerequisites precede dependents.
    Ordered(Vec<String>),
    /// The dependency graph has a cycle; these node ids participate in it.
    Cycle(Vec<String>),
}

impl RoadmapDag {
    /// Kahn's algorithm over the dependency edges (`depends_on` ∪ `blocks`).
    /// Returns [`TopoOrder::Ordered`] (prereqs first) or [`TopoOrder::Cycle`]
    /// with the residual nodes when a cycle makes a full ordering impossible.
    ///
    /// ALGO: Kahn topological sort | `PROBLEM_CLASS`: graph DAG validation
    /// TIME: O(n + e) | SPACE: O(n + e) | YEAR: 2026
    /// SOURCE: <https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm>
    #[must_use]
    pub fn toposort_or_cycle(&self) -> TopoOrder {
        use std::collections::{HashMap, VecDeque};
        // Dependency edge: a `depends_on`/`blocks` edge means source must
        // finish before target. in_degree counts unmet prerequisites.
        let mut in_deg: HashMap<&str, usize> =
            self.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &self.edges {
            if e.rel != "depends_on" && e.rel != "blocks" {
                continue;
            }
            if !in_deg.contains_key(e.source.as_str()) || !in_deg.contains_key(e.target.as_str()) {
                continue;
            }
            adj.entry(e.source.as_str())
                .or_default()
                .push(e.target.as_str());
            let d = in_deg.entry(e.target.as_str()).or_insert(0);
            *d = d.saturating_add(1);
        }
        let mut queue: VecDeque<&str> = in_deg
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(&k, _)| k)
            .collect();
        let mut ordered: Vec<String> = Vec::with_capacity(self.nodes.len());
        while let Some(n) = queue.pop_front() {
            ordered.push(n.to_owned());
            if let Some(succ) = adj.get(n) {
                for &m in succ {
                    if let Some(d) = in_deg.get_mut(m) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(m);
                        }
                    }
                }
            }
        }
        if ordered.len() == self.nodes.len() {
            TopoOrder::Ordered(ordered)
        } else {
            // Residual nodes (in_degree never hit 0) are the cycle.
            let cycle = in_deg
                .iter()
                .filter(|&(_, &d)| d > 0)
                .map(|(&k, _)| k.to_owned())
                .collect();
            TopoOrder::Cycle(cycle)
        }
    }
}

#[derive(surrealdb_types::SurrealValue)]
struct IdRow {
    id: surrealdb_types::RecordId,
}

#[derive(Deserialize, surrealdb_types::SurrealValue)]
struct MetaRow {
    entry_key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    entry_status: String,
}

#[derive(Deserialize, surrealdb_types::SurrealValue, Default)]
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
