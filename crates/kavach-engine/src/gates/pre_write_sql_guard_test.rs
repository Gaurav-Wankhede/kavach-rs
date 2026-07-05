use super::*;

#[test]
fn should_allow_parameterized_query() {
    assert!(check("query.sql", "SELECT id FROM users WHERE id = $1").is_none());
}
