//! Repair proof: merge, never drop; idempotent; preserves the non-dep body.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{parse_all, repair_dep_lines};

#[test]
fn the_52_row_shape_collapses_to_one_line() {
    let body = "DEPENDS_ON: kavach-rs/roadmap/cc-2.1.198-notification-agent-events\n\
                DEPENDS_ON: cc-2.1.198-notification-agent-events\n\
                rest of the card";
    let (out, r) = repair_dep_lines("k", body);
    assert_eq!(
        out,
        "DEPENDS_ON: cc-2.1.198-notification-agent-events\nrest of the card",
        "both spellings collapse to one bare line"
    );
    assert_eq!(r.deps_before, 1, "the pair is ONE dependency, not two");
    assert!(r.changed);
}

#[test]
fn a_dep_present_only_on_the_qualified_line_is_not_lost() {
    // The real trap from `model-route-autoswitch-and-category`: the qualified
    // line carries TWO targets, the bare line only one. Dropping the qualified
    // line would silently destroy `model`.
    let body = "DEPENDS_ON: kavach-rs/roadmap/model, kavach-rs/roadmap/model-shift-router-advisory\n\
                DEPENDS_ON: model-shift-router-advisory\n\
                Extend the advisory.";
    let (out, r) = repair_dep_lines("k", body);
    let deps = parse_all(&out);
    assert_eq!(
        deps,
        vec!["model".to_owned(), "model-shift-router-advisory".to_owned()],
        "both deps survive; `model` is not silently dropped"
    );
    assert_eq!(r.deps_before, 2);
    assert!(out.contains("Extend the advisory."), "body preserved");
}

#[test]
fn repair_is_idempotent() {
    let body = "DEPENDS_ON: kavach-rs/roadmap/a\nDEPENDS_ON: b\nDEPENDS_ON: c\ntail";
    let (once, r1) = repair_dep_lines("k", body);
    let (twice, r2) = repair_dep_lines("k", &once);
    assert_eq!(once, twice, "second run is byte-identical");
    assert!(r1.changed);
    assert!(!r2.changed, "second run reports no change");
}

#[test]
fn a_clean_single_bare_line_is_left_alone() {
    let body = "DEPENDS_ON: a, b\ntail";
    let (out, r) = repair_dep_lines("k", body);
    assert_eq!(out, body);
    assert!(!r.changed);
}

#[test]
fn distinct_deps_across_lines_are_merged_not_dropped() {
    let body = "DEPENDS_ON: x\nDEPENDS_ON: y\nDEPENDS_ON: z\ntail";
    let (out, r) = repair_dep_lines("k", body);
    assert_eq!(parse_all(&out).len(), 3);
    assert_eq!(r.deps_before, 3);
    assert_eq!(out.lines().filter(|l| l.starts_with("DEPENDS_ON:")).count(), 1);
}

#[test]
fn a_body_with_no_deps_is_unchanged() {
    let body = "# Heading\n\nSome prose with no deps.";
    let (out, r) = repair_dep_lines("k", body);
    assert_eq!(out, body);
    assert!(!r.changed);
    assert_eq!(r.deps_before, 0);
}

#[test]
fn dep_line_order_is_first_seen() {
    let body = "DEPENDS_ON: zulu\nDEPENDS_ON: alpha\ntail";
    let (out, _) = repair_dep_lines("k", body);
    assert_eq!(
        parse_all(&out),
        vec!["zulu".to_owned(), "alpha".to_owned()],
        "existing order is preserved, not alphabetised"
    );
}

#[test]
fn trailing_whitespace_and_commas_are_tolerated() {
    let body = "DEPENDS_ON:  a , b ,  \nDEPENDS_ON: a\ntail";
    let (out, _) = repair_dep_lines("k", body);
    assert_eq!(parse_all(&out), vec!["a".to_owned(), "b".to_owned()]);
}