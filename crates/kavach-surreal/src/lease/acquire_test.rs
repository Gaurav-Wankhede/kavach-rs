//! Proof of the occupancy-lease CAS that the fused `claim_card` rests on
//! (roadmap.unit.dispatch-lease-fused-claim). The lease is the holder+liveness
//! layer that bare `entry_status` lacks: it is what stops a second LIVE session
//! resuming a hung holder's card. These tests pin the three load-bearing
//! guarantees — single-winner under contention, fence-epoch monotonicity, and
//! TTL-expiry reclaimability — at the primitive level, with a real in-memory
//! `SurrealDB` so the WHERE-clause CAS is exercised, not mocked.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::acquire;
use crate::lease::status;
use crate::lease::types::AcquireOutcome;
use crate::open_memory;
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

/// Seed a roadmap card with an UNHELD lease (the fresh-claim shape: no holder,
/// no `occupied_until`), so the first `acquire` must win it.
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
async fn two_racers_exactly_one_acquires() {
    // AC5a: the core single-winner guarantee. Two sessions acquire the SAME key;
    // the CAS WHERE-clause (unheld OR expired OR ours) must match for exactly one.
    let db = open_memory().await.expect("mem db");
    seed_unheld(&db, "card-x").await;

    let first = acquire(&db, "roadmap", "card-x", "sess-A")
        .await
        .expect("first acquire");
    let second = acquire(&db, "roadmap", "card-x", "sess-B")
        .await
        .expect("second acquire");

    assert!(
        matches!(first, AcquireOutcome::Acquired(ref l) if l.session_id == "sess-A"),
        "the first racer must WIN the unheld lease, got {first:?}"
    );
    assert!(
        matches!(second, AcquireOutcome::HeldBy { ref session_id, .. } if session_id == "sess-A"),
        "the second racer must LOSE and see sess-A as the live holder, got {second:?}"
    );
}

#[tokio::test]
async fn reacquire_by_holder_bumps_fence_epoch() {
    // AC4: the fence is monotonic. The holder re-acquiring (the renew path) must
    // bump occupied_epoch so a stale evicted holder cannot forge a current token.
    let db = open_memory().await.expect("mem db");
    seed_unheld(&db, "card-e").await;

    let AcquireOutcome::Acquired(first) = acquire(&db, "roadmap", "card-e", "sess-A")
        .await
        .expect("first acquire")
    else {
        panic!("first acquire must win");
    };
    let AcquireOutcome::Acquired(second) = acquire(&db, "roadmap", "card-e", "sess-A")
        .await
        .expect("holder re-acquire")
    else {
        panic!("holder re-acquire must win (WHERE includes occupied_by=ours)");
    };
    assert!(
        second.epoch > first.epoch,
        "fence epoch must strictly increase on re-acquire: {} -> {}",
        first.epoch,
        second.epoch
    );
}

#[tokio::test]
async fn expired_lease_is_reacquirable_by_foreign_session() {
    // AC5c: TTL expiry is the crash-recovery edge. A lease whose occupied_until
    // is in the PAST must be acquirable by a different session — otherwise a
    // crashed holder would wedge the card forever.
    let db = open_memory().await.expect("mem db");
    let past = Utc::now()
        .checked_sub_signed(Duration::seconds(10))
        .expect("past instant");
    db.query(
        "CREATE type::record('roadmap', $id) SET \
         occupied_by='sess-dead', occupied_until=$u, occupied_epoch=5, \
         entry_status='in_progress', entry_key=$id, project='p', title='t', \
         content='c', updated_at=time::now()",
    )
    .bind(("id", "card-exp".to_owned()))
    .bind(("u", past))
    .await
    .expect("seed expired lease");

    let outcome = acquire(&db, "roadmap", "card-exp", "sess-live")
        .await
        .expect("acquire over expired lease");
    assert!(
        matches!(outcome, AcquireOutcome::Acquired(ref l) if l.session_id == "sess-live" && l.epoch > 5),
        "a live session must reclaim an EXPIRED lease and bump the fence past 5, got {outcome:?}"
    );

    // And `status` must now report the new live holder, not the dead one.
    let holder = status(&db, "roadmap", "card-exp")
        .await
        .expect("status query");
    assert_eq!(
        holder.map(|l| l.session_id).as_deref(),
        Some("sess-live"),
        "status must surface the reclaiming session as the live holder"
    );
}
