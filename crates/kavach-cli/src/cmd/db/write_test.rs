use super::{
    CATEGORY_HELP, STRICT_CATEGORIES, mirror_depends_on_into_content, resolve_speculative_deps,
};
use kavach_engine::ExtractedRelationship;

fn rel(rel: &str, target: &str, speculative: bool) -> ExtractedRelationship {
    let mut r = ExtractedRelationship::new(rel, target);
    r.speculative = speculative;
    r
}

/// A speculative (NLU-origin) target absent from `known` is dropped.
#[test]
fn unresolvable_speculative_dep_is_dropped() {
    let rels = vec![rel("depends_on", "row.body_blake3", true)];
    let (kept, dropped) = resolve_speculative_deps(rels, &["real-key".to_owned()]);
    assert!(kept.is_empty());
    assert_eq!(dropped, vec!["row.body_blake3".to_owned()]);
}

/// A speculative target that DOES resolve against a known key passes through.
#[test]
fn resolvable_speculative_dep_passes_through() {
    let rels = vec![rel("depends_on", "real-key", true)];
    let (kept, dropped) = resolve_speculative_deps(rels, &["real-key".to_owned()]);
    assert_eq!(kept.len(), 1);
    assert!(dropped.is_empty());
}

/// Non-speculative (explicit/frontmatter/wikilink) edges pass through UNTOUCHED
/// even when their target resolves against no known row — author intent wins.
#[test]
fn non_speculative_dep_passes_through_when_unresolvable() {
    let rels = vec![rel("depends_on", "ghost-target", false)];
    let (kept, dropped) = resolve_speculative_deps(rels, &[]);
    assert_eq!(kept.len(), 1);
    assert!(dropped.is_empty());
}

/// A qualified target (`slug/cat/key`) resolves against the bare tail of a known key.
#[test]
fn qualified_target_resolves_against_bare_known_key() {
    let rels = vec![rel("depends_on", "proj/roadmap/real-key", true)];
    let (kept, dropped) = resolve_speculative_deps(rels, &["real-key".to_owned()]);
    assert_eq!(kept.len(), 1);
    assert!(dropped.is_empty());
}

/// No flag deps → body is returned unchanged (no spurious `DEPENDS_ON` line).
#[test]
fn no_flag_deps_leaves_body_unchanged() {
    let body = "Some plan body.".to_owned();
    assert_eq!(mirror_depends_on_into_content(body.clone(), &[]), body);
}

/// A flag dep is mirrored into a `DEPENDS_ON` content line the readiness parser reads.
#[test]
fn flag_dep_is_mirrored_into_content() {
    let out = mirror_depends_on_into_content("body".to_owned(), &["roadmap.unit.x".to_owned()]);
    assert!(out.starts_with("DEPENDS_ON: roadmap.unit.x"));
    assert!(out.contains("body"));
}

/// Idempotent: a target already on a `DEPENDS_ON` content line is NOT re-added.
#[test]
fn already_declared_dep_is_not_duplicated() {
    let body = "DEPENDS_ON: roadmap.unit.x\nrest".to_owned();
    let out = mirror_depends_on_into_content(body.clone(), &["roadmap.unit.x".to_owned()]);
    assert_eq!(out, body, "must not duplicate an already-declared dep");
}

/// An empty body + a flag dep yields exactly the `DEPENDS_ON` line (no leading newline).
#[test]
fn empty_body_yields_bare_dep_line() {
    let out = mirror_depends_on_into_content(String::new(), &["roadmap.unit.x".to_owned()]);
    assert_eq!(out, "DEPENDS_ON: roadmap.unit.x");
}

/// Blank/whitespace flag targets are dropped (no empty `DEPENDS_ON` entry).
#[test]
fn blank_flag_targets_are_dropped() {
    let out = mirror_depends_on_into_content("body".to_owned(), &["  ".to_owned()]);
    assert_eq!(out, "body");
}

/// `SSoT` guard (rca.kavach-db-write-category-enum-inconsistent): the clap
/// `--category` help text MUST enumerate exactly `STRICT_CATEGORIES`.
/// This fails closed the instant the two desync — making drift a
/// compile-test failure instead of a silent mislead (the root cause:
/// help was a hand-duplicated literal, not derived from the authority).
#[test]
fn category_help_matches_strict_categories() {
    // Reconstruct the canonical phrasing from the single source.
    let expected = format!("Category ({})", STRICT_CATEGORIES.join(", "));
    assert_eq!(
        CATEGORY_HELP, expected,
        "CATEGORY_HELP desynced from STRICT_CATEGORIES — update BOTH (the \
         enum has ONE source of truth). Add/remove a category in \
         STRICT_CATEGORIES and mirror it in CATEGORY_HELP."
    );
    // And the dead category must never reappear in either.
    assert!(
        !CATEGORY_HELP.contains("proposal") && !STRICT_CATEGORIES.contains(&"proposal"),
        "`proposal` is a dead category accepted by zero validators — it \
         must not return to the help text or the validator set"
    );
}
