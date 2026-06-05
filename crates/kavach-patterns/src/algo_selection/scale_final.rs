//! Backend-at-scale: sketches, data capture, sagas, and location patterns.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SCALE_FINAL: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::CountMinSketch,
        algo: "Count-Min Sketch",
        crate_name: "probabilistic-collections",
        when: "Approximate frequency over heavy stream; memory bounded; care about top-k or thresholds.",
        avoid_when: "Need exact counts; cardinality < ~10k (HashMap is fine).",
        complexity: "O(d) per update/query; d=hash count, w=width",
        edge_cases: "Overestimate only (never under); width too narrow inflates error; conservative-update variant reduces error 2-4x.",
        source: "https://en.wikipedia.org/wiki/Count%E2%80%93min_sketch",
    },
    AlgoRecommendation {
        class: WorkloadClass::BloomCounting,
        algo: "Counting Bloom Filter",
        crate_name: "bloomfilter (counting feature)",
        when: "Membership with deletion support; small false positives acceptable.",
        avoid_when: "Standard Bloom suffices (no deletion); space matters more than deletion (use Cuckoo).",
        complexity: "O(k) per op",
        edge_cases: "Counter overflow at threshold; counter width tradeoff vs memory.",
        source: "https://en.wikipedia.org/wiki/Counting_Bloom_filter",
    },
    AlgoRecommendation {
        class: WorkloadClass::ChangeDataCapture,
        algo: "Logical replication slot + outbox table",
        crate_name: "n/a (Postgres LOGICAL REPLICATION)",
        when: "Reliable propagation of DB changes to downstream (search index, cache, queue).",
        avoid_when: "Read-only replica is enough; downstream tolerates polling.",
        complexity: "O(WAL volume) on producer; O(events) on consumer",
        edge_cases: "Replication slot inactivity grows WAL forever; outbox transactional w/ business write; consumer must be idempotent on replay.",
        source: "https://www.postgresql.org/docs/current/logical-replication.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::Saga,
        algo: "Choreographed (event-driven) or Orchestrated (state machine) saga",
        crate_name: "n/a (pattern) | rust-fsm | statig",
        when: "Multi-service transaction where 2PC is impossible; need compensating actions.",
        avoid_when: "Single-DB transaction is feasible (use it).",
        complexity: "O(steps) forward + O(compensations) on failure",
        edge_cases: "Compensation must be idempotent; partial failure in compensation; saga timeout vs participant retry.",
        source: "https://microservices.io/patterns/data/saga.html",
    },
    AlgoRecommendation {
        class: WorkloadClass::GeoLookup,
        algo: "S2 / H3 cell index",
        crate_name: "s2 | h3o",
        when: "Geo proximity, point-in-polygon at scale, geofencing.",
        avoid_when: "Tiny static set (R-tree is enough); raster grid suffices.",
        complexity: "O(log n) cell lookup; O(1) cell containment",
        edge_cases: "Cell boundary discontinuity; resolution selection trades index size vs precision; antimeridian wrap.",
        source: "https://h3geo.org/",
    },
];
