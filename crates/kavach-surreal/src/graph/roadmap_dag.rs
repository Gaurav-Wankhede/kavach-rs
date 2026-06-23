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

    /// Render the decision-architecture slice as a Mermaid `graph TD` for
    /// injection at decision-time. Nodes are `decision`/`roadmap` entries only
    /// (the architecture spine), status-styled so the model sees which choices are
    /// settled (`verified`) versus open (`todo`). Edges kept: `depends_on` (hard
    /// constraint, solid arrow) and `supersedes` (one decision retired another,
    /// dotted "retires"). `focus` (qnames or bare keys) restricts to the relevant
    /// neighbourhood + caps at `max_nodes` for token discipline; an empty `focus`
    /// renders the whole decision spine. Returns `None` when no decision/roadmap
    /// node survives the filter (nothing to show ⇒ inject nothing).
    ///
    /// SOURCE: <https://mermaid.js.org/syntax/flowchart.html>
    #[must_use]
    pub fn decision_mermaid(&self, focus: &[String], max_nodes: usize) -> Option<String> {
        let in_focus = |id: &str| {
            focus.is_empty()
                || focus
                    .iter()
                    .any(|f| id == f || id.rsplit('/').next() == Some(f.as_str()))
        };
        let mut kept: Vec<&DagNode> = self
            .nodes
            .iter()
            .filter(|n| n.category == "decision" || n.category == "roadmap")
            .filter(|n| in_focus(&n.id))
            .collect();
        if kept.is_empty() {
            return None;
        }
        // Highest-signal first: settled decisions anchor the architecture.
        kept.sort_by_key(|n| status_rank(&n.entry_status));
        kept.truncate(max_nodes);
        let keep_ids: std::collections::HashSet<&str> =
            kept.iter().map(|n| n.id.as_str()).collect();

        let mut out = String::from("graph TD\n");
        for n in &kept {
            let id = dm_sanitize(&n.id);
            let label = dm_escape(&n.title);
            let st = n.entry_status.to_uppercase();
            let cls = status_class(&n.entry_status);
            out.push_str("  ");
            out.push_str(&id);
            out.push_str("[\"");
            out.push_str(&label);
            out.push_str("<br/>");
            out.push_str(&st);
            out.push_str("\"]:::");
            out.push_str(cls);
            out.push('\n');
        }
        for e in &self.edges {
            if !keep_ids.contains(e.source.as_str()) || !keep_ids.contains(e.target.as_str()) {
                continue;
            }
            let arrow = match e.rel.as_str() {
                "depends_on" => Some(" --> "),
                "supersedes" => Some(" -.retires.-> "),
                _ => None,
            };
            if let Some(a) = arrow {
                out.push_str("  ");
                out.push_str(&dm_sanitize(&e.source));
                out.push_str(a);
                out.push_str(&dm_sanitize(&e.target));
                out.push('\n');
            }
        }
        out.push_str("  classDef done fill:#1a7f37,color:#fff;\n");
        out.push_str("  classDef open fill:#9a3412,color:#fff;\n");
        out.push_str("  classDef draft fill:#854d0e,color:#fff;\n");
        Some(out)
    }

    /// The retired patterns: every `pattern` node that is the TARGET of a
    /// `supersedes` edge (something replaced it). Returns `(title, retired_by)`
    /// pairs — `retired_by` is the superseding node's title — so a gate can cite
    /// what the codebase moved to. Empty when nothing is retired.
    #[must_use]
    pub fn retired_patterns(&self) -> Vec<(String, String)> {
        let title_of = |id: &str| {
            self.nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.title.clone())
        };
        self.edges
            .iter()
            .filter(|e| e.rel == "supersedes")
            .filter_map(|e| {
                // source supersedes target ⇒ target is retired, source is the
                // replacement. Both must be pattern nodes we know.
                let retired = title_of(&e.target)?;
                let replacement = title_of(&e.source)?;
                Some((retired, replacement))
            })
            .collect()
    }

    /// Render the `pattern`-category slice as a Mermaid `graph TD` supersession
    /// DAG: each pattern node, status-styled by trust (`verified` = soaked,
    /// `todo` = fresh-but-unsoaked), and `supersedes` edges showing which pattern
    /// RETIRED which (`-.retires.->`). This is the research-refreshed pattern
    /// layer's read-side VIEW — the model sees which detector the codebase has
    /// already replaced, so it adopts the current one. `focus` (qnames or bare
    /// keys, same match rule as [`Self::decision_mermaid`]) restricts to the
    /// relevant pattern neighbourhood for token discipline; an empty `focus`
    /// renders the whole pattern layer. `None` when no pattern node survives the
    /// filter. SOURCE: roadmap.unit.mermaid-decision-architecture.
    #[must_use]
    pub fn pattern_dag_mermaid(&self, focus: &[String], max_nodes: usize) -> Option<String> {
        let in_focus = |id: &str| {
            focus.is_empty()
                || focus
                    .iter()
                    .any(|f| id == f || id.rsplit('/').next() == Some(f.as_str()))
        };
        let mut kept: Vec<&DagNode> = self
            .nodes
            .iter()
            .filter(|n| n.category == "pattern")
            .filter(|n| in_focus(&n.id))
            .collect();
        if kept.is_empty() {
            return None;
        }
        // Soaked (verified) patterns first — those are the trusted current set.
        kept.sort_by_key(|n| status_rank(&n.entry_status));
        kept.truncate(max_nodes);
        let keep_ids: std::collections::HashSet<&str> =
            kept.iter().map(|n| n.id.as_str()).collect();

        let mut out = String::from("graph TD\n");
        for n in &kept {
            // Fresh-but-unsoaked patterns (todo) carry a trust tag so the model
            // knows the boundary is live but not yet battle-tested.
            let trust = if n.entry_status == "todo" {
                " (fresh)"
            } else {
                ""
            };
            out.push_str("  ");
            out.push_str(&dm_sanitize(&n.id));
            out.push_str("[\"");
            out.push_str(&dm_escape(&n.title));
            out.push_str(trust);
            out.push_str("\"]:::");
            out.push_str(status_class(&n.entry_status));
            out.push('\n');
        }
        for e in &self.edges {
            if e.rel != "supersedes"
                || !keep_ids.contains(e.source.as_str())
                || !keep_ids.contains(e.target.as_str())
            {
                continue;
            }
            out.push_str("  ");
            out.push_str(&dm_sanitize(&e.source));
            out.push_str(" -.retires.-> ");
            out.push_str(&dm_sanitize(&e.target));
            out.push('\n');
        }
        out.push_str("  classDef done fill:#1a7f37,color:#fff;\n");
        out.push_str("  classDef open fill:#854d0e,color:#fff;\n");
        out.push_str("  classDef draft fill:#9a3412,color:#fff;\n");
        Some(out)
    }
}

/// Sort key: settled decisions first (lower = earlier). `verified` anchors the
/// architecture, `todo` is still in flux.
fn status_rank(status: &str) -> u8 {
    match status {
        "verified" => 0,
        "active" | "done" => 1,
        "todo" => 2,
        _ => 3,
    }
}

/// Map an entry status to its Mermaid `classDef` bucket.
fn status_class(status: &str) -> &'static str {
    match status {
        "verified" | "active" | "done" => "done",
        "todo" => "open",
        _ => "draft",
    }
}

/// Mermaid id sanitizer (mirrors `flow_dag::sanitize_id`; kept local so the two
/// renderers stay independently tunable).
fn dm_sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Escape a label for a Mermaid quoted string.
fn dm_escape(label: &str) -> String {
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
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

#[cfg(test)]
mod decision_mermaid_tests {
    use super::{DagEdge, DagNode, RoadmapDag};

    fn node(id: &str, title: &str, status: &str, cat: &str) -> DagNode {
        DagNode {
            id: id.to_owned(),
            entry_key: id.rsplit('/').next().unwrap_or(id).to_owned(),
            title: title.to_owned(),
            entry_status: status.to_owned(),
            category: cat.to_owned(),
        }
    }

    fn sample() -> RoadmapDag {
        RoadmapDag {
            nodes: vec![
                node("p/decision/paseto", "paseto over jwt", "verified", "decision"),
                node("p/decision/httpsig", "httpsig every call", "verified", "decision"),
                node("p/decision/scylla", "scylla displaces redis", "todo", "decision"),
                node("p/research/x", "noise", "active", "research"),
            ],
            edges: vec![
                DagEdge { source: "p/decision/paseto".into(), target: "p/decision/httpsig".into(), rel: "depends_on".into() },
                DagEdge { source: "p/decision/httpsig".into(), target: "p/decision/scylla".into(), rel: "supersedes".into() },
            ],
        }
    }

    #[test]
    fn renders_status_styled_graph_td() {
        let m = sample().decision_mermaid(&[], 8).expect("non-empty");
        assert!(m.starts_with("graph TD\n"), "{m}");
        assert!(m.contains(":::done"), "verified ⇒ done class: {m}");
        assert!(m.contains(":::open"), "todo ⇒ open class: {m}");
        // depends_on solid arrow, supersedes dotted 'retires'
        assert!(m.contains(" --> "), "{m}");
        assert!(m.contains("-.retires.-> "), "{m}");
        // research noise excluded — only decision/roadmap nodes
        assert!(!m.contains("noise"), "research excluded: {m}");
    }

    #[test]
    fn focus_filter_restricts_nodes() {
        let m = sample().decision_mermaid(&["paseto".to_owned()], 8).expect("non-empty");
        assert!(m.contains("paseto"), "{m}");
        assert!(!m.contains("scylla"), "out-of-focus excluded: {m}");
    }

    #[test]
    fn no_decision_nodes_yields_none() {
        let only_research = RoadmapDag {
            nodes: vec![node("p/research/x", "noise", "active", "research")],
            edges: vec![],
        };
        assert!(only_research.decision_mermaid(&[], 8).is_none());
    }

    #[test]
    fn max_nodes_caps_output() {
        let m = sample().decision_mermaid(&[], 1).expect("non-empty");
        // Only 1 node kept ⇒ at most 1 node line (verified sorts first).
        let node_lines = m.lines().filter(|l| l.contains(":::")).count();
        assert_eq!(node_lines, 1, "{m}");
    }

    fn pattern_sample() -> RoadmapDag {
        RoadmapDag {
            nodes: vec![
                node("p/pattern/dioxus-0.8", "dioxus-0.8 location via BFF", "todo", "pattern"),
                node("p/pattern/dioxus-0.7", "dioxus-0.7 web-sys gap", "verified", "pattern"),
                node("p/decision/x", "noise", "verified", "decision"),
            ],
            edges: vec![DagEdge {
                source: "p/pattern/dioxus-0.8".into(),
                target: "p/pattern/dioxus-0.7".into(),
                rel: "supersedes".into(),
            }],
        }
    }

    #[test]
    fn pattern_dag_renders_supersession_with_trust_tag() {
        let m = pattern_sample().pattern_dag_mermaid(&[], 8).expect("non-empty");
        assert!(m.starts_with("graph TD\n"), "{m}");
        // fresh (todo) pattern carries the (fresh) trust tag
        assert!(m.contains("(fresh)"), "unsoaked pattern tagged: {m}");
        // supersession edge: newer retires older
        assert!(m.contains("-.retires.-> "), "{m}");
        // non-pattern (decision) node excluded
        assert!(!m.contains("noise"), "non-pattern excluded: {m}");
    }

    #[test]
    fn pattern_dag_no_pattern_nodes_yields_none() {
        let only_decision = RoadmapDag {
            nodes: vec![node("p/decision/x", "noise", "verified", "decision")],
            edges: vec![],
        };
        assert!(only_decision.pattern_dag_mermaid(8).is_none());
    }

    #[test]
    fn retired_patterns_returns_target_and_replacement() {
        let r = pattern_sample().retired_patterns();
        assert_eq!(r.len(), 1, "{r:?}");
        // target (0.7 gap) is retired; source (0.8 location) is the replacement.
        assert_eq!(r[0].0, "dioxus-0.7 web-sys gap");
        assert_eq!(r[0].1, "dioxus-0.8 location via BFF");
    }

    #[test]
    fn retired_patterns_empty_without_supersedes() {
        let no_edge = RoadmapDag {
            nodes: vec![node("p/pattern/x", "p", "verified", "pattern")],
            edges: vec![],
        };
        assert!(no_edge.retired_patterns().is_empty());
    }

    // End-to-end: seeded supersedes entity edge -> fetch() -> retired pair.
    // This is the read path the [RETIRED_PATTERN] guard consumes over RPC.
    #[tokio::test]
    async fn fetch_surfaces_supersedes_edge_as_retired_pair() {
        use crate::{apply_schema, open_memory, project_register, upsert_entry_full, upsert_relationships};

        let db = open_memory().await.expect("open mem db");
        apply_schema(&db).await.expect("schema");
        let proj = project_register(&db, "proofproj", "Proof", "/tmp", None)
            .await
            .expect("register project");

        for (key, title) in [
            ("old-pat", "old pattern: web-sys gap"),
            ("new-pat", "new pattern: router hook"),
        ] {
            let qn = format!("proofproj/pattern/{key}");
            upsert_entry_full()
                .db(&db)
                .category("pattern")
                .project_id(&proj)
                .entry_key(key)
                .title(title)
                .content("c")
                .event_source("test")
                .qualified_name(&qn)
                .references(&[])
                .build_for_call()
                .await
                .expect("upsert pattern");
        }

        let n = upsert_relationships(
            &db,
            "proofproj/pattern/new-pat",
            &[(
                "supersedes".to_owned(),
                "proofproj/pattern/old-pat".to_owned(),
            )],
        )
        .await
        .expect("relate");
        assert_eq!(n, 1, "one supersedes edge written");

        let dag = super::fetch(&db, "proofproj").await.expect("fetch");
        assert_eq!(dag.nodes.len(), 2, "two pattern nodes: {:?}", dag.nodes);
        assert!(
            dag.edges.iter().any(|e| e.rel == "supersedes"),
            "supersedes edge survived into dag: {:?}",
            dag.edges
        );
        let retired = dag.retired_patterns();
        assert_eq!(retired.len(), 1, "exactly one retired pair: {retired:?}");
        assert_eq!(retired[0].0, "old pattern: web-sys gap", "retired title");
        assert_eq!(retired[0].1, "new pattern: router hook", "replacement title");
    }
}
