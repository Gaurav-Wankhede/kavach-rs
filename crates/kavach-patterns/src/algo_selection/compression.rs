//! Compression and hashing utility recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const COMPRESSION: &[AlgoRecommendation] = &[AlgoRecommendation {
    class: WorkloadClass::Compression,
    algo: "Zstd (default) / LZ4 (latency)",
    crate_name: "zstd | lz4_flex",
    when: "Bytes-on-the-wire reduction; Zstd for ratio, LZ4 for latency.",
    avoid_when: "Already compressed (jpeg/zip); tiny payloads (<200B overhead dominates).",
    complexity: "O(n) with SIMD; Zstd levels trade speed for ratio",
    edge_cases: "Dictionary training pays off above 1k records of similar shape; Zstd level 3 is sweet spot.",
    source: "https://github.com/facebook/zstd",
}];
