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
