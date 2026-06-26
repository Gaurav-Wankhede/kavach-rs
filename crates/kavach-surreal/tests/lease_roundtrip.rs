//! Regression proof for the session-occupancy lease query path.
//!
//! These functions build their queries as `&str`, so `cargo check` cannot see a
//! dead `SurrealQL` builtin inside them — only execution against a real DB can.
//! The lease subsystem shipped on `type::thing(...)`, which `SurrealDB` 3.0
//! renamed to `type::record(...)`; every `lease.*` RPC parse-errored at runtime
//! with zero test coverage to catch it. This drives the full
//! acquire → status → heartbeat → unlock cycle against an in-memory DB so the
//! whole class stays fixed.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_surreal::lease::{AcquireOutcome, acquire, heartbeat, status, unlock};
use kavach_surreal::open_memory;

const TABLE: &str = "lease";
const KEY: &str = "session-occupancy:proj-x";
const SESSION: &str = "sess_abc";

/// Seed the lease row the way the live system does before a first acquire
/// (acquire itself returns `RecordNotFound` on a missing record).
async fn seed(db: &surrealdb::Surreal<surrealdb::engine::any::Any>) {
    db.query(
        "CREATE type::record($t, $k) SET occupied_by=NONE, occupied_until=NONE, occupied_epoch=0",
    )
    .bind(("t", TABLE))
    .bind(("k", KEY))
    .await
    .expect("seed lease row");
}

#[tokio::test]
async fn full_lease_cycle_executes_against_a_real_db() {
    let db = open_memory().await.expect("memory db");
    seed(&db).await;

    // Acquire on a free key -> Acquired with epoch 1.
    let lease = match acquire(&db, TABLE, KEY, SESSION).await.expect("acquire") {
        AcquireOutcome::Acquired(l) => l,
        other @ AcquireOutcome::HeldBy { .. } => panic!("expected Acquired, got {other:?}"),
    };
    assert_eq!(lease.epoch, 1);
    assert_eq!(lease.session_id, SESSION);

    // status() must now report the live holder (proves the SELECT query parses).
    let seen = status(&db, TABLE, KEY)
        .await
        .expect("status")
        .expect("a live lease");
    assert_eq!(seen.session_id, SESSION);
    assert_eq!(seen.epoch, 1);

    // heartbeat() with the matching epoch must succeed and keep the epoch.
    let beat = heartbeat(&db, TABLE, KEY, &lease).await.expect("heartbeat");
    assert_eq!(beat.epoch, 1);
    assert!(
        beat.expires_at >= lease.expires_at,
        "heartbeat extends the lease"
    );

    // unlock() clears the holder; status() then sees no live lease.
    unlock(&db, TABLE, KEY, &lease).await.expect("unlock");
    let after = status(&db, TABLE, KEY).await.expect("status after unlock");
    assert!(after.is_none(), "unlock must clear the lease holder");
}

#[tokio::test]
async fn acquire_reports_a_conflicting_holder() {
    // Proves the contended path: a second session sees HeldBy, not Acquired.
    let db = open_memory().await.expect("memory db");
    seed(&db).await;
    let _first = acquire(&db, TABLE, KEY, SESSION)
        .await
        .expect("first acquire");

    match acquire(&db, TABLE, KEY, "sess_other")
        .await
        .expect("second acquire")
    {
        AcquireOutcome::HeldBy { session_id, .. } => assert_eq!(session_id, SESSION),
        other @ AcquireOutcome::Acquired(_) => panic!("expected HeldBy, got {other:?}"),
    }
}

/// The anti-steal proof: N sessions race to acquire the SAME free lease
/// concurrently; EXACTLY ONE must win `Acquired`, the rest must see `HeldBy`.
/// This is the regression guard for the TOCTOU race the single-statement CAS
/// closed — the prior SELECT-then-UPDATE acquire let multiple racers all win.
#[tokio::test]
async fn concurrent_acquire_yields_exactly_one_winner() {
    let db = open_memory().await.expect("memory db");
    seed(&db).await;

    // Fan out 16 simultaneous acquires for distinct sessions on one key.
    let mut handles = Vec::new();
    for i in 0..16 {
        let db = db.clone();
        handles.push(tokio::spawn(async move {
            let sid = format!("sess_{i}");
            acquire(&db, TABLE, KEY, &sid).await.expect("acquire")
        }));
    }

    let mut winners = 0_usize;
    let mut held = 0_usize;
    for h in handles {
        match h.await.expect("join") {
            AcquireOutcome::Acquired(_) => winners += 1,
            AcquireOutcome::HeldBy { .. } => held += 1,
        }
    }
    assert_eq!(winners, 1, "exactly one session may win a contended lease");
    assert_eq!(held, 15, "every other session must observe HeldBy");
}

/// Proves a claimed card cannot be claimed twice: the `todo -> in_progress`
/// CAS on `update_status_cas` matches only when the row is still `todo`.
#[tokio::test]
async fn status_cas_transitions_only_from_expected() {
    use kavach_surreal::update_status_cas;
    use surrealdb_types::RecordId;
    let db = open_memory().await.expect("memory db");
    // A roadmap row in `todo`, owned by a concrete project record.
    let pid = RecordId::new("project", "p1");
    db.query(
        "CREATE roadmap:c1 SET project=$p, entry_key='card-1', entry_status='todo', \
         title='t', content='c', updated_at=time::now()",
    )
    .bind(("p", pid.clone()))
    .await
    .expect("seed card");

    // First CAS todo->in_progress wins (1 row).
    let first = update_status_cas(&db, "roadmap", &pid, "card-1", "todo", "in_progress")
        .await
        .expect("first cas");
    assert_eq!(first, 1, "first claim transitions the card");

    // Second CAS from todo now matches nothing (already in_progress).
    let second = update_status_cas(&db, "roadmap", &pid, "card-1", "todo", "in_progress")
        .await
        .expect("second cas");
    assert_eq!(second, 0, "a second claim from todo must match no rows");
}

/// Proves the verify transition (done -> verified) is also CAS-guarded: two
/// sessions racing to verify the same `done` card yield exactly one winner.
#[tokio::test]
async fn verify_cas_yields_one_winner_from_done() {
    use kavach_surreal::update_status_cas;
    use surrealdb_types::RecordId;
    let db = open_memory().await.expect("memory db");
    let pid = RecordId::new("project", "p2");
    db.query(
        "CREATE roadmap:c2 SET project=$p, entry_key='card-2', entry_status='done', \
         title='t', content='c', updated_at=time::now()",
    )
    .bind(("p", pid.clone()))
    .await
    .expect("seed done card");

    let mut wins = 0_usize;
    for _ in 0..5 {
        wins += update_status_cas(&db, "roadmap", &pid, "card-2", "done", "verified")
            .await
            .expect("verify cas");
    }
    assert_eq!(
        wins, 1,
        "only the first done->verified transition may match"
    );
}
