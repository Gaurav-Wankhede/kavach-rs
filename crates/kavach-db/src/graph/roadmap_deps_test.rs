use super::{is_in_cycle_sql, mirror_card_deps};
use crate::connection::open_memory;

fn deps(ks: &[&str]) -> Vec<String> {
    ks.iter().map(|s| (*s).to_owned()).collect()
}

#[tokio::test]
async fn depless_card_has_no_cycle() {
    let db = open_memory().await.expect("mem db");
    mirror_card_deps(&db, "a", &[]).await.expect("mirror");
    assert!(!is_in_cycle_sql(&db, "a").await.expect("cycle check"));
}

#[tokio::test]
async fn linear_chain_has_no_cycle() {
    // c -> b -> a (c depends on b depends on a). No back-edge.
    let db = open_memory().await.expect("mem db");
    mirror_card_deps(&db, "b", &deps(&["a"]))
        .await
        .expect("mirror b");
    mirror_card_deps(&db, "c", &deps(&["b"]))
        .await
        .expect("mirror c");
    assert!(!is_in_cycle_sql(&db, "c").await.expect("cycle c"));
    assert!(!is_in_cycle_sql(&db, "a").await.expect("cycle a"));
}

#[tokio::test]
async fn self_dependency_is_a_cycle() {
    // The boundary case the Rust DFS also flags: a depends on a.
    let db = open_memory().await.expect("mem db");
    mirror_card_deps(&db, "a", &deps(&["a"]))
        .await
        .expect("mirror");
    assert!(is_in_cycle_sql(&db, "a").await.expect("cycle check"));
}

#[tokio::test]
async fn mutual_dependency_is_a_cycle() {
    // a -> b -> a : both participate in the cycle.
    let db = open_memory().await.expect("mem db");
    mirror_card_deps(&db, "a", &deps(&["b"]))
        .await
        .expect("mirror a");
    mirror_card_deps(&db, "b", &deps(&["a"]))
        .await
        .expect("mirror b");
    assert!(is_in_cycle_sql(&db, "a").await.expect("cycle a"));
    assert!(is_in_cycle_sql(&db, "b").await.expect("cycle b"));
}

#[tokio::test]
async fn re_mirror_is_idempotent_no_duplicate_edges() {
    // Mirroring twice must not stack edges; after a re-mirror that REMOVES the
    // dep, the card is no longer in a cycle (last-writer-wins convergence).
    let db = open_memory().await.expect("mem db");
    mirror_card_deps(&db, "a", &deps(&["b"])).await.expect("m1");
    mirror_card_deps(&db, "b", &deps(&["a"])).await.expect("m2");
    assert!(is_in_cycle_sql(&db, "a").await.expect("cyclic before"));
    // Re-mirror a with NO deps -> breaks the cycle.
    mirror_card_deps(&db, "a", &[]).await.expect("m3");
    assert!(!is_in_cycle_sql(&db, "a").await.expect("acyclic after"));
}

#[tokio::test]
async fn unknown_card_is_not_cyclic() {
    let db = open_memory().await.expect("mem db");
    // The entity table exists once any card has been mirrored; cycle-checking a
    // DIFFERENT, never-declared key must return false (no edges => no cycle).
    mirror_card_deps(&db, "some-other-card", &[])
        .await
        .expect("seed table");
    assert!(
        !is_in_cycle_sql(&db, "never-created")
            .await
            .expect("absent ok")
    );
}

#[tokio::test]
async fn malformed_key_is_rejected() {
    let db = open_memory().await.expect("mem db");
    assert!(mirror_card_deps(&db, "bad key!", &[]).await.is_err());
    assert!(mirror_card_deps(&db, "", &[]).await.is_err());
    assert!(is_in_cycle_sql(&db, "bad/key").await.is_err());
}
