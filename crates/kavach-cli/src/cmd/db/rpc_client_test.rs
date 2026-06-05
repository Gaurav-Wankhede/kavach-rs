use super::*;

#[test]
fn fallback_only_when_daemon_unavailable() {
    assert!(should_fallback_to_direct(DAEMON_UNAVAILABLE));
}

#[test]
fn no_fallback_on_live_daemon_rpc_errors() {
    // Daemon is UP (holding the LOCK) — every non-unavailable error must
    // NOT trigger a competing direct RocksDB open.
    for e in [
        "io: broken pipe",
        "json: expected value",
        "rpc[-32603]: internal error",
        "rpc[-32601]: Method not found",
        "no_result",
    ] {
        assert!(
            !should_fallback_to_direct(e),
            "{e:?} must not fall back to a 2nd RocksDB handle"
        );
    }
}

#[test]
fn lock_contention_detected_on_restart_race_signal() {
    // Exact string observed live when a mid-restart daemon held the lock.
    for e in [
        "open SurrealDB: SurrealDB error: There was a problem with a transaction: \
         IO error: While lock file: /…/kavach.surreal/LOCK: Resource temporarily unavailable",
        "LOCK: Resource temporarily unavailable",
        "While lock file: LOCK:",
    ] {
        assert!(
            is_rocksdb_lock_contention(e),
            "{e:?} is the restart-race contention signal — must trigger retry"
        );
    }
}

#[test]
fn lock_contention_excludes_non_lock_errors() {
    // Must NOT mistake an ordinary failure for the restart race (else we
    // would retry-loop a permanent error instead of surfacing it).
    for e in [
        DAEMON_UNAVAILABLE,
        "io: broken pipe",
        "json: expected value",
        "error: schema apply: table missing",
    ] {
        assert!(
            !is_rocksdb_lock_contention(e),
            "{e:?} is not lock contention — must surface, not retry"
        );
    }
}

#[test]
fn backoff_is_strictly_bounded_and_monotonic() {
    // CWE-835 guard: finite, ascending, sane ceiling — a stuck lock must
    // surface the real error, never spin.
    let steps: Vec<_> = fallback_backoff_schedule().collect();
    assert_eq!(steps.len(), 5, "exactly 5 attempts — finite");
    assert!(
        steps.iter().zip(steps.iter().skip(1)).all(|(a, b)| a < b),
        "monotonically increasing backoff"
    );
    let total: std::time::Duration = steps.iter().sum();
    assert!(
        total <= std::time::Duration::from_secs(4),
        "ceiling ≤4s so a stale lock surfaces fast, got {total:?}"
    );
}
