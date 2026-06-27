// DAG projection of the kanban — the AWARENESS view. The flat board (render.rs)
// answers "what's the status?"; this answers "what can I work on NOW and what is
// it waiting on?" — the dependency question the linear list is blind to.
//
// Reuses the SAME RoadmapDag the scheduler reads (roadmap_dag_fetch +
// toposort_or_cycle), so the CLI view and the dispatch order never disagree.
// Two emit formats: `dag` = topo-tiered text (tier 0 = ready now), `mermaid` =
// flowchart TD for human/GUI viewing. Mermaid is a picture; tiered text is the
// awareness signal (cheap to inject, names READY/BLOCKED/CYCLE inline).
use std::collections::HashMap;
use std::fmt::Write as _;
use kavach_surreal::graph::roadmap_dag::TopoOrder;
use kavach_surreal::{DagEdge, DagNode, MemoryEntry, RoadmapDag};
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
/// Build a `RoadmapDag` from the roadmap ROWS (`list_by_project`) — the same
/// source the flat board and the scheduler read — instead of the entity-graph
/// mirror, which is unpopulated for roadmap cards. Edges come from the cards'
/// declared `DEPENDS_ON:`/`BLOCKED_BY:` lines via the scheduler's own parser
/// (`parse_declared_deps`), so the awareness DAG and dispatch order agree by
/// construction. A declared dep whose key is absent from the project is dropped
/// (it cannot be a node), mirroring the scheduler's tolerance.
fn dag_from_roadmap(rows: &[MemoryEntry]) -> RoadmapDag {
    use kavach_rpc::methods::roadmap::readiness::parse_declared_deps;
    let present: std::collections::HashSet<&str> =
        rows.iter().map(|e| e.entry_key.as_str()).collect();
    let nodes: Vec<DagNode> = rows
        .iter()
        .map(|e| DagNode {
            id: e.entry_key.clone(),
            entry_key: e.entry_key.clone(),
            title: e.title.clone(),
            entry_status: e.entry_status_str().to_owned(),
            category: "roadmap".to_owned(),
        })
        .collect();
    let mut nodes = nodes;
    let mut edges: Vec<DagEdge> = Vec::new();
    let mut phantom_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in rows {
        // prerequisite -> dependent: each declared dep must finish before `e`.
        for dep in parse_declared_deps(&e.content) {
            if !present.contains(dep.as_str()) {
                // Absent in THIS project's rows — most often a cross-project
                // prerequisite the scheduler resolves against the GLOBAL pool and
                // would BLOCK on. Drop-the-edge would falsely mark `e` ready
                // (Finding B). Instead inject a phantom prereq node whose status
                // is never verified/done, so `is_ready` correctly returns false
                // and the missing key is named — matching dispatch (fail-safe).
                if phantom_seen.insert(dep.clone()) {
                    nodes.push(DagNode {
                        id: dep.clone(),
                        entry_key: dep.clone(),
                        title: "(unresolved — cross-project or missing)".to_owned(),
                        entry_status: "missing".to_owned(),
                        category: "roadmap".to_owned(),
                    });
                }
            }
            edges.push(DagEdge {
                source: dep,
                target: e.entry_key.clone(),
                rel: "depends_on".to_owned(),
            });
        }
    }
    RoadmapDag { nodes, edges }
}
/// Only these relations are dependency edges (prerequisite -> dependent). Mirror
/// of the scheduler's edge filter in `roadmap_dag.rs::toposort_or_cycle`.
const DEP_RELS: &[&str] = &["depends_on", "blocks"];
/// Assign each node its dependency depth (tier): tier 0 has no unmet
/// prerequisite among the dependency edges; tier N unlocks once tier <N closes.
///
/// Computed from the topo order so it is cycle-safe: a node in a cycle never
/// receives a tier (it is absent from `Ordered`), which is exactly why the
/// caller renders the cycle separately rather than placing it on a tier.
fn tiers(order: &[String], edges: &[DagEdge]) -> HashMap<String, usize> {
    // prereqs[node] = the set of nodes that must finish before it.
    let mut prereqs: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        if DEP_RELS.contains(&e.rel.as_str()) {
            prereqs
                .entry(e.target.as_str())
                .or_default()
                .push(e.source.as_str());
        }
    }
    let mut depth: HashMap<String, usize> = HashMap::new();
    // `order` is prerequisites-first, so every prereq's depth is known before
    // the dependent is visited — one pass suffices, no fixpoint needed.
    for id in order {
        let d = prereqs.get(id.as_str()).map_or(0, |ps| {
            ps.iter()
                .filter_map(|p| depth.get(*p))
                .max()
                .map_or(0, |m| m.saturating_add(1))
        });
        depth.insert(id.clone(), d);
    }
    depth
}
/// True when `node` is dispatchable (`todo`/`in_progress` with unmet prerequisites
/// = 0) — i.e. every node it depends on is already `verified`/`done` AND the
/// node itself is not yet closed. That is the "ready to dispatch NOW" signal.
/// Closed nodes (verified/done) are NOT dispatchable, even if deps are met.
fn is_ready(node: &DagNode, edges: &[DagEdge], by_id: &HashMap<&str, &DagNode>) -> bool {
    // Node must be dispatchable (not closed).
    if matches!(node.entry_status.as_str(), "verified" | "done") {
        return false;
    }
    // All prerequisites must be satisfied.
    edges
        .iter()
        .filter(|e| DEP_RELS.contains(&e.rel.as_str()) && e.target == node.id)
        .all(|e| {
            by_id
                .get(e.source.as_str())
                .is_none_or(|src| matches!(src.entry_status.as_str(), "verified" | "done"))
        })
}
/// Inline `depends-on` suffix for a node (the prereq keys), or empty when none.
fn deps_suffix(node: &DagNode, edges: &[DagEdge], by_id: &HashMap<&str, &DagNode>) -> String {
    let prereqs: Vec<&str> = edges
        .iter()
        .filter(|e| DEP_RELS.contains(&e.rel.as_str()) && e.target == node.id)
        .filter_map(|e| by_id.get(e.source.as_str()).map(|n| n.entry_key.as_str()))
        .collect();
    if prereqs.is_empty() {
        String::new()
    } else {
        format!("  ⤷ depends-on: {}", prereqs.join(", "))
    }
}
/// Render the topo-tiered text DAG. Returns the assembled string (caller prints).
fn render_tiered_text(dag: &RoadmapDag) -> String {
    let by_id: HashMap<&str, &DagNode> = dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out = format!(
        "[KANBAN DAG] {} node(s), {} edge(s)\n",
        dag.nodes.len(),
        dag.edges
            .iter()
            .filter(|e| DEP_RELS.contains(&e.rel.as_str()))
            .count()
    );
    let order = match dag.toposort_or_cycle() {
        TopoOrder::Ordered(o) => o,
        TopoOrder::Cycle(cycle) => {
            // Fail-loud: a cycle is a deadlock — name the keys, render nothing
            // else as "ready" (none in a cycle can be).
            let keys: Vec<&str> = cycle
                .iter()
                .filter_map(|id| by_id.get(id.as_str()).map(|n| n.entry_key.as_str()))
                .collect();
            writeln!(out, "⚠ CYCLE (deadlock): {}", keys.join(", ")).ok();
            return out;
        }
    };
    let depth = tiers(&order, &dag.edges);
    let max_tier = depth.values().copied().max().unwrap_or(0);
    for tier in 0..=max_tier {
        let mut ids: Vec<&String> = order
            .iter()
            .filter(|id| depth.get(*id) == Some(&tier))
            // Exclude closed nodes (verified/done) from tier display.
            .filter(|id| {
                by_id
                    .get(id.as_str())
                    .is_none_or(|n| !matches!(n.entry_status.as_str(), "verified" | "done"))
            })
            .collect();
        ids.sort();
        if ids.is_empty() {
            continue;
        }
        let label = if tier == 0 { " — ready now" } else { "" };
        writeln!(out, "TIER {tier}{label}").ok();
        for id in ids {
            if let Some(n) = by_id.get(id.as_str()) {
                let marker = if is_ready(n, &dag.edges, &by_id) {
                    "✓READY"
                } else {
                    "⏳WAITING"
                };
                writeln!(
                    out,
                    "  [{}] {} — {}  {marker}{}",
                    n.entry_status,
                    n.entry_key,
                    n.title,
                    deps_suffix(n, &dag.edges, &by_id)
                )
                .ok();
            }
        }
    }
    out
}
/// Sanitize a node id into a Mermaid-safe identifier (alphanumerics + `_`).
fn mermaid_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}
/// Render a `flowchart TD` of the cards + dependency edges (human/GUI export).
fn render_mermaid(dag: &RoadmapDag) -> String {
    let by_id: HashMap<&str, &DagNode> = dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out = String::from("flowchart TD\n");
    for n in &dag.nodes {
        writeln!(
            out,
            "  {}[\"{}<br/>{}\"]",
            mermaid_id(&n.id),
            n.entry_key.replace('"', "'"),
            n.entry_status
        )
        .ok();
    }
    for e in &dag.edges {
        if DEP_RELS.contains(&e.rel.as_str())
            && by_id.contains_key(e.source.as_str())
            && by_id.contains_key(e.target.as_str())
        {
            // prerequisite -> dependent: source must finish before target.
            writeln!(
                out,
                "  {} --> {}",
                mermaid_id(&e.source),
                mermaid_id(&e.target)
            )
            .ok();
        }
    }
    out
}
/// Entry point: build the DAG from roadmap `rows`, render it in the requested
/// `format`, and print it. `format` is `"dag"` (tiered text) or `"mermaid"`;
/// any other value is treated as `"dag"` (clap restricts the flag, so this is
/// defense-in-depth).
pub(in crate::cmd::db) fn render_dag_from_rows(rows: &[MemoryEntry], format: &str) -> i32 {
    let dag = dag_from_roadmap(rows);
    let body = if format == "mermaid" {
        render_mermaid(&dag)
    } else {
        render_tiered_text(&dag)
    };
    match print_or_exit(body.trim_end()) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
#[cfg(test)]
#[path = "dag_render_test.rs"]
mod tests;
