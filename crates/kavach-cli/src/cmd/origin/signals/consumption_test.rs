use super::score;
use crate::cmd::origin::role_query::Candidate;
use crate::cmd::origin::site::Kind;

#[test]
fn consumed_by_symbol_in_value() {
    let cand = Candidate {
        name: "DATABASE_URL".into(),
        kind: Kind::LetBinding,
        file: "x".into(),
        line: 1,
        value: Some("PgPool::connect(url)".into()),
        is_secret: false,
    };
    assert!((score(&["PgPool".into()], &cand) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn consumed_by_not_found_with_none_value() {
    let cand = Candidate {
        name: "DATABASE_URL".into(),
        kind: Kind::LetBinding,
        file: "x".into(),
        line: 1,
        value: None,
        is_secret: false,
    };
    assert!((score(&["PgPool".into()], &cand) - 0.0).abs() < f32::EPSILON);
}

#[test]
fn empty_consumed_by_returns_zero() {
    let cand = Candidate {
        name: "DATABASE_URL".into(),
        kind: Kind::LetBinding,
        file: "x".into(),
        line: 1,
        value: Some("PgPool::connect(url)".into()),
        is_secret: false,
    };
    assert!((score(&[], &cand) - 0.0).abs() < f32::EPSILON);
}
