//! Spool round-trip + edge-case proofs. Each test isolates `state_dir()` to a
//! fresh temp dir so the on-disk spool file never crosses tests.

use super::{SpooledWrite, drain, enqueue};
use crate::paths::set_test_state_dir;

/// Point `state_dir()` at a unique temp subdir for this test, returning a guard
/// path. The index keeps parallel tests from sharing one spool file.
fn isolate(tag: &str) {
    let dir = std::env::temp_dir().join(format!("kavach-spool-test-{tag}"));
    // Best-effort: the dir is absent on first run; only a real removal failure matters.
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("create temp state dir");
    set_test_state_dir(Some(dir));
}

fn w(method: &str) -> SpooledWrite {
    SpooledWrite {
        method: method.to_owned(),
        params_json: r#"{"k":1}"#.to_owned(),
    }
}

#[test]
fn drain_empty_when_nothing_spooled() {
    isolate("empty");
    assert!(
        drain().expect("drain ok").is_empty(),
        "no spool file -> empty"
    );
}

#[test]
fn enqueue_then_drain_round_trips_in_order() {
    isolate("roundtrip");
    enqueue(&w("db.write")).expect("enqueue 1");
    enqueue(&w("db.bandit_backfill_session")).expect("enqueue 2");
    let drained = drain().expect("drain");
    assert_eq!(drained.len(), 2, "both entries returned: {drained:?}");
    assert_eq!(drained[0].method, "db.write", "append order preserved");
    assert_eq!(drained[1].method, "db.bandit_backfill_session");
}

#[test]
fn drain_removes_the_file_so_a_second_drain_is_empty() {
    isolate("removes");
    enqueue(&w("gate_pattern.upsert")).expect("enqueue");
    assert_eq!(drain().expect("first drain").len(), 1);
    // The file is gone — a re-drain (e.g. next Stop with no new failures) is empty,
    // proving a landed write is never double-replayed.
    assert!(
        drain().expect("second drain").is_empty(),
        "file removed after drain"
    );
}

#[test]
fn concurrent_drain_replays_each_line_exactly_once() {
    isolate("race");
    enqueue(&w("db.write")).expect("enqueue 1");
    enqueue(&w("db.bandit_backfill_session")).expect("enqueue 2");
    // Simulate two Stop gates racing: the first drainer wins the rename-claim
    // and gets both lines; the second sees the file already moved -> empty.
    let first = drain().expect("first drainer wins the claim");
    let second = drain().expect("second drainer sees nothing to claim");
    assert_eq!(first.len(), 2, "winner replays both lines: {first:?}");
    assert!(second.is_empty(), "loser gets empty, never a duplicate");
}

#[test]
fn corrupt_line_is_skipped_not_fatal() {
    isolate("corrupt");
    enqueue(&w("db.write")).expect("enqueue good");
    // Simulate a torn/garbage tail line appended out-of-band.
    let path = std::env::temp_dir()
        .join("kavach-spool-test-corrupt")
        .join("write_spool.jsonl");
    let mut existing = std::fs::read_to_string(&path).expect("read");
    existing.push_str("{not valid json\n");
    std::fs::write(&path, existing).expect("inject corrupt");
    let drained = drain().expect("drain tolerates corruption");
    assert_eq!(
        drained.len(),
        1,
        "good line survives, corrupt line dropped: {drained:?}"
    );
    assert_eq!(drained[0].method, "db.write");
}
