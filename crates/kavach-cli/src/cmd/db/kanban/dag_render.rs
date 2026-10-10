// DAG projection of the kanban — the AWARENESS view. The flat board (render.rs)
// answers "what's the status?"; this answers "what can I work on NOW and what is
// it waiting on?" — the dependency question the linear list is blind to.
//
// Reuses the SAME RoadmapDag the scheduler reads (roadmap_dag_fetch +
// toposort_or_cycle), so the CLI view and the dispatch order never disagree.
// Two emit formats: `dag` = topo-tiered text (tier 0 = ready now), `mermaid` =
// flowchart TD for human/GUI viewing. Mermaid is a picture; tiered text is the
// awareness signal (cheap to inject, names READY/BLOCKED/CYCLE inline).
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use kavach_db::graph::roadmap_dag::TopoOrder;
use kavach_db::{DagEdge, DagNode, MemoryEntry, RoadmapDag};
use std::collections::HashMap;
use std::fmt::Write as _;
/// Build a `RoadmapDag` from the roadmap ROWS (`list_by_project`) — the same
/// source the flat board and the scheduler read — instead of the entity-graph
/// mirror, which is unpopulated for roadmap cards. Edges come from the cards'
/// declared `DEPENDS_ON:` lines via the scheduler's own parser
/// (`parse_declared_deps`), so the awareness DAG and dispatch order agree by
/// construction. A declared dep whose key is absent from the project is dropped
/// (it cannot be a node), mirroring the scheduler's tolerance.
fn dag_from_roadmap(rows: &[MemoryEntry]) -> RoadmapDag {
    use kavach_rpc::methods::roadmap::readiness::{bare_dep_key, parse_declared_deps};
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
        //
        // Normalize EVERY declared dep to its bare `entry_key` tail first. The
        // daemon sink mirrors graph-edge targets into the `DEPENDS_ON:` line, and
        // those targets arrive project-qualified (`<project>/<category>/<key>`),
        // while `present` holds bare keys. Without this the qualified form never
        // matched, so every `--depends-on` edge ALSO rendered as a phantom
        // "(unresolved)" node — a permanent 230-node/165-edge DAG with half its
        // nodes bogus (`r-kavach-depends-on-ghost-nodes`).
        for dep in parse_declared_deps(&e.content) {
            let dep = bare_dep_key(dep.trim()).to_owned();
            if dep.is_empty() {
                continue;
            }
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
        // `conflicts_with` is declared ONLY inside a MicroTicket body (it is not a
        // `DEPENDS_ON:` line), so it is extracted here rather than by the shared
        // dep parser. Unknown-field tolerance is intentional: a malformed or
        // non-ticket body simply contributes no conflict edges rather than
        // failing the whole board render.
        for other in declared_conflicts(&e.content) {
            // Normalize to the bare entry_key tail — `conflicts_with` is written
            // by `kavach db mt new` as a bare key, but a hand-edited body may
            // carry a qualified one, and an unmatched key would silently fail to
            // block anything.
            edges.push(DagEdge {
                source: e.entry_key.clone(),
                target: bare_dep_key(other.trim()).to_owned(),
                rel: CONFLICT_REL.to_owned(),
            });
        }
    }
    RoadmapDag { nodes, edges }
}

/// Pull `conflicts_with` keys out of a `MTICKET v1` body.
///
/// Returns bare keys (the caller normalizes). An unparseable body yields an
/// empty vec — a card written before the schema existed must not break the board.
fn declared_conflicts(content: &str) -> Vec<String> {
    let Some(json) = content
        .trim_start()
        .strip_prefix("MTICKET v1")
        .map(str::trim)
    else {
        return Vec::new();
    };
    kavach_db::microticket::MicroTicket::from_json(json)
        .map(|t| t.conflicts_with)
        .unwrap_or_default()
}
/// Only these relations are dependency edges (prerequisite -> dependent). Mirror
/// of the scheduler's edge filter in `roadmap_dag.rs::toposort_or_cycle`.
const DEP_RELS: &[&str] = &["depends_on", "blocks"];

/// Mutual exclusion: two tickets linked by `conflicts_with` must NEVER be
/// dispatchable at the same time. Distinct from `alternative_to` in the ontology
/// set — that means "both are valid options", this means "doing A makes B wrong".
/// A parallel runner that picks both is precisely the bug this relation exists
/// to prevent, so it is enforced in `is_ready`, not merely displayed.
const CONFLICT_REL: &str = "conflicts_with";

/// A node is conflict-blocked if an OPEN card claims conflict with it.
///
/// Symmetry: `A conflicts_with B` is written on A, but the exclusion must apply
/// to BOTH, or the pairing could be dodged by declaring it on whichever side is
/// dispatched second.
fn conflict_blocked(node_id: &str, edges: &[DagEdge], by_id: &HashMap<&str, &DagNode>) -> bool {
    edges
        .iter()
        .filter(|e| e.rel == CONFLICT_REL)
        .any(|e| {
            // Either endpoint is this node...
            let touches = e.source == node_id || e.target == node_id;
            // ...and the OTHER endpoint is still open (conflict resolves on close).
            touches
                && [e.source.as_str(), e.target.as_str()]
                    .iter()
                    .filter(|k| **k != node_id)
                    .filter_map(|k| by_id.get(k))
                    .any(|n| !matches!(n.entry_status.as_str(), "verified" | "done"))
        })
}
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
    // Mutual exclusion beats everything: a card that conflicts with another
    // OPEN card is never dispatchable, whatever its dependencies say. Enforced
    // here rather than only rendered, so `parallel-run` (which picks from this
    // same readiness signal) can never select both sides of a conflict.
    if conflict_blocked(&node.id, edges, by_id) {
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
/// Inline `conflicts-with` suffix naming the OPEN card(s) that block this one,
/// or empty when unblocked. Mirrors [`deps_suffix`].
fn conflicts_suffix(node: &DagNode, edges: &[DagEdge], by_id: &HashMap<&str, &DagNode>) -> String {
    let others: Vec<&str> = edges
        .iter()
        .filter(|e| e.rel == CONFLICT_REL && (e.source == node.id || e.target == node.id))
        .filter_map(|e| {
            [e.source.as_str(), e.target.as_str()]
                .into_iter()
                .find(|k| *k != node.id)
                .and_then(|k| by_id.get(k))
                .map(|n| n.entry_key.as_str())
        })
        .filter(|k| {
            by_id
                .get(*k)
                .is_none_or(|n| !matches!(n.entry_status.as_str(), "verified" | "done"))
        })
        .collect();
    if others.is_empty() {
        String::new()
    } else {
        format!("  ⊘ conflicts-with: {}", others.join(", "))
    }
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
/// `show_closed` keeps `done`/`verified` nodes in their tiers (K-5): the
/// default awareness view hides closed rows, but an explicit
/// `--status done|verified` filter asks for exactly those rows.
fn render_tiered_text(dag: &RoadmapDag, show_closed: bool) -> String {
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
            // Closed nodes (verified/done) are excluded from tier display
            // unless explicitly requested via `--status done|verified`.
            .filter(|id| {
                show_closed
                    || by_id
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
                // A blocked-by-conflict card says WHY it is waiting — an
                // unexplained WAITING on a card with no unmet deps is the same
                // silent-wrongness class as the phantom-node bug.
                let conflict_note = if conflict_blocked(&n.id, &dag.edges, &by_id) {
                    conflicts_suffix(n, &dag.edges, &by_id)
                } else {
                    String::new()
                };
                writeln!(
                    out,
                    "  [{}] {} — {}  {marker}{}{}",
                    n.entry_status,
                    n.entry_key,
                    n.title,
                    deps_suffix(n, &dag.edges, &by_id),
                    conflict_note
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
    render_dag_from_filtered(rows, format, false, 0)
}

/// Filtered entry point (K-5): render the DAG over pre-filtered `rows`
/// (status/key/lane already applied by the caller). `show_closed` keeps
/// `done`/`verified` tiers visible; `limit` caps rendered nodes (0 = all).
/// An empty filtered set reports itself instead of the full-graph view so a
/// lane filter is verifiable from the output.
pub(in crate::cmd::db) fn render_dag_from_filtered(
    rows: &[MemoryEntry],
    format: &str,
    show_closed: bool,
    limit: usize,
) -> i32 {
    if rows.is_empty() {
        return match print_or_exit("[KANBAN DAG] 0 node(s), 0 edge(s) — no rows match the filters")
        {
            Ok(()) => 0,
            Err(io_err) => into_exit_code(io_err),
        };
    }
    let scoped: Vec<MemoryEntry> = if limit == 0 {
        rows.to_vec()
    } else {
        rows.iter().take(limit).cloned().collect()
    };
    let dag = dag_from_roadmap(&scoped);
    let body = if format == "mermaid" {
        render_mermaid(&dag)
    } else {
        render_tiered_text(&dag, show_closed)
    };
    match print_or_exit(body.trim_end()) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
#[cfg(test)]
#[path = "dag_render_test.rs"]
mod tests;
