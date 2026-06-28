//! Test that add_file_modified populates both lifetime and per-turn lists.
use crate::state::SessionState;

#[test]
fn add_file_modified_populates_lifetime_list() {
    let mut s = SessionState::default();
    assert!(s.add_file_modified("test.rs"));
    assert!(s.files_modified.contains(&"test.rs".to_owned()));
}

#[test]
fn add_file_modified_populates_this_turn_list() {
    let mut s = SessionState::default();
    assert!(s.add_file_modified("test.rs"));
    assert!(
        s.files_modified_this_turn.contains(&"test.rs".to_owned()),
        "add_file_modified must also populate files_modified_this_turn; got {:?}",
        s.files_modified_this_turn
    );
}
