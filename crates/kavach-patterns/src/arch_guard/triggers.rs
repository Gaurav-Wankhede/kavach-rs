//! Architecture trigger keywords and matcher.

use aho_corasick::AhoCorasick;
use std::sync::LazyLock;

use super::types::ArchScope;

/// Scale/distribution keywords.
const SCALE_TRIGGERS: &[&str] = &[
    "horizontal_scale",
    "load_balancer",
    "replica",
    "shard",
    "partition",
    "consistent_hash",
    "raft",
    "paxos",
    "gossip_protocol",
    "leader_election",
];

/// Caching pattern keywords.
const CACHE_TRIGGERS: &[&str] = &[
    "cache_aside",
    "write_through",
    "write_behind",
    "cache_invalidation",
    "ttl_cache",
    "distributed_cache",
    "moka",
    "cached",
];

/// Messaging/communication keywords.
const MESSAGING_TRIGGERS: &[&str] = &[
    "message_queue",
    "event_bus",
    "pub_sub",
    "kafka",
    "rabbitmq",
    "nats",
    "redis_pubsub",
    "async_channel",
    "actor_model",
    "backpressure",
];

/// Data layer keywords.
const DATA_TRIGGERS: &[&str] = &[
    "read_replica",
    "write_primary",
    "cqrs",
    "event_sourcing",
    "saga",
    "two_phase_commit",
    "eventual_consistency",
    "strong_consistency",
    "CAP",
];

/// Service pattern keywords.
const SERVICE_TRIGGERS: &[&str] = &[
    "circuit_breaker",
    "bulkhead",
    "retry_policy",
    "rate_limiter",
    "service_mesh",
    "sidecar",
    "api_gateway",
    "grpc_service",
    "rest_api",
];

fn mk_ac(patterns: &[&str]) -> AhoCorasick {
    loop {
        if let Ok(ac) = AhoCorasick::builder().build(patterns) {
            break ac;
        }
        if let Ok(ac) = AhoCorasick::new(["scale"]) {
            break ac;
        }
        if let Ok(ac) = AhoCorasick::new(["s"]) {
            break ac;
        }
        if let Ok(ac) = AhoCorasick::new(["a"]) {
            break ac;
        }
        if let Ok(ac) = AhoCorasick::new(["."]) {
            break ac;
        }
    }
}

/// Pre-built Aho-Corasick automaton for all triggers.
static AC: LazyLock<AhoCorasick> = LazyLock::new(|| {
    let patterns: Vec<&str> = SCALE_TRIGGERS
        .iter()
        .chain(CACHE_TRIGGERS)
        .chain(MESSAGING_TRIGGERS)
        .chain(DATA_TRIGGERS)
        .chain(SERVICE_TRIGGERS)
        .copied()
        .collect();
    mk_ac(&patterns)
});

/// Returns the scope for a pattern index.
const fn scope_for_index(idx: usize) -> ArchScope {
    let scale_end = SCALE_TRIGGERS.len();
    let cache_end = scale_end.saturating_add(CACHE_TRIGGERS.len());
    let msg_end = cache_end.saturating_add(MESSAGING_TRIGGERS.len());
    let data_end = msg_end.saturating_add(DATA_TRIGGERS.len());

    if idx < scale_end {
        ArchScope::Scale
    } else if idx < cache_end {
        ArchScope::Cache
    } else if idx < msg_end {
        ArchScope::Messaging
    } else if idx < data_end {
        ArchScope::Data
    } else {
        ArchScope::Service
    }
}

/// Returns the keyword for a pattern index.
fn keyword_for_index(idx: usize) -> &'static str {
    let all: Vec<&str> = SCALE_TRIGGERS
        .iter()
        .chain(CACHE_TRIGGERS)
        .chain(MESSAGING_TRIGGERS)
        .chain(DATA_TRIGGERS)
        .chain(SERVICE_TRIGGERS)
        .copied()
        .collect();
    all.get(idx).copied().map_or("unknown", |s| s)
}

/// Find all arch trigger matches in content.
pub(super) fn find_matches(content: &str) -> Vec<(usize, &'static str, ArchScope)> {
    let mut results = Vec::new();
    for mat in AC.find_iter(content) {
        let idx = mat.pattern().as_usize();
        results.push((mat.start(), keyword_for_index(idx), scope_for_index(idx)));
    }
    results
}
