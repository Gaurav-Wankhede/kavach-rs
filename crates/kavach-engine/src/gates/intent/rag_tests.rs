//! RAG-collapse edge cases: empty + single-entry lists pass through unchanged.
//! Multi-entry collapse depends on a live persisted tree and is covered by the
//! end-to-end smoke path.
use super::classify::collapse_required_via_rag;

#[test]
fn should_leave_empty_list_unchanged() {
    let out = collapse_required_via_rag(Vec::new(), "", "");
    assert!(out.is_empty());
}

#[test]
fn should_leave_single_entry_list_unchanged() {
    let single = vec!["rust".into()];
    let out = collapse_required_via_rag(single.clone(), "irrelevant", "general");
    assert_eq!(out, single);
}
