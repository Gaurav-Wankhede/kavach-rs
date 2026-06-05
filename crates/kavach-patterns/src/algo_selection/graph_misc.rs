//! Graph analytics and miscellaneous algorithms.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const GRAPH_MISC: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::Mst,
        algo: "Kruskal (sparse) / Prim (dense)",
        crate_name: "petgraph::algo::min_spanning_tree",
        when: "Need minimum spanning tree.",
        avoid_when: "Edges have all-equal weight (any spanning tree works).",
        complexity: "Kruskal O(E log E); Prim O(E + V log V) with heap",
        edge_cases: "Disconnected input -> minimum spanning forest; tie-breaking is non-unique.",
        source: "https://docs.rs/petgraph/latest/petgraph/algo/fn.min_spanning_tree.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::SccDfs,
        algo: "Tarjan SCC",
        crate_name: "petgraph::algo::tarjan_scc",
        when: "Strongly connected components in directed graph.",
        avoid_when: "Undirected graph (use connected_components).",
        complexity: "O(V + E)",
        edge_cases: "Self-loops form their own SCC; iteration order matters for determinism.",
        source: "https://docs.rs/petgraph/latest/petgraph/algo/fn.tarjan_scc.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::UnionFind,
        algo: "Union-Find with path compression",
        crate_name: "petgraph::unionfind",
        when: "Connectivity queries under merges; Kruskal MST building block.",
        avoid_when: "Need to undo merges (use offline link-cut trees).",
        complexity: "O(alpha(n)) ~= O(1) amortized",
        edge_cases: "Path compression invalidates parent pointers; refresh before traversal.",
        source: "https://docs.rs/petgraph/latest/petgraph/unionfind/",
    },
    AlgoRecommendation {
        class: WorkloadClass::NearestNeighbor,
        algo: "HNSW (instant-distance / hnsw_rs)",
        crate_name: "instant-distance | hnsw_rs",
        when: "Vector similarity over >=10k vectors; sub-linear query needed.",
        avoid_when: "n < ~5000 (brute force with SIMD wins on cache locality).",
        complexity: "~O(log n) query; O(n log n) build",
        edge_cases: "Recall vs latency trade via ef_search; cold cache; index mutation cost.",
        source: "https://docs.rs/instant-distance/",
    },
];
