//! Backend-at-scale: resilience and reliability recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SCALE_RESILIENCE: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::Idempotency,
        algo: "Idempotency-Key + persisted result table (UNIQUE INDEX) + TTL",
        crate_name: "sqlx (manual)",
        when: "External webhook / payment retry; client may resend same op; must not double-charge.",
        avoid_when: "Read-only op; naturally idempotent (PUT/DELETE on resource).",
        complexity: "O(1) lookup + INSERT...ON CONFLICT DO NOTHING",
        edge_cases: "Concurrent retries reach handler before INSERT lands (use UNIQUE + return cached result); TTL too short causes silent duplicate; race between status and result write.",
        source: "https://stripe.com/docs/api/idempotent_requests",
    },
    AlgoRecommendation {
        class: WorkloadClass::Deduplication,
        algo: "Bloom filter + persisted set (large) / HashSet (small)",
        crate_name: "bloomfilter | xorf | std",
        when: "Stream of events where duplicates are possible; downstream cost of duplicate is high.",
        avoid_when: "Source guarantees uniqueness (PRIMARY KEY); FP rate matters more than throughput.",
        complexity: "O(1) per event with k hashes",
        edge_cases: "Bloom FP must be sized for stream size; clear-on-rotation loses dedup at boundary; hash collision under adversarial input.",
        source: "https://docs.rs/bloomfilter/latest/bloomfilter/",
    },
    AlgoRecommendation {
        class: WorkloadClass::CircuitBreaker,
        algo: "Closed/Open/Half-open state machine",
        crate_name: "failsafe | tower::circuit-breaker",
        when: "Calls to flaky downstream that benefits from cooldown rather than retry storm.",
        avoid_when: "Local CPU work (no remote failure); transient hiccup that retry-with-backoff handles fine.",
        complexity: "O(1) per call",
        edge_cases: "Half-open probe under load triggers thundering herd; threshold tuning for low-traffic services; reset window vs failure window.",
        source: "https://docs.rs/failsafe/latest/failsafe/",
    },
    AlgoRecommendation {
        class: WorkloadClass::RetryWithBackoff,
        algo: "Exponential backoff + jitter + bounded attempts",
        crate_name: "backon | tokio-retry",
        when: "Transient failure (network, 5xx, lock conflict).",
        avoid_when: "4xx (client error; retry won't help); idempotency unsure (could double-process).",
        complexity: "O(attempts); total time = sum(min(base*2^i + jitter, max))",
        edge_cases: "Unbounded retry = DoS amplifier; missing jitter creates thundering herd; deadline-aware retry must check budget.",
        source: "https://docs.rs/backon/latest/backon/",
    },
];
