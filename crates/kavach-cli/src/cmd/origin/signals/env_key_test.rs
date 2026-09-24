use super::score;

#[test]
fn hint_found_case_insensitive() {
    assert!((score(&["url".into()], "DATABASE_URL") - 1.0).abs() < f32::EPSILON);
}

#[test]
fn hint_not_found() {
    assert!((score(&["dsn".into()], "DATABASE_URL") - 0.0).abs() < f32::EPSILON);
}

#[test]
fn empty_hints_returns_zero() {
    assert!((score(&[], "X") - 0.0).abs() < f32::EPSILON);
}

#[test]
fn hint_mid_word_is_not_a_hit() {
    // "api" must not fire inside "rapid" (old substring matched).
    assert!((score(&["api".into()], "rapid") - 0.0).abs() < f32::EPSILON);
}

#[test]
fn hint_prefix_of_token_is_a_hit() {
    assert!((score(&["data".into()], "DATABASE_URL") - 1.0).abs() < f32::EPSILON);
}
