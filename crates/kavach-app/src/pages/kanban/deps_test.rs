//! Tests for `declared_deps` — the inline dependency-key parser the kanban tier
//! layout uses. Pure dependency ordering; there is no blocked/gate concept.
use super::declared_deps;

#[test]
fn no_dep_line_yields_empty() {
    assert!(declared_deps("just a plain card body, no prerequisites").is_empty());
}

#[test]
fn depends_on_line_is_parsed() {
    assert_eq!(declared_deps("DEPENDS_ON: a, b c"), vec!["a", "b", "c"]);
}

#[test]
fn blocked_by_is_a_dependency_alias() {
    // BLOCKED_BY: is accepted as a back-compat alias for DEPENDS_ON: — it
    // declares a prerequisite key, NOT a gate; the word carries no special state.
    assert_eq!(declared_deps("BLOCKED_BY: prereq.one"), vec!["prereq.one"]);
}

#[test]
fn bullet_continuation_after_a_dep_header_is_collected() {
    let body = "DEPENDS_ON:\n- first.key extra words\n- second.key\n\nunrelated tail";
    assert_eq!(declared_deps(body), vec!["first.key", "second.key"]);
}
