use super::{CATEGORY_HELP, STRICT_CATEGORIES};

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
