//! Scale concern classification and matching.

use super::workload::WorkloadClass;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ScaleConcern {
    Throughput,
    Consistency,
    Availability,
    RateAndQuota,
    OrderingAndCausality,
}

impl ScaleConcern {
    pub(super) const fn matches(self, c: WorkloadClass) -> bool {
        use WorkloadClass::{
            Backpressure, BloomCounting, ChangeDataCapture, CircuitBreaker, Concurrent,
            ConsistentHashing, CountMinSketch, Crdt, Deduplication, EventOrdering, FanIn, FanOut,
            Idempotency, LeaderElection, RateLimiterDistributed, RateLimiterPerKey,
            ReplicationGossip, RetryWithBackoff, Saga, ShardingHotKey, StreamWindow,
        };
        match self {
            Self::Throughput => matches!(
                c,
                Backpressure
                    | FanOut
                    | FanIn
                    | StreamWindow
                    | CountMinSketch
                    | BloomCounting
                    | Concurrent
                    | ShardingHotKey
                    | ConsistentHashing
            ),
            Self::Consistency => matches!(
                c,
                Crdt | ReplicationGossip
                    | LeaderElection
                    | EventOrdering
                    | ChangeDataCapture
                    | Saga
            ),
            Self::Availability => matches!(
                c,
                CircuitBreaker | RetryWithBackoff | ReplicationGossip | Crdt
            ),
            Self::RateAndQuota => matches!(
                c,
                RateLimiterPerKey | RateLimiterDistributed | Backpressure | CircuitBreaker
            ),
            Self::OrderingAndCausality => matches!(
                c,
                EventOrdering
                    | Crdt
                    | LeaderElection
                    | ChangeDataCapture
                    | Idempotency
                    | Deduplication
                    | Saga
            ),
        }
    }
}
