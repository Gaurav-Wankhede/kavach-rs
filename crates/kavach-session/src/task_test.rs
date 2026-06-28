use crate::state::SessionState;

#[test]
fn test_add_file_modified() {
    let mut s = SessionState::default();
    assert!(s.add_file_modified("a.rs"));
    assert!(!s.add_file_modified("a.rs"));
    assert!(s.add_file_modified("b.rs"));
    assert_eq!(s.files_modified.len(), 2);
}

#[test]
fn test_has_task() {
    let mut s = SessionState::default();
    assert!(!s.has_task());
    s.current_task = "test".into();
    assert!(s.has_task());
}

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
