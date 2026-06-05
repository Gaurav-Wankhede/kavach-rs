//! Threshold classification + pairwise duplicate check.
//!
//! A Jaccard score maps to one of three discrete decisions; `check` runs the
//! candidate against every existing body and keeps the worst (highest) match.
use super::shingle::{jaccard, shingles};

/// Decision returned by the guard for one comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DupDecision {
    /// Below 0.70 — independent functions.
    Clean,
    /// 0.70..0.85 — extract a shared helper.
    Advise,
    /// >= 0.85 — near-identical copy-paste.
    Block,
}

const ADVISE_THRESHOLD: f64 = 0.70;
const BLOCK_THRESHOLD: f64 = 0.85;

/// Classify a Jaccard score into a discrete decision.
pub(crate) fn classify(score: f64) -> DupDecision {
    if score >= BLOCK_THRESHOLD {
        DupDecision::Block
    } else if score >= ADVISE_THRESHOLD {
        DupDecision::Advise
    } else {
        DupDecision::Clean
    }
}

/// Check a candidate function body against a list of existing function bodies.
/// Returns the worst (highest) decision and the matched body's index, if any.
/// SOURCE: blog.nelhage.com/post/fuzzy-dedup/ — pairwise Jaccard for small N.
pub(crate) fn check(candidate: &str, existing: &[&str]) -> (DupDecision, Option<usize>) {
    let cand_shingles = shingles(candidate);
    if cand_shingles.is_empty() {
        return (DupDecision::Clean, None);
    }
    let mut worst = (DupDecision::Clean, None);
    let mut worst_score = 0.0_f64;
    for (idx, body) in existing.iter().enumerate() {
        let other = shingles(body);
        if other.is_empty() {
            continue;
        }
        let score = jaccard(&cand_shingles, &other);
        if score > worst_score {
            worst_score = score;
            worst = (classify(score), Some(idx));
        }
    }
    worst
}
