// Read-side proofs: ranking query + practice_delta_mermaid renderer.
use super::{
    AntiPatternRanked, mistake_row_mermaid, practice_delta_focus_filter, practice_delta_mermaid,
    top_anti_patterns,
};
use crate::error::Result;

#[test]
fn mistake_row_renders_single_banned_to_fix_dag() {
    let m = mistake_row_mermaid(
        "permission",
        "should I proceed?",
        "auto-continue; never ask",
        4,
    );
    assert!(m.starts_with("graph LR\n"), "{m}");
    assert!(m.contains("BANNED [permission]"), "{m}");
    assert!(m.contains("should I proceed?"), "{m}");
    assert!(m.contains("×4"), "recurrence count on banned node: {m}");
    assert!(m.contains("INSTEAD: auto-continue; never ask"), "{m}");
    assert!(m.contains("w -.fixed by.-> b"), "fix edge: {m}");
}

#[test]
fn mistake_row_escapes_quotes_in_labels() {
    // A banned sample containing a double-quote must not break the Mermaid
    // quoted-string label (would corrupt the whole diagram).
    let m = mistake_row_mermaid("g", "he said \"hi\"", "fix", 1);
    assert!(!m.contains("\"hi\""), "raw quote leaked into label: {m}");
    assert!(m.contains("&quot;"), "{m}");
}

#[test]
fn practice_delta_renders_worst_vs_best_with_fix_edges() {
    let ranked = vec![
        AntiPatternRanked {
            name: "anti.continuation_menu.395f9852".to_owned(),
            gate: "stop".to_owned(),
            correct_action: "auto-continue; never ask permission".to_owned(),
            hit_count: 7,
        },
        AntiPatternRanked {
            name: "anti.x_internal_secret.abcd".to_owned(),
            gate: "pre_write".to_owned(),
            correct_action: "RFC 9421 httpsig".to_owned(),
            hit_count: 3,
        },
    ];
    let m = practice_delta_mermaid(&ranked).expect("non-empty");
    assert!(m.starts_with("graph LR\n"), "{m}");
    assert!(m.contains("WORST") && m.contains("BEST"), "{m}");
    // worst slug short-formed (hash stripped), hit count + gate shown
    assert!(m.contains("continuation_menu"), "{m}");
    assert!(m.contains("×7 via stop"), "{m}");
    // fix edge worst -> best
    assert!(m.contains("w0 -.fixed by.-> b0"), "{m}");
    assert!(m.contains("RFC 9421 httpsig"), "{m}");
}

#[test]
fn practice_delta_empty_yields_none() {
    assert!(practice_delta_mermaid(&[]).is_none());
}
use crate::graph::mistakes::{append_mistake_event, cluster_event_to_pattern};
use crate::open_memory;

/// Route one mistake observation through the capture path under `gate`/`fix`.
/// Distinct (gate, fix) pairs cluster to distinct `anti_patterns` by content key
/// (`anti.<gate>.<blake3(fix)[..8]>`); the same pair re-clusters to the same node.
/// Helper returns `Result` and propagates with `?` — it asserts nothing, so it
/// does not trip `panic_in_result_fn` (which fires only on asserting bodies).
async fn seed(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    gate: &str,
    fix: &str,
) -> Result<()> {
    let ev = append_mistake_event(db, gate, fix, "banned phrase", "sess", Some("proj")).await?;
    cluster_event_to_pattern(db, &ev, gate, fix).await?;
    Ok(())
}

#[tokio::test]
async fn ranks_anti_patterns_by_recurrence() {
    let db = open_memory().await.expect("open in-memory db");
    // Cluster A: 3 recurrences of the same behavioral mistake → hit_count 3.
    for _ in 0..3 {
        seed(&db, "gate_a", "do A instead")
            .await
            .expect("seed cluster A");
    }
    // Cluster B: 1 recurrence → hit_count 1.
    seed(&db, "gate_b", "do B instead")
        .await
        .expect("seed cluster B");

    let top = top_anti_patterns(&db, 10)
        .await
        .expect("read anti-patterns");
    assert_eq!(top.len(), 2, "two distinct clusters expected, got {top:?}");
    // Most-recurrent first.
    assert_eq!(top[0].gate, "gate_a", "A (3 hits) must rank first");
    assert_eq!(top[0].hit_count, 3);
    assert_eq!(top[0].correct_action, "do A instead");
    assert_eq!(top[1].gate, "gate_b");
    assert_eq!(top[1].hit_count, 1);
}

#[tokio::test]
async fn limit_truncates_to_top_n() {
    let db = open_memory().await.expect("open in-memory db");
    seed(&db, "gate_a", "fix a")
        .await
        .expect("seed a");
    seed(&db, "gate_b", "fix b")
        .await
        .expect("seed b");
    seed(&db, "gate_c", "fix c")
        .await
        .expect("seed c");

    let top = top_anti_patterns(&db, 2).await.expect("read anti-patterns");
    assert_eq!(top.len(), 2, "limit=2 must cap the result");
}

#[tokio::test]
async fn empty_graph_returns_no_rows() {
    let db = open_memory().await.expect("open in-memory db");
    let top = top_anti_patterns(&db, 10)
        .await
        .expect("read anti-patterns");
    assert!(top.is_empty(), "no anti_patterns ⇒ empty result");
}
