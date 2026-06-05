//! Cache and probabilistic filter recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const CACHES_FILTERS: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::Lru,
        algo: "lru / moka (production)",
        crate_name: "lru | moka",
        when: "Bounded cache with recency eviction; moka adds time-based + concurrent.",
        avoid_when: "Need LFU or admission control (use TinyLFU/W-TinyLFU via moka).",
        complexity: "O(1) get/put",
        edge_cases: "Capacity 0 or 1 edge cases; concurrent get-while-evict.",
        source: "https://docs.rs/moka/latest/moka/",
    },
    AlgoRecommendation {
        class: WorkloadClass::Lfu,
        algo: "TinyLFU / W-TinyLFU (via moka)",
        crate_name: "moka",
        when: "Skewed access pattern; one-hit-wonder pollution of LRU is hurting hit rate.",
        avoid_when: "Uniform access (LRU/FIFO is fine).",
        complexity: "O(1) per op; sketch overhead ~bytes/key",
        edge_cases: "Frequency sketch saturation; admission filter ttl.",
        source: "https://arxiv.org/abs/1512.00727",
    },
    AlgoRecommendation {
        class: WorkloadClass::ApproxMembership,
        algo: "Binary Fuse Filter (xorf)",
        crate_name: "xorf",
        when: "Memory-bound 'probably-present' over large static set; allow false positives.",
        avoid_when: "Set mutates frequently (filter is immutable); FP rate matters strongly.",
        complexity: "~9 bits/key, 3 lookups",
        edge_cases: "Construction failure on tiny sets (<~50); 0 false negatives — false positives only.",
        source: "https://docs.rs/xorf/latest/xorf/",
    },
    AlgoRecommendation {
        class: WorkloadClass::ApproxCardinality,
        algo: "HyperLogLog++ / UltraLogLog",
        crate_name: "hyperloglogplus | ultraloglog",
        when: "Need approximate count of unique items at low memory (~1.5KB for 2% error).",
        avoid_when: "Need exact count; cardinality < ~1000 (HashSet is fine).",
        complexity: "O(1) per insert; constant memory",
        edge_cases: "UltraLogLog is 30% better at same memory; merge across shards is associative.",
        source: "https://en.wikipedia.org/wiki/HyperLogLog",
    },
];
