//! Shortest path algorithm recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SHORTEST_PATHS: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::ShortestPathSparse,
        algo: "Dijkstra (BinaryHeap-backed)",
        crate_name: "petgraph::algo::dijkstra",
        when: "Sparse graph (E ~ V); non-negative weights; |V| > 1000.",
        avoid_when: "Negative edges (Bellman-Ford); need all-pairs (Floyd-Warshall/Johnson).",
        complexity: "O((V+E) log V) with binary heap",
        edge_cases: "Negative weights silently produce wrong answers; assert non-negative; tie-breaking on equal distances.",
        source: "https://docs.rs/petgraph/latest/petgraph/algo/dijkstra/",
    },
    AlgoRecommendation {
        class: WorkloadClass::ShortestPathRoadNet,
        algo: "Contraction Hierarchies / A*",
        crate_name: "fast_paths (CH) | petgraph::algo::astar",
        when: "Road networks; many queries on a static graph; need <1ms latency.",
        avoid_when: "Graph mutates; admissible heuristic unknown.",
        complexity: "Sub-linear queries after preprocessing",
        edge_cases: "A* requires admissible heuristic; Euclidean for road networks; fail-safe to Dijkstra if heuristic absent.",
        source: "https://docs.rs/fast_paths/",
    },
    AlgoRecommendation {
        class: WorkloadClass::ShortestPathDense,
        algo: "Floyd-Warshall",
        crate_name: "petgraph::algo::floyd_warshall",
        when: "All-pairs needed; dense graph; |V| < ~500.",
        avoid_when: "|V| > 1000 (memory O(V^2)).",
        complexity: "O(V^3)",
        edge_cases: "Detects negative cycles via diagonal less-than zero.",
        source: "https://docs.rs/petgraph/latest/petgraph/algo/floyd_warshall/",
    },
    AlgoRecommendation {
        class: WorkloadClass::ShortestPathAStar,
        algo: "A*",
        crate_name: "petgraph::algo::astar",
        when: "Single-source single-target with admissible heuristic.",
        avoid_when: "No heuristic available (degrades to Dijkstra) or many targets.",
        complexity: "Better than Dijkstra in practice if heuristic is tight",
        edge_cases: "Inadmissible heuristic returns wrong path; verify h(n) <= true cost.",
        source: "https://docs.rs/petgraph/latest/petgraph/algo/fn.astar.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::AllPairsShortestPath,
        algo: "Johnson (sparse) / Floyd-Warshall (dense)",
        crate_name: "petgraph::algo",
        when: "Need full distance matrix.",
        avoid_when: "Single-source only; Dijkstra is enough.",
        complexity: "Johnson O(V^2 log V + VE) | FW O(V^3)",
        edge_cases: "Johnson handles negative edges via Bellman-Ford reweighting.",
        source: "https://docs.rs/petgraph/latest/petgraph/algo/",
    },
];
