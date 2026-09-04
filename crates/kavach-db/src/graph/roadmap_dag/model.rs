use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by kavach-engine scheduler tests + fetch(); non_exhaustive => E0639"
)]
pub struct DagNode {
    pub id: String,
    pub entry_key: String,
    pub title: String,
    pub entry_status: String,
    pub category: String,
}

#[derive(Debug, Clone, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by kavach-engine scheduler tests + fetch(); non_exhaustive => E0639"
)]
pub struct DagEdge {
    pub source: String,
    pub target: String,
    pub rel: String,
}

#[derive(Debug, Clone, Default, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by kavach-engine scheduler tests + fetch(); non_exhaustive => E0639"
)]
pub struct RoadmapDag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
}

/// Topological-sort outcome for the dependency DAG.
///
/// A cycle is a deadlock: no node in the cycle can ever become ready, so the
/// scheduler must reject the dispatch and name the offending keys rather than
/// spin forever.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched cross-crate by kavach-engine DagScheduler; non_exhaustive => E0004"
)]
pub enum TopoOrder {
    /// Dependency-respecting order: prerequisites precede dependents.
    Ordered(Vec<String>),
    /// The dependency graph has a cycle; these node ids participate in it.
    Cycle(Vec<String>),
}
