//! Sorting and graph algorithm recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SORTING_GRAPHS: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::SortStable,
        algo: "Vec::sort (Driftsort, Rust 1.81+)",
        crate_name: "std::vec",
        when: "Need stable ordering by composite key; preserve input order on ties.",
        avoid_when: "Order on ties irrelevant (sort_unstable is faster).",
        complexity: "O(n log n) worst, O(n) best for nearly-sorted",
        edge_cases: "Driftsort succeeded Timsort in Rust 1.81 — no migration needed.",
        source: "https://blog.rust-lang.org/2024/08/08/Rust-1.81.0/",
    },
    AlgoRecommendation {
        class: WorkloadClass::SortUnstable,
        algo: "Vec::sort_unstable (Ipnsort, Rust 1.81+)",
        crate_name: "std::vec",
        when: "Default for any sort where stability is not required.",
        avoid_when: "Stable order needed (.sort()).",
        complexity: "O(n log n)",
        edge_cases: "Ipnsort uses introsort + branchless partition; faster than pdqsort.",
        source: "https://blog.rust-lang.org/2024/08/08/Rust-1.81.0/",
    },
    AlgoRecommendation {
        class: WorkloadClass::SortPartial,
        algo: "select_nth_unstable",
        crate_name: "std::slice",
        when: "Only need k-th element or top-k; full sort is wasteful.",
        avoid_when: "Need full sort downstream.",
        complexity: "O(n) avg / O(n log n) worst",
        edge_cases: "k must be in 0..n; partitions in place; left/right slices not sorted.",
        source: "https://doc.rust-lang.org/std/slice/fn.select_nth_unstable.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::GraphTraversal,
        algo: "petgraph (BFS/DFS)",
        crate_name: "petgraph",
        when: "Shortest unweighted path, connectivity, topological sort.",
        avoid_when: "Need specialized weighted SP (use Dijkstra/A*).",
        complexity: "O(V+E)",
        edge_cases: "Disconnected graph; cycles for topo-sort fail; visit-order assumption.",
        source: "https://docs.rs/petgraph/latest/petgraph/",
    },
];
