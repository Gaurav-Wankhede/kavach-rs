use super::filter_test_pending_for_project;
use crate::state::SessionState;

#[test]
fn filter_removes_other_project_files() {
    let mut s = SessionState::default();
    s.test_files_pending = vec![
        "/Users/g/Astro/astro-advisor/src/forecast.rs".into(),
        "/Users/g/Nicole/Backend/src/routes.rs".into(),
    ];
    s.test_nudge_count = 5;
    filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
    assert_eq!(s.test_files_pending.len(), 1);
    assert_eq!(
        s.test_files_pending[0],
        "/Users/g/Nicole/Backend/src/routes.rs"
    );
    assert_eq!(s.test_nudge_count, 5);
}

#[test]
fn filter_resets_nudge_when_all_cleared() {
    let mut s = SessionState::default();
    s.test_files_pending = vec!["/Users/g/Astro/src/forecast.rs".into()];
    s.test_nudge_count = 49;
    filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
    assert!(s.test_files_pending.is_empty());
    assert_eq!(s.test_nudge_count, 0);
}

#[test]
fn filter_keeps_all_when_same_project() {
    let mut s = SessionState::default();
    s.test_files_pending = vec![
        "/Users/g/Nicole/src/auth.rs".into(),
        "/Users/g/Nicole/src/pay.rs".into(),
    ];
    s.test_nudge_count = 3;
    filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
    assert_eq!(s.test_files_pending.len(), 2);
    assert_eq!(s.test_nudge_count, 3);
}

#[test]
fn filter_noop_on_empty_pending() {
    let mut s = SessionState::default();
    s.test_nudge_count = 2;
    filter_test_pending_for_project(&mut s, "/Users/g/Nicole");
    assert!(s.test_files_pending.is_empty());
    assert_eq!(s.test_nudge_count, 2);
}

#[test]
fn filter_noop_on_empty_work_dir() {
    let mut s = SessionState::default();
    s.test_files_pending = vec!["/Users/g/Astro/src/lib.rs".into()];
    s.test_nudge_count = 1;
    filter_test_pending_for_project(&mut s, "");
    assert_eq!(s.test_files_pending.len(), 1);
    assert_eq!(s.test_nudge_count, 1);
}
