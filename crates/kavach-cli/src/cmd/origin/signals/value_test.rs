use super::score;

#[test]
fn regex_matches_value() {
    assert!((score(Some("^postgres://"), Some("postgres://x")) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn regex_does_not_match_value() {
    assert!((score(Some("^postgres://"), Some("redis://x")) - 0.0).abs() < f32::EPSILON);
}

#[test]
fn missing_value_returns_zero() {
    assert!((score(Some("^postgres://"), None) - 0.0).abs() < f32::EPSILON);
}

#[test]
fn missing_regex_returns_zero() {
    assert!((score(None, Some("x")) - 0.0).abs() < f32::EPSILON);
}

#[test]
fn invalid_regex_returns_zero() {
    assert_eq!(score(Some("("), Some("x")), 0.0);
}
