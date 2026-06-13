//! Threshold classification + pairwise duplicate check.
//!
//! A Jaccard score maps to one of three discrete decisions; `check` runs the
//! candidate against every existing body and keeps the worst (highest) match.
use super::shingle::{jaccard, shingles};
use crate::gates::gate_config::gate_threshold;

/// Decision returned by the guard for one comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DupDecision {
    /// Below the advise cutoff — independent functions.
    Clean,
    /// advise..block — extract a shared helper.
    Advise,
    /// >= block cutoff — near-identical copy-paste.
    Block,
}

/// The project whose gate-config overlay these thresholds resolve against.
const PROJECT: &str = "kavach-rs";
/// Compiled defaults.
///
/// The values used when no DB override exists (fail-closed: a missing row never
/// changes the historical behavior). The DB key namespace `dup.*` lets an
/// operator retune copy-paste sensitivity per project at runtime.
const ADVISE_THRESHOLD: f64 = 0.70;
const BLOCK_THRESHOLD: f64 = 0.85;

/// Classify a Jaccard score into a discrete decision.
///
/// Cutoffs resolve through the dynamic gate-config overlay (`dup.advise` /
/// `dup.block`), each falling back to its compiled default on any miss.
pub(crate) fn classify(score: f64) -> DupDecision {
    let advise = gate_threshold(PROJECT, "dup.advise", ADVISE_THRESHOLD);
    let block = gate_threshold(PROJECT, "dup.block", BLOCK_THRESHOLD);
    if score >= block {
        DupDecision::Block
    } else if score >= advise {
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
