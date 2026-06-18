//! KavachBrain — the unified retrieval/graph contract (Brain-OS G3,
//! roadmap.unit.harness.brain-os.g3-kavachbrain-trait).
//!
//! One coherent surface over the scattered substrate: BM25/FTS retrieval (g1a),
//! RRF fusion (g1b), the concept/bridge/mistake DAG, and the gap/think synthesis
//! that closes the self-improving loop. No vectors — the ONNX embedder was
//! removed (decision/onnx-removal-dag-rlaif-only); retrieval is keyword + graph.
//!
//! This card delivers the TRAIT contract plus the fusion core of `search`
//! (`hybrid_search`, pure + provable on kv-mem). `think`/`gap` bodies and the
//! live FTS+graph rank sources land in g2 (`kavach think`) — they are DECLARED
//! here, not silently deferred.
use crate::error::Result;
use crate::rrf::{RRF_K, rrf_fuse};

/// One fused retrieval hit: the row id and its Reciprocal-Rank-Fusion score.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BrainHit {
    /// Stable id of the retrieved row (e.g. `decision:<key>`).
    pub id: String,
    /// RRF score across the contributing rank lists; higher = more relevant.
    pub score: f64,
}

/// A report of what the brain does NOT know for a query — the gap-analysis that
/// auto-files research cards in `think` mode (g2 fills this in).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct GapReport {
    /// Free-text gaps the synthesis could not answer from retrieved context.
    pub missing: Vec<String>,
}

/// The unified brain surface. One trait so callers (CLI, gates, web) speak ONE
/// vocabulary instead of reaching into FTS, graph, and RRF separately.
#[expect(
    async_fn_in_trait,
    reason = "single-crate internal trait; no Send bound needed across the in-proc dispatch"
)]
pub trait KavachBrain {
    /// Hybrid keyword+graph retrieval, ranked by RRF. The default impl fuses
    /// pre-ranked source lists via [`hybrid_search`]; live FTS+graph queries are
    /// supplied by the implementor.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<BrainHit>>;

    /// Gap analysis: what the retrieved context does NOT cover (g2).
    async fn gap(&self, query: &str) -> Result<GapReport>;
}

/// Fuse per-source ranked id lists into one RRF-ranked hit list, truncated to
/// `limit`. This is the engine-independent core of `KavachBrain::search`: each
/// `ranked` slice is one retrieval source (FTS-rank, graph-proximity-rank, …),
/// most-relevant first. Rank-only fusion, so the incomparable BM25-score and
/// graph-hop scales never need normalizing.
#[must_use]
pub fn hybrid_search(ranked: &[&[String]], limit: usize) -> Vec<BrainHit> {
    let mut hits: Vec<BrainHit> = rrf_fuse(ranked, RRF_K)
        .into_iter()
        .map(|(id, score)| BrainHit { id, score })
        .collect();
    hits.truncate(limit);
    hits
}

#[cfg(test)]
#[path = "brain_test.rs"]
mod brain_test;
