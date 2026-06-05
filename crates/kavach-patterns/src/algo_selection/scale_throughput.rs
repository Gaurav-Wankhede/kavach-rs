//! Backend-at-scale: throughput and streaming recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SCALE_THROUGHPUT: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::Backpressure,
        algo: "Bounded mpsc channel (tokio::sync::mpsc::channel(N))",
        crate_name: "tokio | async-channel",
        when: "Producer can outpace consumer; want graceful slow-down rather than OOM.",
        avoid_when: "Unbounded is acceptable (rare); load-shed by drop is preferred (use tower::load_shed).",
        complexity: "O(1) send/recv; capacity ~1024 reduces contention 35% in 2026 benchmarks",
        edge_cases: "Send-on-full blocks the producer; .try_send + drop for fire-and-forget; consumer panic without close = sender blocks forever.",
        source: "https://oneuptime.com/blog/post/2026-01-25-high-throughput-data-ingestion-pipeline-rust/view",
    },
    AlgoRecommendation {
        class: WorkloadClass::FanOut,
        algo: "tokio::sync::broadcast or work-queue + N consumers",
        crate_name: "tokio",
        when: "One event must reach many subscribers; pub/sub at process scope.",
        avoid_when: "Cross-process pub/sub (use NATS/Redis Streams).",
        complexity: "O(subscribers) per send",
        edge_cases: "Slow subscriber lags; broadcast drops oldest if recv is slow; backpressure via channel capacity.",
        source: "https://docs.rs/tokio/latest/tokio/sync/broadcast/",
    },
    AlgoRecommendation {
        class: WorkloadClass::FanIn,
        algo: "futures::stream::select_all / tokio::JoinSet",
        crate_name: "tokio | futures",
        when: "Aggregate N concurrent sources into one ordered (or unordered) stream.",
        avoid_when: "Strict ordering required across sources (use single-producer queue).",
        complexity: "O(1) per merged item",
        edge_cases: "select_all empty input never resolves; JoinSet panic propagation; cancellation when one source errors.",
        source: "https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::StreamWindow,
        algo: "Tumbling/Sliding window via t-digest or histogram",
        crate_name: "tdigest | hdrhistogram",
        when: "Aggregate metrics over time window (p99 latency, error rate per minute).",
        avoid_when: "Need per-event detail (use append-only log).",
        complexity: "O(log n) per insert; merge associative",
        edge_cases: "Window boundary discontinuity; out-of-order events past watermark must be late-rejected or processed; TTL on bucket eviction.",
        source: "https://docs.rs/hdrhistogram/latest/hdrhistogram/",
    },
];
