use crate::state::SessionState;

#[test]
fn add_file_modified_records_into_this_turn_list() {
    // The TDD red-oracle reads `files_modified_this_turn` to map a touched test
    // file to its unit; if a Write never populates it, the oracle can never
    // record RED and blocks every test-first new-module write (root cause of
    // heal.incident.tdd-red-oracle-compile-fail-not-recorded).
    let mut s = SessionState::default();
    assert!(s.add_file_modified("crates/x/src/foo_test.rs"));
    assert!(
        s.files_modified_this_turn
            .iter()
            .any(|f| f == "crates/x/src/foo_test.rs"),
        "a Write must record into files_modified_this_turn, not only files_modified"
    );
}

#[test]
fn add_file_modified_dedups() {
    let mut s = SessionState::default();
    assert!(s.add_file_modified("a.rs"));
    assert!(!s.add_file_modified("a.rs"), "second add is a no-op");
    assert_eq!(
        s.files_modified_this_turn
            .iter()
            .filter(|f| *f == "a.rs")
            .count(),
        1,
        "no duplicate this-turn entry"
    );
}
