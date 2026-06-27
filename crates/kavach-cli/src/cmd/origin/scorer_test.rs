use super::{rank, THRESHOLD};
use crate::cmd::origin::role_query::{Candidate, RoleQuery};
use crate::cmd::origin::site::Kind;

#[test]
fn candidate_matching_query_scores_above_threshold() {
    let q = RoleQuery {
        role: "db".into(),
        value_regex: Some("^postgres://".into()),
        consumed_by: vec![],
        env_key_hints: vec!["url".into()],
        name_aliases: vec!["DATABASE_URL".into()],
    };
    let cand = Candidate {
        name: "DATABASE_URL".into(),
        kind: Kind::EnvVar,
        file: "c.rs".into(),
        line: 1,
        value: Some("postgres://h/db".into()),
        is_secret: false,
    };
    let ranked = rank(&q, vec![cand]);
    assert_eq!(ranked.len(), 1);
    assert!(ranked[0].score >= THRESHOLD);
}

#[test]
fn default_query_returns_empty() {
    let q = RoleQuery::default();
    let cand = Candidate {
        name: "DATABASE_URL".into(),
        kind: Kind::EnvVar,
        file: "c.rs".into(),
        line: 1,
        value: Some("postgres://h/db".into()),
        is_secret: false,
    };
    let ranked = rank(&q, vec![cand]);
    assert!(ranked.is_empty());
}
