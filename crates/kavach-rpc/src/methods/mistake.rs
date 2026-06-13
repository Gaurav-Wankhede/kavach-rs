// RPC methods for the mistake/anti_pattern tier: hit-count (read) and record
// (embed + append event + cluster to anti_pattern). `record` runs INSIDE the
// daemon — the single RocksDB writer — so the embed+append+cluster sequence
// happens under the daemon's exclusive lock instead of an ephemeral hook child
// fighting it for the lock (the single-writer-invariant violation that left the
// mistake ledger silently empty: the hook's `open_default()` would SIGTERM the
// daemon and race, landing nothing). SOURCE: rca.mistake-ledger-dark-via-direct-open.
use std::sync::OnceLock;

use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::{ErrorObjectOwned, error::INTERNAL_ERROR_CODE};
use kavach_surreal::graph::mistakes::{append_mistake_event, cluster_event_to_pattern};
use kavach_surreal::{Embedder, graph_nearest_anti_patterns, graph_query_anti_pattern_hit_count};
use serde::{Deserialize, Serialize};

/// Process-cached BGE-small embedder. The daemon is long-lived, so the ONNX
/// model loads exactly once on the first `mistake.record` and is reused for
/// every subsequent call — embedding a mistake on a hot daemon is then just a
/// forward pass, not a model reload.
static EMBEDDER: OnceLock<Embedder> = OnceLock::new();

fn embedder() -> Result<&'static Embedder, ErrorObjectOwned> {
    if let Some(e) = EMBEDDER.get() {
        return Ok(e);
    }
    let built = Embedder::try_new().map_err(|e| {
        ErrorObjectOwned::owned(
            INTERNAL_ERROR_CODE,
            format!("embedder init: {e}"),
            None::<()>,
        )
    })?;
    Ok(EMBEDDER.get_or_init(|| built))
}

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
/// Embeds the gate/banned/correct triple, appends an append-only
/// `mistake_event`, then clusters it under the nearest `anti_pattern`
/// (cosine kNN, threshold 0.85). Runs entirely inside the daemon, which
/// already holds the `RocksDB` exclusive lock, so there is no second-opener
/// race — the exact fault that made the hook-invoked direct-open path land
/// nothing.
///
/// # Errors
///
/// Returns an RPC error if embedder init, embedding, event append, or pattern
/// clustering fails.
pub async fn record(state: &AppState, p: RecordParams) -> Result<RecordResult, ErrorObjectOwned> {
    let text = format!(
        "gate={} | banned: {} | correct: {}",
        p.gate, p.banned_sample, p.correct_action
    );
    let embedding = embedder()?.embed_one(&text).await.map_err(|e| {
        ErrorObjectOwned::owned(INTERNAL_ERROR_CODE, format!("embed: {e}"), None::<()>)
    })?;
    let event_id = append_mistake_event(
        &state.db,
        &p.gate,
        &p.correct_action,
        &p.banned_sample,
        &p.session_id,
        p.project.as_deref(),
        embedding.clone(),
    )
    .await
    .map_err(surreal_to_rpc)?;
    let pattern_id =
        cluster_event_to_pattern(&state.db, &event_id, &embedding, &p.gate, &p.correct_action)
            .await
            .map_err(surreal_to_rpc)?;
    Ok(RecordResult {
        ids: format!("{event_id:?} -> {pattern_id:?}"),
    })
}

/// Default number of relevant mistakes surfaced at the point of action.
const NEAREST_DEFAULT_K: usize = 3;
/// Default cosine floor: below this a past mistake is not relevant to the edit.
const NEAREST_DEFAULT_FLOOR: f32 = 0.6;

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NearestParams {
    /// Free text to relevance-match against — e.g. the file content being written.
    pub text: String,
    /// How many hits to return (defaults to `NEAREST_DEFAULT_K`).
    pub k: Option<usize>,
    /// Cosine relevance floor (defaults to `NEAREST_DEFAULT_FLOOR`).
    pub floor: Option<f32>,
}

impl NearestParams {
    /// Construct params for a `mistake.nearest` call (`#[non_exhaustive]` ⇒ no
    /// cross-crate struct literal).
    #[must_use]
    pub const fn new(text: String, k: Option<usize>, floor: Option<f32>) -> Self {
        Self { text, k, floor }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NearestHit {
    pub gate: String,
    pub correct_action: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NearestResult {
    pub hits: Vec<NearestHit>,
}

/// Cosine-retrieve the `anti_patterns` most relevant to `text`.
///
/// The read side of the `[MISTAKE_GUARD]` pre-write frame: embeds the query once
/// (cached BGE model) and runs the kNN over the HNSW-indexed centroid set,
/// keeping only hits at or above the floor. Empty result (no relevant mistake /
/// fresh graph) is a success, never an error — the gate must fail open on benign
/// emptiness.
///
/// # Errors
/// Returns an RPC error only on embedder init/embedding failure or a real DB
/// query failure.
pub async fn nearest(
    state: &AppState,
    p: NearestParams,
) -> Result<NearestResult, ErrorObjectOwned> {
    let embedding = embedder()?.embed_one(&p.text).await.map_err(|e| {
        ErrorObjectOwned::owned(INTERNAL_ERROR_CODE, format!("embed: {e}"), None::<()>)
    })?;
    let k = p.k.unwrap_or(NEAREST_DEFAULT_K);
    let floor = p.floor.unwrap_or(NEAREST_DEFAULT_FLOOR);
    let hits = graph_nearest_anti_patterns(&state.db, &embedding, k, floor)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(NearestResult {
        hits: hits
            .into_iter()
            .map(|h| NearestHit {
                gate: h.gate,
                correct_action: h.correct_action,
                score: h.score,
            })
            .collect(),
    })
}
