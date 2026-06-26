//! Proofs for the Stop-path spool glue. The daemon is NOT running under the test
//! harness, so every `kavach_rpc::client::call` Errs — which is exactly the
//! failure path we need: `call_or_spool` must then durably enqueue, and
//! `drain_and_replay` must drain what a prior failed Stop left.

use super::{call_or_spool, drain_and_replay};
use kavach_session::{drain_write_spool, paths::set_test_state_dir};

/// Isolate `state_dir()` to a fresh temp dir so the on-disk spool never crosses
/// tests (the spool file lives under `state_dir()`).
fn isolate(tag: &str) {
    let dir = std::env::temp_dir().join(format!("kavach-engine-spool-{tag}"));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("temp state dir");
    set_test_state_dir(Some(dir));
}

#[test]
fn call_or_spool_enqueues_when_daemon_is_down() {
    isolate("enqueue");
    // No daemon in tests → the live call Errs → the write must be spooled.
    call_or_spool("db.write", &serde_json::json!({"k": 1}));
    let spooled = drain_write_spool().expect("drain");
    assert_eq!(
        spooled.len(),
        1,
        "failed call spooled, not dropped: {spooled:?}"
    );
    assert_eq!(spooled[0].method, "db.write");
}

#[test]
fn drain_and_replay_is_a_noop_when_spool_empty() {
    isolate("empty");
    // Nothing spooled → replay drains an empty spool and returns cleanly.
    drain_and_replay();
    assert!(
        drain_write_spool().expect("drain").is_empty(),
        "empty stays empty"
    );
}

#[test]
fn drain_and_replay_re_spools_writes_that_fail_again() {
    isolate("respool");
    // Seed the spool via a failed call, then replay: the daemon is still down, so
    // each replay fails and is re-spooled — the signal survives across Stops.
    call_or_spool("gate_pattern.upsert", &serde_json::json!({"project": "p"}));
    assert_eq!(drain_write_spool().expect("seed").len(), 1);
    // Re-seed (drain above consumed it) and prove replay re-spools.
    call_or_spool("gate_pattern.upsert", &serde_json::json!({"project": "p"}));
    drain_and_replay();
    let after = drain_write_spool().expect("after replay");
    assert_eq!(
        after.len(),
        1,
        "replay that fails again re-spools, never drops: {after:?}"
    );
    assert_eq!(after[0].method, "gate_pattern.upsert");
}
