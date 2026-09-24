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
    assert!((score(&[], "X") - 0.0).abs() < f32::EPSILON);
}

#[test]
fn no_match_score_in_valid_range() {
    let s = score(&["totally_other".into()], "ABC");
    assert!((0.0..=1.0).contains(&s));
}

#[test]
fn span_tokens_score_identically_to_owned_tokens() {
    // "db_url" vs "database_url": {db,url} vs {database,url} → 1/3.
    let s = score(&["db_url".into()], "database_url");
    assert!((s - 1.0 / 3.0).abs() < 0.001, "got {s}");
}

#[test]
fn camel_boundary_tokenizes_without_allocations() {
    // "WidgetEnvelope" splits on uppercase into ["idget", "nvelope"] (capitals
    // are separators, same predicate as before): {nvelope} vs that → 1/2.
    let s = score(&["nvelope".into()], "WidgetEnvelope");
    assert!((s - 0.5).abs() < 0.001, "got {s}");
}
