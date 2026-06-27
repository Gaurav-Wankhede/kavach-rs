//   {"name":"database-backed recommendations","reason":"adds I/O; lookup is write-time only; static table is sufficient"},
//   {"name":"ML classifier","reason":"overkill; deterministic rubric is sufficient and verifiable"},
//   {"name":"trait-per-workload-class","reason":"premature abstraction; enum + match is clearer"}
// ]
// TIME: O(classes) full scan | O(1) per match w/ HashMap index (future opt) | SPACE: O(table entries)
// YEAR: 2026 | SEARCHED: 2026-05
//
//! Algorithm Selection Rubric — When to Use Which (2026)
//!
//! Companion to `dsa_guard`. Where `dsa_guard` says "don't do X", this module
//! tells engineers "use Y when {`edge_case_signature`}".
//!
//! SOURCES (verified 2026-05):
//! - <https://opendsa-server.cs.vt.edu/ODSA/Books/Everything/html/IntroDSA.html>
//! - <https://www.designgurus.io/blog/choosing-the-right-data-structure>
//! - <https://www.nucamp.co/blog/data-structures-and-algorithms-in-2026>
//! - <https://doc.rust-lang.org/std/collections/index.html>
//! - <https://arxiv.org/abs/2504.17033> (STOC 2025 SSSP successor — research-tier)

mod basic_maps;
mod caches_filters;
mod collections;
mod compression;
mod filters;
mod graph_misc;
mod heaps_search;
mod misc_utilities;
mod range_dedup;
mod scale_consistency;
mod scale_final;
mod scale_rate_limit;
mod scale_resilience;
mod scale_throughput;
mod shortest_paths;
mod sorting_graphs;
mod workload;

// Re-export public API types
pub use filters::ScaleConcern;
pub use workload::{AlgoRecommendation, WorkloadClass};

/// Assemble the complete recommendation table from all leaf modules.
const TABLE: &[&[AlgoRecommendation]] = &[
    basic_maps::MAPS,
    caches_filters::CACHES_FILTERS,
    collections::COLLECTIONS,
    compression::COMPRESSION,
    graph_misc::GRAPH_MISC,
    heaps_search::HEAPS_SEARCH,
    misc_utilities::MISC_UTILITIES,
    range_dedup::RANGE_DEDUP,
    scale_consistency::SCALE_CONSISTENCY,
    scale_final::SCALE_FINAL,
    scale_rate_limit::SCALE_RATE_LIMIT,
    scale_resilience::SCALE_RESILIENCE,
    scale_throughput::SCALE_THROUGHPUT,
    shortest_paths::SHORTEST_PATHS,
    sorting_graphs::SORTING_GRAPHS,
];

/// Filter recommendations by scale concern.
#[must_use]
pub fn for_scale_concern(concern: ScaleConcern) -> Vec<&'static AlgoRecommendation> {
    TABLE
        .iter()
        .flat_map(|slice| slice.iter())
        .filter(|r| concern.matches(r.class))
        .collect()
}

/// Look up recommendation by workload class.
#[must_use]
pub fn recommend(class: WorkloadClass) -> Option<&'static AlgoRecommendation> {
    TABLE
        .iter()
        .flat_map(|slice| slice.iter())
        .find(|r| r.class == class)
}

/// Iterate the full rubric.
#[must_use]
pub fn all() -> Vec<&'static AlgoRecommendation> {
    TABLE.iter().flat_map(|slice| slice.iter()).collect()
}

#[cfg(test)]
#[path = "algo_selection_test.rs"]
#[cfg(test)]
mod tests;