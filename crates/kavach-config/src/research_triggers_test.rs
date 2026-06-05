//! Agreement proof: the canonical bug/fix floor is honored by
//! `requires_research` REGARDLESS of the user config's `research_triggers`.
//! This is the regression that pins the `TABULA_RASA` non-determinism shut — a
//! bug/fix prompt now gates research on the config path (A) exactly as the
//! `research_guard` path (B) and the intent-tree path (C) do, because all three
//! consult this one list.

use super::{BUG_FIX_TRIGGERS, has_bug_fix_trigger};

#[test]
fn every_canonical_trigger_is_detected() {
    for tok in BUG_FIX_TRIGGERS {
        assert!(
            has_bug_fix_trigger(&format!("please {tok} this for me")),
            "canonical trigger {tok:?} must fire has_bug_fix_trigger"
        );
    }
}

#[test]
fn config_path_honors_bug_floor_even_without_matching_config_trigger() {
    // "fix the login bug" is NOT in the live config's research_triggers
    // (implement/create/build/add/integrate/setup/configure) — yet the canonical
    // floor must still force research. Pre-fix this returned false on path A
    // while path B returned true: the exact disagreement we eliminated.
    assert!(
        crate::blocklist::requires_research("fix the login bug"),
        "bug/fix intent must require research on the config-driven path"
    );
    assert!(
        crate::blocklist::requires_research("debug the crash in the parser"),
        "debug/crash intent must require research on the config-driven path"
    );
}

#[test]
fn non_bug_non_trigger_prompt_does_not_force_research() {
    // A plain prose prompt with no bug token and no config trigger stays false —
    // the floor raises bug work, it does not blanket-require everything.
    assert!(
        !has_bug_fix_trigger("write a haiku about the ocean"),
        "non-bug prose must not trip the bug/fix floor"
    );
}
