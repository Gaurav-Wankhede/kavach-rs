// Proves KavachBrain's fusion core (Brain-OS g3): hybrid_search fuses per-source
// rank lists via RRF, ranks consensus first, and truncates to the limit. The
// trait itself is a contract; this exercises the load-bearing `search` core.
use super::{BrainHit, hybrid_search};

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn fuses_two_sources_consensus_first() {
    // "shared" is in both lists (rank 2 + rank 1) -> highest RRF score.
    let fts = ids(&["fts_only", "shared"]);
    let graph = ids(&["shared", "graph_only"]);
    let hits = hybrid_search(&[&fts, &graph], 10);
    assert_eq!(hits[0].id, "shared", "consensus id ranks first: {hits:?}");
    assert_eq!(hits.len(), 3, "union of both sources, deduped");
}

#[test]
fn truncates_to_limit() {
    let only = ids(&["a", "b", "c", "d"]);
    let hits = hybrid_search(&[&only], 2);
    assert_eq!(hits.len(), 2, "limit caps the fused list");
    assert_eq!(hits[0].id, "a");
    assert_eq!(hits[1].id, "b");
}

#[test]
fn empty_sources_yield_no_hits() {
    let hits = hybrid_search(&[], 5);
    assert!(hits.is_empty());
}

#[test]
fn scores_are_descending() {
    let l = ids(&["x", "y", "z"]);
    let hits: Vec<BrainHit> = hybrid_search(&[&l], 10);
    assert!(
        hits.windows(2).all(|w| w[0].score >= w[1].score),
        "hits must be sorted by descending RRF score: {hits:?}"
    );
}
