// ALGO: FruchtermanReingold
// PROBLEM_CLASS: graph-layout
// REJECTED: [{"name":"cytoscape_js_interop","reason":"adds a JS dependency to a native Rust app; user chose native SVG"},{"name":"fdg_crate","reason":"pulls petgraph + nalgebra for ~40 lines we can own; no new dep"}]
// TIME: O(iters * n^2) for repulsion | SPACE: O(n+e)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: O(n^2) repulsion is fine at the 200-node cap; past that, switch to a Barnes-Hut quadtree.
// SOURCE: https://docs.rs/dioxus-html/latest/dioxus_html/elements/svg/index.html
use serde::{Deserialize, Serialize};

use crate::rpc_client::{Error as RpcError, rpc};

#[derive(Debug, Serialize)]
struct GraphFetchParams {
    entity_type: Option<String>,
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct GraphNodeDto {
    id: String,
    label: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct GraphEdgeDto {
    from: String,
    to: String,
    #[expect(
        dead_code,
        reason = "deserialized from the RPC payload; kept for forward-compat edge tooltips, not yet rendered"
    )]
    rel: String,
}

#[derive(Debug, Deserialize)]
struct GraphFetchResultDto {
    success: bool,
    nodes: Vec<GraphNodeDto>,
    edges: Vec<GraphEdgeDto>,
    total: usize,
    #[serde(default)]
    error: Option<String>,
}

/// A node with a computed canvas position. `x`/`y` are in the layout's local
/// coordinate space; the renderer fits them to the SVG viewBox.
#[derive(Clone, PartialEq)]
pub struct PlacedNode {
    pub label: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
}

/// An edge resolved to the endpoint positions, ready to draw as a line.
#[derive(Clone, PartialEq)]
pub struct PlacedEdge {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Clone, PartialEq)]
pub enum LoadState {
    Ok {
        nodes: Vec<PlacedNode>,
        edges: Vec<PlacedEdge>,
        shown: usize,
        total: usize,
    },
    DaemonOffline,
    /// The graph genuinely has no entities for this filter (not an error).
    Empty,
    /// The fetch failed; carries a user-facing reason so the UI shows a real
    /// error instead of an indistinguishable "empty" state.
    Failed(String),
}

/// Canvas bounds the layout targets and the renderer's viewBox matches.
pub const CANVAS_W: f64 = 1000.0;
pub const CANVAS_H: f64 = 700.0;
const NODE_CAP: usize = 200;

/// Fetch the typed graph and run a force-directed layout. Returns positioned
/// nodes + edges so the renderer is a pure painter with no layout logic.
pub fn load(slug: &str, etype: Option<&str>) -> LoadState {
    if slug.is_empty() {
        return LoadState::Empty;
    }
    let res = rpc::<GraphFetchParams, GraphFetchResultDto>(
        "db.graph_fetch",
        GraphFetchParams {
            entity_type: etype.map(str::to_owned),
            limit: NODE_CAP,
        },
    );
    let dto = match res {
        Ok(r) if r.success => r,
        Ok(r) => {
            let msg = r
                .error
                .unwrap_or_else(|| "graph fetch reported failure".to_owned());
            tracing::error!(error = %msg, "db.graph_fetch !success");
            return LoadState::Failed(msg);
        }
        Err(RpcError::DaemonOffline(_)) => return LoadState::DaemonOffline,
        Err(e) => {
            tracing::error!(error = %e, "db.graph_fetch failed");
            return LoadState::Failed(e.to_string());
        }
    };
    if dto.nodes.is_empty() {
        return LoadState::Empty;
    }
    let total = dto.total;
    let shown = dto.nodes.len();
    let (nodes, edges) = layout::run(&dto.nodes, &dto.edges);
    LoadState::Ok {
        nodes,
        edges,
        shown,
        total,
    }
}

/// Force-directed layout math, isolated so the unavoidable float arithmetic of
/// a physics simulation is suppressed in one bounded place — not scattered as
/// per-line attributes across the page. The four restriction lints below are
/// inherent to any force layout (it IS iterated float math over fixed-length
/// vectors); the bounds-correctness is guaranteed structurally (every index is
/// `< n`, every vec has length `n`), which clippy cannot prove.
mod layout {
    #![expect(
        clippy::float_arithmetic,
        reason = "Fruchterman-Reingold is a physics simulation; float math is its substance"
    )]
    #![expect(
        clippy::arithmetic_side_effects,
        reason = "displacement accumulation over f64; overflow is not a concern at canvas scale"
    )]
    #![expect(
        clippy::indexing_slicing,
        reason = "pos/disp are length-n; every index derives from 0..n, provably in-bounds"
    )]
    #![expect(
        clippy::cast_precision_loss,
        reason = "node counts are capped at 200; usize->f64 is exact well below the 2^52 mantissa limit"
    )]

    use super::{CANVAS_H, CANVAS_W, GraphEdgeDto, GraphNodeDto, PlacedEdge, PlacedNode};

    const FR_ITERS: usize = 120;

    /// Deterministic Fruchterman-Reingold. No RNG (banned in this environment
    /// and it would make layouts jitter between renders) — seed positions on a
    /// circle by index, which converges to a stable, reproducible layout.
    pub(super) fn run(
        nodes: &[GraphNodeDto],
        edges: &[GraphEdgeDto],
    ) -> (Vec<PlacedNode>, Vec<PlacedEdge>) {
        let n = nodes.len();
        let area = CANVAS_W * CANVAS_H;
        // Ideal edge length: the classic k = sqrt(area / n).
        let k = (area / n as f64).sqrt();

        let mut pos = seed_circle(n);
        let edge_pairs = resolve_edges(nodes, edges);

        let mut temp = CANVAS_W / 10.0;
        let cooling = temp / (FR_ITERS as f64 + 1.0);

        for _ in 0..FR_ITERS {
            let mut disp = vec![(0.0_f64, 0.0_f64); n];
            apply_repulsion(&pos, &mut disp, k, n);
            apply_attraction(&pos, &mut disp, &edge_pairs, k);
            apply_displacement(&mut pos, &disp, temp, n);
            temp = (temp - cooling).max(1.0);
        }

        let placed = nodes
            .iter()
            .zip(pos.iter())
            .map(|(nd, &(x, y))| PlacedNode {
                label: nd.label.clone(),
                kind: nd.kind.clone(),
                x,
                y,
            })
            .collect();
        let placed_edges = edge_pairs
            .iter()
            .map(|&(a, b)| PlacedEdge {
                x1: pos[a].0,
                y1: pos[a].1,
                x2: pos[b].0,
                y2: pos[b].1,
            })
            .collect();
        (placed, placed_edges)
    }

    /// Seed on a circle so the layout is deterministic and edges have something
    /// to pull against from iteration 0.
    fn seed_circle(n: usize) -> Vec<(f64, f64)> {
        let cx = CANVAS_W / 2.0;
        let cy = CANVAS_H / 2.0;
        let radius = CANVAS_H.min(CANVAS_W) * 0.4;
        (0..n)
            .map(|i| {
                let theta = (i as f64) * std::f64::consts::TAU / (n as f64);
                (cx + radius * theta.cos(), cy + radius * theta.sin())
            })
            .collect()
    }

    /// Map node id -> index, then resolve edge endpoints, dropping any edge
    /// whose target is off-canvas (not in the node set) or a self-loop.
    fn resolve_edges(nodes: &[GraphNodeDto], edges: &[GraphEdgeDto]) -> Vec<(usize, usize)> {
        let idx: std::collections::HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, nd)| (nd.id.as_str(), i))
            .collect();
        edges
            .iter()
            .filter_map(|e| Some((*idx.get(e.from.as_str())?, *idx.get(e.to.as_str())?)))
            .filter(|(a, b)| a != b)
            .collect()
    }

    /// Repulsive forces between every pair: fr = k^2 / d.
    fn apply_repulsion(pos: &[(f64, f64)], disp: &mut [(f64, f64)], k: f64, n: usize) {
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let mag = dx.hypot(dy).max(0.01);
                let force = (k * k) / mag;
                let (ux, uy) = (dx / mag, dy / mag);
                disp[i].0 = ux.mul_add(force, disp[i].0);
                disp[i].1 = uy.mul_add(force, disp[i].1);
                disp[j].0 = ux.mul_add(-force, disp[j].0);
                disp[j].1 = uy.mul_add(-force, disp[j].1);
            }
        }
    }

    /// Attractive forces along edges: fa = d^2 / k.
    fn apply_attraction(
        pos: &[(f64, f64)],
        disp: &mut [(f64, f64)],
        edge_pairs: &[(usize, usize)],
        k: f64,
    ) {
        for &(a, b) in edge_pairs {
            let dx = pos[a].0 - pos[b].0;
            let dy = pos[a].1 - pos[b].1;
            let mag = dx.hypot(dy).max(0.01);
            let force = (mag * mag) / k;
            let (ux, uy) = (dx / mag, dy / mag);
            disp[a].0 = ux.mul_add(-force, disp[a].0);
            disp[a].1 = uy.mul_add(-force, disp[a].1);
            disp[b].0 = ux.mul_add(force, disp[b].0);
            disp[b].1 = uy.mul_add(force, disp[b].1);
        }
    }

    /// Apply displacement capped by the cooling temperature, clamped to canvas.
    fn apply_displacement(pos: &mut [(f64, f64)], disp: &[(f64, f64)], temp: f64, n: usize) {
        for i in 0..n {
            let d = disp[i]
                .0
                .mul_add(disp[i].0, disp[i].1 * disp[i].1)
                .sqrt()
                .max(0.01);
            let limited = d.min(temp);
            pos[i].0 = (disp[i].0 / d)
                .mul_add(limited, pos[i].0)
                .clamp(20.0, CANVAS_W - 20.0);
            pos[i].1 = (disp[i].1 / d)
                .mul_add(limited, pos[i].1)
                .clamp(20.0, CANVAS_H - 20.0);
        }
    }
}

/// Color a node by its kind so the canvas is legible at a glance. Falls back to
/// a neutral gray for kinds we don't special-case.
pub fn color_for(kind: &str) -> &'static str {
    match kind {
        "concept" => "#5b8def",
        "skill" => "#2ec4b6",
        "mistake_event" | "anti_pattern" => "#e63946",
        "file" => "#8d99ae",
        "decision" => "#f4a261",
        "roadmap" | "roadmap_unit" => "#9b5de5",
        "research" => "#06d6a0",
        "project" => "#ffd166",
        _ => "#6c757d",
    }
}

#[cfg(test)]
mod tests {
    use super::layout;
    use super::{CANVAS_H, CANVAS_W, GraphEdgeDto, GraphNodeDto};

    fn node(id: &str) -> GraphNodeDto {
        GraphNodeDto {
            id: id.to_owned(),
            label: id.to_owned(),
            kind: "concept".to_owned(),
        }
    }

    fn edge(from: &str, to: &str) -> GraphEdgeDto {
        GraphEdgeDto {
            from: from.to_owned(),
            to: to.to_owned(),
            rel: "relates".to_owned(),
        }
    }

    #[test]
    fn layout_places_every_node_inside_the_canvas() {
        let nodes = vec![node("a"), node("b"), node("c"), node("d")];
        let edges = vec![edge("a", "b"), edge("b", "c")];
        let (placed, placed_edges) = layout::run(&nodes, &edges);
        assert_eq!(placed.len(), 4, "every node gets a position");
        assert_eq!(placed_edges.len(), 2, "both in-set edges are resolved");
        for p in &placed {
            assert!(
                (20.0..=CANVAS_W - 20.0).contains(&p.x),
                "x in bounds: {}",
                p.x
            );
            assert!(
                (20.0..=CANVAS_H - 20.0).contains(&p.y),
                "y in bounds: {}",
                p.y
            );
            assert!(p.x.is_finite() && p.y.is_finite(), "no NaN/inf positions");
        }
    }

    #[test]
    fn layout_is_deterministic() {
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![edge("a", "c")];
        let run1 = layout::run(&nodes, &edges).0;
        let run2 = layout::run(&nodes, &edges).0;
        for (p, q) in run1.iter().zip(run2.iter()) {
            assert!(
                (p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9,
                "no RNG jitter"
            );
        }
    }

    #[test]
    fn edges_to_offcanvas_targets_are_dropped() {
        // An edge pointing at a node not in the set must not produce a line.
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("a", "ghost")];
        let (_, placed_edges) = layout::run(&nodes, &edges);
        assert!(placed_edges.is_empty(), "off-canvas edge dropped");
    }

    #[test]
    fn single_node_lays_out_without_panic_or_nan() {
        // n=1: repulsion/attraction loops never run; the lone node must still get
        // a finite, in-bounds position.
        let nodes = vec![node("solo")];
        let (placed, placed_edges) = layout::run(&nodes, &[]);
        assert_eq!(placed.len(), 1);
        assert!(placed_edges.is_empty());
        for p in &placed {
            assert!(
                p.x.is_finite() && p.y.is_finite(),
                "single node has finite coords"
            );
            assert!(
                (20.0..=CANVAS_W - 20.0).contains(&p.x) && (20.0..=CANVAS_H - 20.0).contains(&p.y)
            );
        }
    }

    #[test]
    fn empty_graph_yields_empty_layout() {
        let (placed, placed_edges) = layout::run(&[], &[]);
        assert!(
            placed.is_empty() && placed_edges.is_empty(),
            "n=0 produces no geometry"
        );
    }

    #[test]
    fn self_loops_are_dropped() {
        // a->a would make attraction divide by ~0; the filter must remove it.
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("a", "a"), edge("a", "b")];
        let (placed, placed_edges) = layout::run(&nodes, &edges);
        assert_eq!(placed_edges.len(), 1, "self-loop dropped, real edge kept");
        for p in &placed {
            assert!(p.x.is_finite() && p.y.is_finite(), "no NaN from self-loop");
        }
    }
}
