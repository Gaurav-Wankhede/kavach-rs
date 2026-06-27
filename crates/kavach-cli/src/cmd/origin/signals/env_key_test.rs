use super::score;

#[test]
fn hint_found_case_insensitive() {
    assert_eq!(score(&["url".into()], "DATABASE_URL"), 1.0);
}

#[test]
fn hint_not_found() {
    assert_eq!(score(&["dsn".into()], "DATABASE_URL"), 0.0);
}

#[test]
fn empty_hints_returns_zero() {
    assert_eq!(score(&[], "X"), 0.0);
}
