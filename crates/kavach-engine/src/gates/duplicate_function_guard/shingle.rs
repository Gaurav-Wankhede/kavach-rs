//! Token-shingle signature build + Jaccard similarity.
//!
//! Tokenize → `SHINGLE_K`-gram shingle set → Jaccard coefficient. No external
//! deps; cheap and deterministic for the <1k-functions-per-crate regime.
use std::collections::HashSet;

/// Shingle size: 5-gram tokens.
/// SOURCE: Manning IR — n in [3,5] best for code; 5 reduces false positives.
const SHINGLE_K: usize = 5;
/// Minimum body size to bother checking. Tiny functions trivially overlap.
const MIN_BODY_TOKENS: usize = 30;

/// Tokenize a Rust function body by whitespace + punctuation boundaries.
fn tokenize(body: &str) -> Vec<&str> {
    body.split(|c: char| c.is_whitespace() || "{}()[];,.:?".contains(c))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Build a set of `SHINGLE_K`-gram shingles; empty when the body is too short.
pub(super) fn shingles(body: &str) -> HashSet<String> {
    let toks = tokenize(body);
    if toks.len() < MIN_BODY_TOKENS || toks.len() < SHINGLE_K {
        return HashSet::new();
    }
    let mut set = HashSet::with_capacity(toks.len());
    for window in toks.windows(SHINGLE_K) {
        set.insert(window.join(" "));
    }
    set
}

/// Jaccard similarity = |A ∩ B| / |A ∪ B|. Returns 0.0 when either set is empty.
pub(crate) fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "jaccard coefficient requires f64 for division; set cardinality fits in f64 range"
    )]
    let intersection = a.intersection(b).count() as f64;
    #[expect(
        clippy::cast_precision_loss,
        reason = "jaccard coefficient requires f64 for division; set cardinality fits in f64 range"
    )]
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        return 0.0;
    }
    #[expect(
        clippy::float_arithmetic,
        reason = "jaccard coefficient: intentional float division for similarity score"
    )]
    {
        intersection / union
    }
}
