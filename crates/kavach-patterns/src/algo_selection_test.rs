use super::*;

#[test]
fn lookup_returns_recommendation() {
    let r = recommend(WorkloadClass::PointLookup).unwrap();
    assert_eq!(r.class, WorkloadClass::PointLookup);
    assert!(r.algo.contains("HashMap"));
    assert!(!r.when.is_empty());
    assert!(!r.avoid_when.is_empty());
    assert!(!r.complexity.is_empty());
    assert!(!r.edge_cases.is_empty());
    assert!(r.source.starts_with("https://"));
}

#[test]
fn every_class_has_a_row() {
    let classes = [
        WorkloadClass::PointLookup,
        WorkloadClass::OrderedRangeQuery,
        WorkloadClass::TopK,
        WorkloadClass::SetMembership,
        WorkloadClass::ApproxMembership,
        WorkloadClass::ApproxCardinality,
        WorkloadClass::StreamingQuantile,
        WorkloadClass::PriorityQueue,
        WorkloadClass::Lru,
        WorkloadClass::Lfu,
        WorkloadClass::StringSearchSingle,
        WorkloadClass::StringSearchMulti,
        WorkloadClass::PrefixSearch,
        WorkloadClass::NearestNeighbor,
        WorkloadClass::SequentialScan,
        WorkloadClass::FifoQueue,
        WorkloadClass::LifoStack,
        WorkloadClass::DequeBoth,
        WorkloadClass::SortStable,
        WorkloadClass::SortUnstable,
        WorkloadClass::SortPartial,
        WorkloadClass::GraphTraversal,
        WorkloadClass::ShortestPathSparse,
        WorkloadClass::ShortestPathDense,
        WorkloadClass::ShortestPathRoadNet,
        WorkloadClass::ShortestPathAStar,
        WorkloadClass::AllPairsShortestPath,
        WorkloadClass::Mst,
        WorkloadClass::SccDfs,
        WorkloadClass::UnionFind,
        WorkloadClass::Concurrent,
        WorkloadClass::PersistentImmutable,
        WorkloadClass::HashIntKey,
        WorkloadClass::HashCryptoSafe,
        WorkloadClass::Compression,
        WorkloadClass::StringHash,
        WorkloadClass::Pagination,
        WorkloadClass::DistinctCount,
        WorkloadClass::RangeMin,
        WorkloadClass::PointUpdateRangeQuery,
        WorkloadClass::Dedup,
        WorkloadClass::Recursion,
        WorkloadClass::RateLimiterPerKey,
        WorkloadClass::RateLimiterDistributed,
        WorkloadClass::ConsistentHashing,
        WorkloadClass::ShardingHotKey,
        WorkloadClass::Crdt,
        WorkloadClass::ReplicationGossip,
        WorkloadClass::LeaderElection,
        WorkloadClass::Idempotency,
        WorkloadClass::Deduplication,
        WorkloadClass::Backpressure,
        WorkloadClass::FanOut,
        WorkloadClass::FanIn,
        WorkloadClass::CircuitBreaker,
        WorkloadClass::RetryWithBackoff,
        WorkloadClass::EventOrdering,
        WorkloadClass::StreamWindow,
        WorkloadClass::CountMinSketch,
        WorkloadClass::BloomCounting,
        WorkloadClass::ChangeDataCapture,
        WorkloadClass::Saga,
        WorkloadClass::GeoLookup,
    ];
    for c in classes {
        assert!(recommend(c).is_some(), "missing rule for {c:?}");
    }
}

#[test]
fn all_returns_full_table() {
    assert!(all().len() >= 60);
}

#[test]
fn scale_concern_filters() {
    let throughput = for_scale_concern(ScaleConcern::Throughput);
    assert!(
        throughput
            .iter()
            .any(|r| r.class == WorkloadClass::Backpressure)
    );
    assert!(
        throughput
            .iter()
            .any(|r| r.class == WorkloadClass::ConsistentHashing)
    );

    let consistency = for_scale_concern(ScaleConcern::Consistency);
    assert!(consistency.iter().any(|r| r.class == WorkloadClass::Crdt));
    assert!(
        consistency
            .iter()
            .any(|r| r.class == WorkloadClass::LeaderElection)
    );

    let rate = for_scale_concern(ScaleConcern::RateAndQuota);
    assert!(
        rate.iter()
            .any(|r| r.class == WorkloadClass::RateLimiterPerKey)
    );
    assert!(
        rate.iter()
            .any(|r| r.class == WorkloadClass::RateLimiterDistributed)
    );

    let ordering = for_scale_concern(ScaleConcern::OrderingAndCausality);
    assert!(
        ordering
            .iter()
            .any(|r| r.class == WorkloadClass::EventOrdering)
    );
    assert!(
        ordering
            .iter()
            .any(|r| r.class == WorkloadClass::Idempotency)
    );
}

#[test]
fn every_recommendation_has_https_source() {
    for r in all() {
        assert!(
            r.source.starts_with("https://"),
            "non-https source for {:?}: {}",
            r.class,
            r.source
        );
    }
}

#[test]
fn complexity_strings_present() {
    for r in all() {
        assert!(
            !r.complexity.is_empty(),
            "empty complexity for {:?}",
            r.class
        );
        assert!(
            !r.edge_cases.is_empty(),
            "empty edge_cases for {:?}",
            r.class
        );
    }
}
