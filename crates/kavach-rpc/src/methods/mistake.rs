// RPC methods for the mistake/anti_pattern tier: hit-count (read) and record
// (append event + cluster to anti_pattern by content key). `record` runs INSIDE
// the server process — the single RocksDB writer — so the append+cluster
// sequence happens under the exclusive lock instead of an ephemeral hook child
// fighting it for the lock (the single-writer-invariant violation that left the
// mistake ledger silently empty: the hook's `open_default()` would SIGTERM the
// daemon and race, landing nothing). SOURCE: rca.mistake-ledger-dark-via-direct-open.
//
// Mistakes are tracked structurally in the DAG (no embeddings): a mistake_event
// is text-clustered to its anti_pattern by a deterministic content key, recurrence
// is counted via inbound instance_of edges, and RLAIF grades the node. The former
// ONNX embedder + cosine retrieval were removed — decision/onnx-removal-dag-rlaif-only.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::graph::mistakes::{append_mistake_event, cluster_event_to_pattern};
use kavach_surreal::graph_query_anti_pattern_hit_count;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HitCountParams {
    pub name: String,
}

impl HitCountParams {
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HitCountResult {
    pub name: String,
    pub hit_count: i64,
}

/// Queries the hit count for an anti-pattern by name.
///
/// # Errors
///
/// Returns an RPC error if the database query fails.
pub async fn hit_count(
    state: &AppState,
    p: HitCountParams,
) -> Result<HitCountResult, ErrorObjectOwned> {
    let n = graph_query_anti_pattern_hit_count(&state.db, &p.name)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(HitCountResult {
        name: p.name,
        hit_count: n,
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecordParams {
    pub gate: String,
    pub banned_sample: String,
    pub correct_action: String,
    pub session_id: String,
    pub project: Option<String>,
}

impl RecordParams {
    /// Construct the params for a `mistake.record` call. A constructor (rather
    /// than a struct literal) is required because the struct is
    /// `#[non_exhaustive]` — cross-crate literal construction is forbidden.
    #[must_use]
    pub const fn new(
        gate: String,
        banned_sample: String,
        correct_action: String,
        session_id: String,
        project: Option<String>,
    ) -> Self {
        Self {
            gate,
            banned_sample,
            correct_action,
            session_id,
            project,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecordResult {
    /// `"<event_id> -> <pattern_id>"` — the ids written, for caller logging.
    pub ids: String,
}

/// Record one mistake observation into the knowledge graph.
///
/// Appends an append-only `mistake_event`, then clusters it under its
/// `anti_pattern` by content key (`anti.<gate>.<blake3(correct_action)[..8]>` —
/// no vectors). Runs entirely inside the server process, which already holds the
/// `RocksDB` exclusive lock, so there is no second-opener race — the exact fault
/// that made the hook-invoked direct-open path land nothing.
///
/// # Errors
///
/// Returns an RPC error if the event append or pattern clustering fails.
pub async fn record(state: &AppState, p: RecordParams) -> Result<RecordResult, ErrorObjectOwned> {
    let event_id = append_mistake_event(
        &state.db,
        &p.gate,
        &p.correct_action,
        &p.banned_sample,
        &p.session_id,
        p.project.as_deref(),
    )
    .await
    .map_err(surreal_to_rpc)?;
    let pattern_id = cluster_event_to_pattern(&state.db, &event_id, &p.gate, &p.correct_action)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(RecordResult {
        ids: format!("{event_id:?} -> {pattern_id:?}"),
    })
}
