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
async fn seed(db: &surrealdb::Surreal<surrealdb::engine::local::Db>) {
    db.query("CREATE type::record($t, $k) SET occupied_by=NONE, occupied_until=NONE, occupied_epoch=0")
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
    let seen = status(&db, TABLE, KEY).await.expect("status").expect("a live lease");
    assert_eq!(seen.session_id, SESSION);
    assert_eq!(seen.epoch, 1);

    // heartbeat() with the matching epoch must succeed and keep the epoch.
    let beat = heartbeat(&db, TABLE, KEY, &lease).await.expect("heartbeat");
    assert_eq!(beat.epoch, 1);
    assert!(beat.expires_at >= lease.expires_at, "heartbeat extends the lease");

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
    let _first = acquire(&db, TABLE, KEY, SESSION).await.expect("first acquire");

    match acquire(&db, TABLE, KEY, "sess_other").await.expect("second acquire") {
        AcquireOutcome::HeldBy { session_id, .. } => assert_eq!(session_id, SESSION),
        other @ AcquireOutcome::Acquired(_) => panic!("expected HeldBy, got {other:?}"),
    }
}
