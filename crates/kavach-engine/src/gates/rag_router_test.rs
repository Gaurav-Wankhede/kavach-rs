//! RAG-router degradation tests: a missing label must yield empty context.
use super::advisory::advisory_context;

#[test]
fn should_return_empty_when_label_missing() {
    // No row with this label in any live db — function must degrade gracefully
    // to empty string rather than panic or return context.
    let ctx = advisory_context(
        "kavach-rag-test-nonexistent-label-xyz-42",
        "src/lib.rs",
        "fix unwrap",
        "implement",
        3,
    );
    assert!(ctx.is_empty());
}
