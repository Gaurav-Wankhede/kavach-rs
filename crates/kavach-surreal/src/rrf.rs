//! Reciprocal Rank Fusion — Brain-OS Gap 1 (roadmap.unit.harness.brain-os.g1b).
//!
//! Fuses N independently-ranked id lists into one consensus ranking WITHOUT
//! score normalization: each list contributes `1/(k + rank)` per id (1-based
//! rank), summed across lists, sorted descending. This is exactly right for
//! kavach's hybrid retrieval because the two input signals — BM25/FTS rank
//! (g1a) and graph-proximity rank (edge distance in the concept/bridge/mistake
//! DAG) — live on INCOMPARABLE scales (a BM25 score vs a hop count); RRF needs
//! only the rank POSITIONS, never the raw scores. No vector list participates —
//! the ONNX embedder was removed (decision/onnx-removal-dag-rlaif-only).
//!
//! `k = 60` is the canonical constant (TREC-2009 empirical sweet spot; the
//! default in `OpenSearch` / `Elasticsearch` / Azure AI Search / `MongoDB` / `Weaviate`).
//! Low k lets a single top-1 dominate; high k rewards consensus across lists.
//! SOURCE: research.reciprocal-rank-fusion-rrf.
use std::collections::HashMap;

/// Canonical RRF rank constant. See module docs.
pub const RRF_K: f64 = 60.0;

/// Fuse ranked id lists into one ranking by Reciprocal Rank Fusion.
///
/// Each inner slice is one source's ranking, most-relevant FIRST. An id absent
/// from a list simply contributes nothing for that list (the term is omitted),
/// so RRF naturally rewards ids that ALL sources surface. Ties in fused score
/// break by id ascending, for a deterministic order.
///
/// Returns `(id, rrf_score)` pairs sorted by descending score. `k` is the rank
/// constant (pass [`RRF_K`] for the canonical default); it must be positive.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "RRF is float-sum by definition: 1/(k+rank) over rank lists. Ranks are \
              bounded by SEARCH_LIMIT_MAX, far below f64's 2^52 exact-int range, so \
              the usize->f64 cast is lossless in practice; scores rank, never settle \
              money. SOURCE: research.reciprocal-rank-fusion-rrf"
)]
pub fn rrf_fuse(lists: &[&[String]], k: f64) -> Vec<(String, f64)> {
    let mut scores: HashMap<&str, f64> = HashMap::new();
    for list in lists {
        for (idx, id) in list.iter().enumerate() {
            // 1-based rank: the first element is rank 1, not 0.
            let rank = idx as f64 + 1.0;
            *scores.entry(id.as_str()).or_insert(0.0) += 1.0 / (k + rank);
        }
    }
    let mut fused: Vec<(String, f64)> = scores
        .into_iter()
        .map(|(id, s)| (id.to_owned(), s))
        .collect();
    // Higher score first; deterministic id-ascending tie-break.
    fused.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    fused
}

#[cfg(test)]
#[path = "rrf_test.rs"]
mod rrf_test;
