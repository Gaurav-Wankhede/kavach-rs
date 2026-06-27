use super::score;

#[test]
fn exact_match_case_insensitive() {
    let s = score(&["DATABASE_URL".into()], "DATABASE_URL");
    assert!((s - 1.0).abs() < f32::EPSILON);
}

#[test]
fn partial_jaccard_score() {
    let s = score(&["db_url".into()], "database_url");
    assert!((0.0..=1.0).contains(&s));
}

#[test]
fn empty_aliases_returns_zero() {
    assert_eq!(score(&[], "X"), 0.0);
}

#[test]
fn no_match_score_in_valid_range() {
    let s = score(&["totally_other".into()], "ABC");
    assert!((0.0..=1.0).contains(&s));
}
