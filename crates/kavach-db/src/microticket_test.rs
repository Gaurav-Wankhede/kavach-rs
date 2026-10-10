//! `MicroTicket v1` validation — fail-closed proof that every rule the module
//! docs claim is actually enforced.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{
    Change, MicroTicket, Modality, Op, Source, SourceKind, TicketKind, VerifyMethod, VerifyStep,
};

fn ok_ticket() -> MicroTicket {
    MicroTicket {
        kind: TicketKind::Bug,
        title: "Reject JSON tool calls with unknown enum values".to_owned(),
        goal: "A tool call carrying an unrecognized enum value fails validation instead of being coerced."
            .to_owned(),
        modality: Modality::Must,
        change: Change {
            op: Op::Edit,
            when: "a tool_call argument parses as a string and matches no enum member".to_owned(),
            then: "validation returns ErrUnknownEnum naming the field and the value".to_owned(),
        },
        scope: vec!["crates/kavach-db/src/microticket.rs".to_owned()],
        do_not_touch: vec!["crates/kavach-db/src/citation.rs".to_owned()],
        verify: vec![VerifyStep {
            method: VerifyMethod::Test,
            run: Some("cargo test -p kavach-db microticket".to_owned()),
            expect: "the test passes and asserts ErrUnknownEnum, not Ok(None)".to_owned(),
        }],
        source: Some(Source {
            kind: SourceKind::File,
            r#ref: "crates/kavach-db/src/microticket.rs:1".to_owned(),
            note: None,
        }),
        depends_on: vec![],
        conflicts_with: vec![],
        supersedes: vec![],
        manual_by: None,
    }
}

fn errs(t: &MicroTicket) -> Vec<String> {
    match t.validate() {
        Ok(()) => Vec::new(),
        Err(e) => e.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn a_complete_ticket_validates() {
    let t = ok_ticket();
    assert_eq!(t.validate(), Ok(()), "{:?}", errs(&t));
}

#[test]
fn json_roundtrip_is_stable() {
    let t = ok_ticket();
    let j = t.to_json().expect("serialize");
    assert_eq!(MicroTicket::from_json(&j).expect("parse"), t);
}

#[test]
fn unknown_field_is_rejected() {
    // Fail-closed on typos: a silently-ignored field is a silently-lost
    // constraint, which is exactly the under-specification we forbid.
    let j = r#"{"kind":"bug","title":"x","goal":"y","bogus":1}"#;
    assert!(MicroTicket::from_json(j).is_err(), "unknown field must not parse");
}

#[test]
fn caps_are_enforced_per_field() {
    let mut t = ok_ticket();
    t.title = "x".repeat(super::caps::TITLE + 1);
    assert!(errs(&t).iter().any(|e| e.starts_with("title")));

    let mut t = ok_ticket();
    t.goal = "x".repeat(super::caps::GOAL + 1);
    assert!(errs(&t).iter().any(|e| e.starts_with("goal")));
}

#[test]
fn goal_must_be_one_sentence() {
    let mut t = ok_ticket();
    t.goal = "It fixes the bug. It also ships the feature.".to_owned();
    let e = errs(&t);
    assert!(e.iter().any(|x| x.starts_with("goal")), "{e:?}");
    assert!(e.iter().any(|x| x.contains("ONE sentence")), "{e:?}");
}

#[test]
fn multi_line_title_is_rejected() {
    let mut t = ok_ticket();
    t.title = "First line\nSecond line".to_owned();
    assert!(errs(&t).iter().any(|e| e.starts_with("title")));
}

#[test]
fn verify_is_mandatory_and_non_empty() {
    let mut t = ok_ticket();
    t.verify.clear();
    assert!(errs(&t).iter().any(|e| e.starts_with("verify")));
}

#[test]
fn cmd_and_test_require_a_run() {
    let mut t = ok_ticket();
    t.verify[0].run = None;
    let e = errs(&t);
    assert!(e.iter().any(|x| x.starts_with("verify[0].run")), "{e:?}");
}

#[test]
fn manual_step_requires_manual_by() {
    let mut t = ok_ticket();
    t.verify[0] = VerifyStep {
        method: VerifyMethod::Manual,
        run: None,
        expect: "a human reads the rendered card".to_owned(),
    };
    assert!(errs(&t).iter().any(|e| e.starts_with("manual_by")));

    t.manual_by = Some("reviewer".to_owned());
    assert_eq!(t.validate(), Ok(()), "{:?}", errs(&t));
}

#[test]
fn scope_and_dnt_are_both_mandatory() {
    // CISOSE: the "do not touch" line is what stops scope creep — so an empty
    // one is a spec failure, not an omission.
    let mut t = ok_ticket();
    t.scope.clear();
    assert!(errs(&t).iter().any(|e| e.starts_with("scope")));

    let mut t = ok_ticket();
    t.do_not_touch.clear();
    assert!(errs(&t).iter().any(|e| e.starts_with("do_not_touch")));
}

#[test]
fn a_path_in_both_scope_and_dnt_is_a_contradiction() {
    let mut t = ok_ticket();
    let p = t.scope[0].clone();
    t.do_not_touch = vec![p];
    let e = errs(&t);
    assert!(e.iter().any(|x| x.contains("no-op")), "{e:?}");
}

#[test]
fn conflicting_and_dependent_key_is_unsatisfiable() {
    let mut t = ok_ticket();
    t.depends_on = vec!["mt.a.thing".to_owned()];
    t.conflicts_with = vec!["mt.a.thing".to_owned()];
    let e = errs(&t);
    assert!(e.iter().any(|x| x.contains("unsatisfiable")), "{e:?}");
}

#[test]
fn relationship_caps_and_duplicates_are_enforced() {
    let mut t = ok_ticket();
    t.depends_on = vec!["mt.a.one".to_owned(), "mt.a.one".to_owned()];
    assert!(errs(&t).iter().any(|e| e.contains("duplicate")));

    let mut t = ok_ticket();
    t.depends_on = (0..=super::caps::REL_MAX).map(|i| format!("mt.a.k{i}")).collect();
    assert!(errs(&t).iter().any(|e| e.starts_with("depends_on")));
}

#[test]
fn url_source_requires_a_url() {
    let mut t = ok_ticket();
    t.source = Some(Source {
        kind: SourceKind::Url,
        r#ref: "crates/kavach-db".to_owned(),
        note: None,
    });
    assert!(errs(&t).iter().any(|e| e.starts_with("source.ref")));
}

#[test]
fn all_errors_are_reported_not_just_the_first() {
    let mut t = ok_ticket();
    t.title = "x".repeat(super::caps::TITLE + 1);
    t.verify.clear();
    t.do_not_touch.clear();
    let e = errs(&t);
    assert!(e.len() >= 3, "author should fix everything at once: {e:?}");
}

#[test]
fn sentence_counter_accepts_normal_goals() {
    // A trailing period is the normal end of the sentence, not a second one.
    let mut t = ok_ticket();
    t.goal = "The DAG no longer renders phantom nodes for qualified deps.".to_owned();
    assert_eq!(t.validate(), Ok(()), "{:?}", errs(&t));
}