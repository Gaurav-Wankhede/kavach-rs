//! Backend-at-scale: consistency and replication recommendations.

use super::workload::{AlgoRecommendation, WorkloadClass};

pub(super) const SCALE_CONSISTENCY: &[AlgoRecommendation] = &[
    AlgoRecommendation {
        class: WorkloadClass::Crdt,
        algo: "ORSWOT / MVReg / G-Counter via rust-crdt or datacake-crdt + HLC",
        crate_name: "rust-crdt | datacake-crdt",
        when: "Multi-master eventual consistency; must merge without conflict; offline edits need to converge.",
        avoid_when: "Single-leader is acceptable (Raft is simpler); strict serializability required.",
        complexity: "Merge is associative+commutative+idempotent; ORSWOT O(elements + tombstones)",
        edge_cases: "Tombstone garbage collection without observed-remove invariants; HLC monotonic guarantee under clock skew; ORSWOT vs OR-Set semantics.",
        source: "https://github.com/rust-crdt/rust-crdt",
    },
    AlgoRecommendation {
        class: WorkloadClass::ReplicationGossip,
        algo: "Anti-entropy with Merkle tree / SWIM",
        crate_name: "foca | chitchat",
        when: "N replicas need eventually-converged state without central coordinator; failure detection.",
        avoid_when: "Strong consistency required (use Raft).",
        complexity: "O(log n) rounds to converge; O(1) failure detection per round",
        edge_cases: "Indirect ping window vs flapping; suspect-state TTL; gossip amplification under partition.",
        source: "https://docs.rs/foca/latest/foca/",
    },
    AlgoRecommendation {
        class: WorkloadClass::LeaderElection,
        algo: "Raft (openraft) / single-writer lease via etcd or Postgres advisory lock",
        crate_name: "openraft | etcd-client | sqlx pg_advisory_lock",
        when: "Need single-writer for serialization; durable log of decisions; cluster size <=7.",
        avoid_when: "Stateless workload (use random shard holder); cluster size >7 (consensus overhead dominates).",
        complexity: "Raft O(log n) commit; lease renew O(1) per heartbeat",
        edge_cases: "Split-brain under network partition (use lease + fence token); leader-step-down latency; clock-bounded leases need monotonic clock.",
        source: "https://docs.rs/openraft/latest/openraft/",
    },
    AlgoRecommendation {
        class: WorkloadClass::EventOrdering,
        algo: "Hybrid Logical Clock (HLC)",
        crate_name: "datacake-crdt (HLCTimestamp)",
        when: "Events from N nodes must be totally ordered for replay/CRDT merge.",
        avoid_when: "Single-node (atomic counter is enough); causal ordering only (vector clock).",
        complexity: "O(1) per timestamp",
        edge_cases: "Wall-clock skew bounded; logical-component overflow at >65k events/ms; causality lost across nodes that never gossip.",
        source: "https://cse.buffalo.edu/tech-reports/2014-04.pdf",
    },
];
