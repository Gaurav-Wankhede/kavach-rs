//! Repair proof: merge, never drop; idempotent; preserves the non-dep body.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use super::{is_prose_token, parse_all, repair_dep_lines, strip_prose_deps};

#[test]
fn the_nlu_harvested_words_are_prose() {
    for w in ["their", "dedup", "rename", "read-back-tokens"] {
        if w.contains('-') {
            continue;
        }
        assert!(is_prose_token(w), "{w} must be recognised as prose");
    }
    // A real key shape is never prose, even when absent from the DB.
    for k in [
        "g1a-fts-index",
        "v0.3.cli",
        "v0_3_cli",
        "roadmap.unit.harness.reward-ledger",
        "g1a",
        "p9",
    ] {
        assert!(!is_prose_token(k), "{k} must NOT be treated as prose");
    }
}

#[test]
fn strip_prose_deps_removes_only_prose() {
    // Real-world shape from `model-route-autoswitch-and-category`:
    //   DEPENDS_ON: model, model-shift-router-advisory
    // `model` is the card's own first word lifted out of its title ("Model-route
    // Grade-B") — flat lowercase prose, no such card exists. The second token is
    // a real (verified decision) key and MUST survive.
    let body = "DEPENDS_ON: model, model-shift-router-advisory\n\
                DEPENDS_ON: their\n\
                rest";
    let (out, removed) = strip_prose_deps(body);
    let mut sorted = removed.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["model".to_owned(), "their".to_owned()],
        "both flat-lowercase tokens are prose"
    );
    assert!(
        out.starts_with("DEPENDS_ON: model-shift-router-advisory"),
        "the real key survives: {out}"
    );
    assert!(!out.contains("their"), "prose is gone");
    assert!(!out.contains(", model,"), "the prose word is gone: {out}");
    assert!(out.contains("rest"), "body preserved");
}

/// A dep that is absent-but-key-shaped is KEPT — it is a real typo to fix, not
/// garbage to delete. Deleting it would silently drop a live edge.
#[test]
fn absent_but_key_shaped_deps_are_kept() {
    let body = "DEPENDS_ON: roadmap.unit.infra.kavach-harness.reward-ledger\nrest";
    let (out, removed) = strip_prose_deps(body);
    assert!(removed.is_empty(), "nothing removed: {removed:?}");
    assert_eq!(out, body, "byte-identical");
}

/// A card whose deps are ALL prose ends up with no dependency line at all.
#[test]
fn all_prose_deps_leave_no_dep_line() {
    let body = "DEPENDS_ON: their\nDEPENDS_ON: rename\nbody text";
    let (out, removed) = strip_prose_deps(body);
    assert_eq!(removed.len(), 2);
    assert!(!out.contains("DEPENDS_ON"), "no dep line remains: {out}");
    assert!(out.contains("body text"));
}

/// `g1a` is a shorthand a human wrote for `...g1a-fts-index` — key-shaped, so it
/// is preserved as an (unresolved) reference rather than deleted. Resolving it is
/// a judgement call for the operator, not an automatic deletion.
#[test]
fn human_shorthand_is_preserved_not_deleted() {
    let body = "DEPENDS_ON: g1a, roadmap.unit.harness.brain-os.g1a-fts-index\nrest";
    let (out, removed) = strip_prose_deps(body);
    assert!(removed.is_empty());
    assert!(out.contains("g1a"), "shorthand kept: {out}");
}

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

/// A dep-free body ending in a newline must not be rewritten just to drop it —
/// otherwise a dry run lists every clean row as "changed" and `--apply` churns
/// hundreds of untouched cards.
#[test]
fn a_dep_free_body_keeps_its_trailing_newline() {
    let body = "# Heading\n\nprose\n";
    let (out, r) = repair_dep_lines("k", body);
    assert_eq!(out, body, "byte-identical");
    assert!(!r.changed);
}

/// The same applies to a repaired body: the author's trailing newline survives.
#[test]
fn repair_preserves_a_trailing_newline() {
    let body = "DEPENDS_ON: kavach-rs/roadmap/a\nDEPENDS_ON: a\ntail\n";
    let (out, _) = repair_dep_lines("k", body);
    assert!(out.ends_with('\n'), "trailing newline preserved: {out:?}");
    assert!(out.starts_with("DEPENDS_ON: a\ntail"), "got: {out:?}");
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