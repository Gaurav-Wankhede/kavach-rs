//! Miscellaneous utility algorithm recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const MISC_UTILITIES: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::StringHash,
        algo: "xxHash3 / GxHash",
        crate_name: "xxhash-rust | gxhash",
        when: "Non-crypto hashing of bytes/strings; checksums; bloom-style sketches.",
        avoid_when: "Untrusted input that becomes hash key (use SipHash); content-addressing (use BLAKE3).",
        complexity: "Multi-GB/s with SIMD",
        edge_cases: "Verify seed compatibility across processes/languages.",
        source: "https://docs.rs/xxhash-rust/",
    },
    AlgoRecommendation {
        class: WorkloadClass::Pagination,
        algo: "Keyset pagination (WHERE id > last_id)",
        crate_name: "sqlx (manual)",
        when: "DB pagination over indexed monotonic key.",
        avoid_when: "Random access to page N (rare; admin only; accept the cliff).",
        complexity: "O(log n + page_size) per page vs OFFSET O(n)",
        edge_cases: "Composite sort keys need composite cursor; tie-breaking on equal sort key needs id tiebreaker.",
        source: "https://use-the-index-luke.com/no-offset",
    },
];
