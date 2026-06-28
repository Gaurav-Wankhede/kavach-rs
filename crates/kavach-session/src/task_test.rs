use crate::state::SessionState;

#[test]
fn test_add_file_modified_populates_this_turn() {
    let mut s = SessionState::default();
    assert!(s.add_file_modified("test.rs"));
    assert!(
        s.files_modified_this_turn.contains(&"test.rs".to_owned()),
        "add_file_modified must populate files_modified_this_turn; got {:?}",
        s.files_modified_this_turn
    );
}
