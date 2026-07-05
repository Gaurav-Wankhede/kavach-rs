use super::project_key_str;
use surrealdb_types::RecordId;

#[test]
fn string_key_returns_bare_slug_not_debug_wrapper() {
    let id = RecordId::new("project", "backend");
    let got = project_key_str(&id);
    assert_eq!(got, "backend", "must be the bare slug");
    assert!(!got.contains("String("), "must not leak the Debug wrapper");
}

#[test]
fn composite_id_has_no_double_nesting() {
    let id = RecordId::new("project", "nicole-carpenter");
    let composite = format!("{}:{}", project_key_str(&id), "fix.foo");
    assert_eq!(composite, "nicole-carpenter:fix.foo");
}
