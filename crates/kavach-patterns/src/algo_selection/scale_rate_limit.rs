//! Backend-at-scale: rate limiting and hashing recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SCALE_RATE_LIMIT: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::RateLimiterPerKey,
        algo: "GCRA token bucket (governor crate)",
        crate_name: "governor",
        when: "Per-tenant or per-IP rate limit on a single node; lock-free.",
        avoid_when: "Multi-node where the limit must be globally enforced (use distributed variant).",
        complexity: "O(1) per check; no allocation steady-state",
        edge_cases: "Burst at boundary of new period; clock skew on suspended VMs; quota=0 must error not divide-by-zero.",
        source: "https://docs.rs/governor/latest/governor/",
    },
    AlgoRecommendation {
        class: WorkloadClass::RateLimiterDistributed,
        algo: "Sliding-window counter on Redis Sorted Sets / Gubernator",
        crate_name: "redis | gubernator",
        when: "Cluster-wide quota; multiple replicas must agree; SLA latency >5ms tolerable.",
        avoid_when: "Single-node sufficient (use governor); strict global accuracy needed under partition (use leader-leased counter).",
        complexity: "Redis ZADD/ZCOUNT O(log n); RTT-bound",
        edge_cases: "Network partition causes overshoot; Redis eviction drops counters; clock skew between cluster + clients.",
        source: "https://arxiv.org/html/2602.11741",
    },
    AlgoRecommendation {
        class: WorkloadClass::ConsistentHashing,
        algo: "Maglev / Jump / Rendezvous",
        crate_name: "maglev | jump-consistent-hash | rendezvous",
        when: "Sharding requests across N replicas; need stable routing under add/remove.",
        avoid_when: "N is fixed at compile time and never changes (modulo-N is fine); need per-key weights (use weighted rendezvous).",
        complexity: "Maglev O(N) lookup table build, O(1) lookup; Jump O(log n) lookup w/o table",
        edge_cases: "Hot key bypasses sharding; Maglev table size must be prime; node failure must trigger re-shard for AP not CP.",
        source: "https://github.com/topics/consistent-hashing",
    },
    AlgoRecommendation {
        class: WorkloadClass::ShardingHotKey,
        algo: "Power-of-two-choices + sub-key sharding",
        crate_name: "n/a (pattern)",
        when: "One key absorbs >5% of traffic; tail latency dominated by hot shard.",
        avoid_when: "Uniform key distribution (consistent hashing alone is fine).",
        complexity: "O(1) extra hash + 2x routing memory",
        edge_cases: "Hot-key detection lag; aggregation across sub-shards on read; rebalance gives stale reads briefly.",
        source: "https://www.eecs.harvard.edu/~michaelm/postscripts/handbook2001.pdf",
    },
];
