// SPEC: docs/architecture/session-occupancy-lease.md
// Pure-data unit tests for lease types — no DB roundtrip; integration tests live in tests/ once lib.rs unlocks.
// SOURCE: https://doc.rust-lang.org/book/ch11-03-test-organization.html
use super::types::{AcquireOutcome, LEASE_TTL_SECS, Lease};
use chrono::{Duration, Utc};

#[test]
fn lease_ttl_is_five_minutes() {
    assert_eq!(
        LEASE_TTL_SECS, 300,
        "TTL must match spec §Heartbeat 5m window"
    );
}

#[test]
fn lease_clone_preserves_all_fields() {
    let now = Utc::now();
    let a = Lease {
        session_id: "s1".to_owned(),
        epoch: 7,
        expires_at: now,
    };
    let b = a.clone();
    assert_eq!(a.session_id, b.session_id);
    assert_eq!(a.epoch, b.epoch);
    assert_eq!(a.expires_at, b.expires_at);
    assert_eq!(a, b, "PartialEq must agree on all three fields");
}

#[test]
fn lease_inequality_on_epoch() {
    let now = Utc::now();
    let a = Lease {
        session_id: "s1".to_owned(),
        epoch: 1,
        expires_at: now,
    };
    let b = Lease {
        session_id: "s1".to_owned(),
        epoch: 2,
        expires_at: now,
    };
    assert_ne!(
        a, b,
        "different epochs must compare unequal — fencing-token integrity"
    );
}

#[test]
fn acquire_outcome_acquired_variant() {
    let now = Utc::now();
    let lease = Lease {
        session_id: "s1".to_owned(),
        epoch: 3,
        expires_at: now,
    };
    let out = AcquireOutcome::Acquired(lease.clone());
    assert!(
        matches!(out, AcquireOutcome::Acquired(l) if l == lease),
        "expected Acquired variant"
    );
}

#[test]
fn acquire_outcome_held_by_variant() {
    let now = Utc::now();
    let expires = now + Duration::seconds(LEASE_TTL_SECS);
    let out = AcquireOutcome::HeldBy {
        session_id: "rival".to_owned(),
        expires_at: expires,
    };
    assert!(
        matches!(
            out,
            AcquireOutcome::HeldBy {
                session_id,
                expires_at,
            } if session_id == "rival" && expires_at == expires
        ),
        "expected HeldBy variant with correct session_id and expires_at"
    );
}

#[test]
fn lease_debug_includes_session_id() {
    let lease = Lease {
        session_id: "test-sid-42".to_owned(),
        epoch: 5,
        expires_at: Utc::now(),
    };
    let dbg = format!("{lease:?}");
    assert!(
        dbg.contains("test-sid-42"),
        "Debug impl must surface session_id (non-secret identifier)"
    );
    assert!(
        dbg.contains("epoch=5"),
        "Debug impl must surface epoch (fencing-token visibility)"
    );
}
