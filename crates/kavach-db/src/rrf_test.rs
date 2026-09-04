// Proves Reciprocal Rank Fusion (Brain-OS g1b): the fused order rewards
// consensus across lists, omits absent ids per-list, and is deterministic.
use super::{RRF_K, rrf_fuse};

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn consensus_id_outranks_a_single_list_top() {
    // "b" is rank-2 in BOTH lists; "a" is rank-1 in only the first.
    // RRF: a = 1/(60+1) = .01639 ; b = 1/(60+2)+1/(60+2) = .03226 -> b wins.
    let fts = ids(&["a", "b"]);
    let graph = ids(&["c", "b"]);
    let fused = rrf_fuse(&[&fts, &graph], RRF_K);
    assert_eq!(
        fused[0].0, "b",
        "id surfaced by both lists ranks first: {fused:?}"
    );
}

#[test]
fn absent_id_contributes_nothing() {
    // "x" appears only in list 1; "y" only in list 2, both at rank 1 -> tie,
    // broken by id ascending ("x" < "y"). Neither gets a phantom second term.
    let l1 = ids(&["x"]);
    let l2 = ids(&["y"]);
    let fused = rrf_fuse(&[&l1, &l2], RRF_K);
    assert_eq!(fused.len(), 2);
    let sx = fused.iter().find(|(id, _)| id == "x").unwrap().1;
    let sy = fused.iter().find(|(id, _)| id == "y").unwrap().1;
    assert!(
        (sx - sy).abs() < f64::EPSILON,
        "single-list rank-1 ties: {fused:?}"
    );
    assert_eq!(fused[0].0, "x", "id-ascending tie-break");
}

#[test]
fn rank_position_drives_score_monotonically() {
    // One list, three ids: score must strictly decrease with rank.
    let only = ids(&["first", "second", "third"]);
    let fused = rrf_fuse(&[&only], RRF_K);
    assert_eq!(fused[0].0, "first");
    assert_eq!(fused[1].0, "second");
    assert_eq!(fused[2].0, "third");
    assert!(
        fused[0].1 > fused[1].1 && fused[1].1 > fused[2].1,
        "{fused:?}"
    );
}

#[test]
fn empty_input_yields_empty_output() {
    let fused = rrf_fuse(&[], RRF_K);
    assert!(fused.is_empty());
    let empty_lists: Vec<String> = Vec::new();
    let fused2 = rrf_fuse(&[&empty_lists], RRF_K);
    assert!(fused2.is_empty(), "empty lists => empty fusion");
}

#[test]
fn low_k_lets_top_pick_dominate_more_than_high_k() {
    // With small k, the gap between rank-1 and a consensus rank-2/2 narrows;
    // this just asserts k is actually wired (different k => different scores).
    let a = ids(&["top", "mid"]);
    let b = ids(&["other", "mid"]);
    let low = rrf_fuse(&[&a, &b], 1.0);
    let high = rrf_fuse(&[&a, &b], 1000.0);
    let low_mid = low.iter().find(|(id, _)| id == "mid").unwrap().1;
    let high_mid = high.iter().find(|(id, _)| id == "mid").unwrap().1;
    assert!(low_mid > high_mid, "smaller k => larger per-term weight");
}
