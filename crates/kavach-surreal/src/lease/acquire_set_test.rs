//! Proof of the all-or-nothing batch lease-set (E8): a session reserves a Vec of
//! keys steal-proof, and a single conflict ROLLS BACK every partial win so no
//! orphaned half-set is left holding cards. Real in-memory `SurrealDB` so the CAS
//! WHERE-clause is exercised, not mocked.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{AcquireSetOutcome, acquire_set};
use crate::lease::{Lease, acquire, status, unlock};
use crate::lease::types::AcquireOutcome;
use crate::open_memory;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

async fn seed_unheld(db: &Surreal<Db>, id: &str) {
    db.query(
        "CREATE type::record('roadmap', $id) SET \
         occupied_by=NONE, occupied_until=NONE, occupied_epoch=NONE, \
         entry_status='in_progress', entry_key=$id, project='p', title='t', \
         content='c', updated_at=time::now()",
    )
    .bind(("id", id.to_owned()))
    .await
    .expect("seed unheld card");
}

#[tokio::test]
async fn all_unheld_keys_are_acquired() {
    let db = open_memory().await.expect("mem db");
    for id in ["a", "b", "c"] {
        seed_unheld(&db, id).await;
    }
    let out = acquire_set(&db, "roadmap", &["a", "b", "c"], "sess-A")
        .await
        .expect("acquire_set");
    let AcquireSetOutcome::AllAcquired(leases) = out else {
        panic!("all-unheld set must be fully acquired, got {out:?}");
    };
    assert_eq!(leases.len(), 3, "one lease per key in input order");
    for id in ["a", "b", "c"] {
        let holder = status(&db, "roadmap", id).await.expect("status");
        assert_eq!(
            holder.map(|l| l.session_id).as_deref(),
            Some("sess-A"),
            "key {id} must be held by sess-A after the batch"
        );
    }
}

#[tokio::test]
async fn one_conflict_rolls_back_the_whole_set() {
    // THE steal-proof guarantee: B already holds "b". A's batch [a, b, c] must
    // FAIL on b and RELEASE its win on "a" (rollback) — leaving a unheld, not
    // orphaned under sess-A.
    let db = open_memory().await.expect("mem db");
    for id in ["a", "b", "c"] {
        seed_unheld(&db, id).await;
    }
    // B takes the middle key first.
    let mid = acquire(&db, "roadmap", "b", "sess-B").await.expect("B acquire");
    assert!(matches!(mid, AcquireOutcome::Acquired(_)));

    let out = acquire_set(&db, "roadmap", &["a", "b", "c"], "sess-A")
        .await
        .expect("acquire_set with conflict");
    let AcquireSetOutcome::Conflict {
        conflict_key,
        held_by,
    } = out
    else {
        panic!("a contended key must yield Conflict, got {out:?}");
    };
    assert_eq!(conflict_key, "b", "the contended key is reported");
    assert_eq!(held_by, "sess-B", "the true holder is reported");

    // ROLLBACK proof: "a" (won then rolled back) must be UNHELD again, never
    // orphaned under sess-A; "b" still B's; "c" never touched (loop stopped at b).
    let a_holder = status(&db, "roadmap", "a").await.expect("status a");
    assert!(
        a_holder.is_none(),
        "the partial win on 'a' must be rolled back to unheld, got {a_holder:?}"
    );
    let b_holder = status(&db, "roadmap", "b").await.expect("status b");
    assert_eq!(
        b_holder.map(|l| l.session_id).as_deref(),
        Some("sess-B"),
        "'b' stays with its real holder"
    );
}

#[tokio::test]
async fn after_rollback_the_set_is_reacquirable() {
    // The rollback must leave keys CLEAN enough that a later batch (after the
    // conflict clears) fully succeeds — proving rollback released real leases.
    let db = open_memory().await.expect("mem db");
    for id in ["a", "b"] {
        seed_unheld(&db, id).await;
    }
    drop(acquire(&db, "roadmap", "b", "sess-B").await.expect("B acquire"));
    let first = acquire_set(&db, "roadmap", &["a", "b"], "sess-A")
        .await
        .expect("first batch");
    assert!(matches!(first, AcquireSetOutcome::Conflict { .. }));

    // B releases b; A's full set must now succeed (a was rolled back, so free).
    unlock(
        &db,
        "roadmap",
        "b",
        &Lease {
            session_id: "sess-B".to_owned(),
            // epoch from B's acquire was 1 (first acquire on a fresh card).
            epoch: 1,
            expires_at: chrono::Utc::now(),
        },
    )
    .await
    .expect("B unlock");

    let second = acquire_set(&db, "roadmap", &["a", "b"], "sess-A")
        .await
        .expect("second batch");
    assert!(
        matches!(second, AcquireSetOutcome::AllAcquired(ref v) if v.len() == 2),
        "after b is freed and a was rolled back, the full set must acquire, got {second:?}"
    );
}

#[tokio::test]
async fn duplicate_keys_are_deduped() {
    let db = open_memory().await.expect("mem db");
    seed_unheld(&db, "a").await;
    let out = acquire_set(&db, "roadmap", &["a", "a", "a"], "sess-A")
        .await
        .expect("acquire_set dups");
    let AcquireSetOutcome::AllAcquired(leases) = out else {
        panic!("a single deduped key must acquire, got {out:?}");
    };
    assert_eq!(leases.len(), 1, "duplicate keys collapse to one lease");
}

#[tokio::test]
async fn empty_key_set_is_trivially_all_acquired() {
    let db = open_memory().await.expect("mem db");
    let out = acquire_set(&db, "roadmap", &[], "sess-A")
        .await
        .expect("acquire_set empty");
    assert!(
        matches!(out, AcquireSetOutcome::AllAcquired(ref v) if v.is_empty()),
        "an empty request acquires the empty set, got {out:?}"
    );
}
