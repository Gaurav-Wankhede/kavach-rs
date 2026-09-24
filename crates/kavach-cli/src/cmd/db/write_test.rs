use super::{
    CATEGORY_HELP, STRICT_CATEGORIES, check_protected_closure, length_ratio_can_reach,
    lower_chars, mirror_depends_on_into_content, resolve_speculative_deps, similarity_lowered,
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

#[test]
fn protected_closure_blocks_true_positives() {
    // Canonical protected key + closure title is still blocked.
    assert!(check_protected_closure("auth.migration.p0-fix", "Closed as done").is_some());
    assert!(check_protected_closure("todo-12", "Closed.").is_some());
    assert!(check_protected_closure("market-gap-v2", "ARCHIVED").is_some());
    assert!(check_protected_closure("grpc-api", "deprecated endpoint").is_some());
}

#[test]
fn protected_closure_ignores_substring_false_positives() {
    // Token-boundary matching: prose containing pattern letters must not block.
    assert!(check_protected_closure("emphasis-notes", "Writeup disclosed").is_none());
    assert!(check_protected_closure("rapid-prototype", "Fix nonblocking IO").is_none());
    assert!(check_protected_closure("stalemate-review", "Update docs").is_none());
    // No closure word → no block even on a protected key.
    assert!(check_protected_closure("todo-12", "Add tests").is_none());
    // Unprotected key + closure title → no block.
    assert!(check_protected_closure("notes-misc", "Closed").is_none());
}

#[test]
fn similarity_lowered_scores_sane_values() {
    let score = |a: &str, b: &str| similarity_lowered(&lower_chars(a), &lower_chars(b));
    // Exact identities land exactly on the 0/1 rails (ordering compares —
    // float `==` is banned by `clippy::float_cmp`).
    assert!(score("hello world", "hello world") >= 1.0);
    assert!(score("Hello World", "hello world") >= 1.0);
    assert!(score("", "") >= 1.0);
    assert!(score("", "a") <= 0.0);
    assert!(score("auth guard", "auth guards") > 0.85);
    assert!(score("totally different title here", "nothing alike at all") < 0.5);
    // Symmetric: pair order lands in the same band either way
    // ("abc"/"abcd" = 0.75, "kitten"/"sitten" ≈ 0.83).
    for (a, b) in [("abc", "abcd"), ("kitten", "sitten")] {
        let (s1, s2) = (score(a, b), score(b, a));
        assert!(s1 > 0.7 && s1 < 0.9 && s2 > 0.7 && s2 < 0.9, "pair=({a:?}, {b:?})");
    }
}

#[test]
fn length_ratio_gate_never_skips_a_hit() {
    // Soundness: every pair that reaches threshold passes the gate (the gate
    // may pass pairs that miss — it must never skip pairs that hit).
    let titles = [
        "auth guard for session tokens",
        "auth guards for session token",
        "completely unrelated roadmap item",
        "todo",
        "todo!",
        "",
        "a much longer title with many words in it",
    ];
    for a in titles {
        for b in titles {
            let s = similarity_lowered(&lower_chars(a), &lower_chars(b));
            if s >= super::FUZZY_THRESHOLD {
                assert!(
                    length_ratio_can_reach(lower_chars(a).len(), lower_chars(b).len()),
                    "gate skipped a hit: ({a:?}, {b:?}) scored {s}"
                );
            }
        }
    }
}
