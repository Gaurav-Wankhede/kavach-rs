// RPC method namespace for Brain-OS auto-recall: `brain.think` exposes the
// vectorless hybrid retrieval (BM25 corpus + concept FTS, RRF-fused) so gates
// can consult memory through the single-writer server, never opening the DB
// directly. Mirrors methods/concept.rs (search verb, 1 file).
// SOURCE: roadmap.unit.harness.brain-os.g2-think-mode + g3 auto-recall.
use crate::error::db_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_db::{BrainHit, search_corpus_project};
use serde::{Deserialize, Serialize};

/// Default hit count when the caller omits `limit`. Small: the recall block is
/// injected into every prompt, so it must stay token-cheap.
const DEFAULT_LIMIT: usize = 5;

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct ThinkParams {
    /// Free-text query (typically the user prompt) to retrieve against.
    pub query: String,
    /// Max fused hits to return. Defaults to [`DEFAULT_LIMIT`].
    pub limit: Option<usize>,
    /// Optional project slug to prioritize project-specific memory hits.
    pub project: Option<String>,
}

/// Hybrid keyword+graph retrieval over the memory corpus, RRF-ranked.
///
/// # Errors
/// Propagates `database_to_rpc` when any underlying corpus query fails. An empty
/// query yields an empty list (not an error) — the caller injects nothing.
pub async fn think(state: &AppState, p: ThinkParams) -> Result<Vec<BrainHit>, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(DEFAULT_LIMIT);
    search_corpus_project(&state.db, p.project.as_deref(), &p.query, limit)
        .await
        .map_err(db_to_rpc)
}
