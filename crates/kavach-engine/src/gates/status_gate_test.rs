use super::*;

#[test]
fn non_roadmap_category_is_not_gated() {
    assert_eq!(
        verify_status_promotion("decision", "done", "", None),
        StatusGateVerdict::NotGated
    );
}

#[test]
fn todo_status_is_not_gated() {
    assert_eq!(
        verify_status_promotion("roadmap", "todo", "", None),
        StatusGateVerdict::NotGated
    );
}

#[test]
fn in_progress_status_is_not_gated() {
    assert_eq!(
        verify_status_promotion("roadmap", "in_progress", "", None),
        StatusGateVerdict::NotGated
    );
}

#[test]
fn done_is_a_completion_status() {
    assert!(is_completion_status("done"));
}

#[test]
fn verified_is_a_completion_status() {
    assert!(is_completion_status("verified"));
}

#[test]
fn todo_is_not_a_completion_status() {
    assert!(!is_completion_status("todo"));
}
