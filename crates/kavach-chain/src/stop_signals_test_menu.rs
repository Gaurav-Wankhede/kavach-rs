//! Continuation-menu detector tests — split from `stop_signals_test.rs` to keep
//! each test file under the nano-file ceiling. Included as a submodule of the
//! parent `tests` module via `#[path]`, so `super::super::*` reaches the crate.
use super::super::*;

#[test]
fn continuation_menu_blocks_choice_offering() {
    for m in [
        "Say \"continue to W4\" and I'll proceed, or redirect me to the Jacobs Ladder Marketing port.",
        "Want me to continue with the migration or switch to the API work?",
        "I can continue here, or pivot to the other thread — let me know which to proceed.",
        "say go and i'll continue, or redirect me to the other task",
        // EXACT verbatim transcript the user reported (2026-06-17): the
        // continue-or-PAUSE form, with `[AUTO_CONTINUE] runnable=0` from the gate.
        // "pause here" is NOT in the or-target alternation, but the `(?:want|...)
        // me to (?:continue|proceed|keep going) ... \bor\b` arm fires on the
        // "Want me to continue ... or" prefix regardless of the post-`or` clause.
        "Want me to continue to a new card, or pause here?",
    ] {
        assert!(detect_continuation_menu(m).unwrap(), "missed: {m}");
    }
}

#[test]
fn continuation_menu_catches_announce_then_offer_out() {
    for m in [
        "the classified index list pages can bind use_resource against the verified contract. That's the next card unless you'd like to redirect.",
        "That's the next card unless you'd like to redirect.",
        "X is the next card unless you want me to redirect.",
        "The index pages are next up — want me to proceed, or redirect me?",
        "Next step is the directory port; let me know if you'd like to switch.",
        "unless you'd like me to pivot, the next task is the Jacobs Ladder Marketing client",
    ] {
        assert!(detect_continuation_menu(m).unwrap(), "missed: {m}");
    }
}

#[test]
fn continuation_menu_allows_announce_then_execute() {
    assert!(
        !detect_continuation_menu(
            "Card closed. Next up: the index pages — claiming it now and binding use_resource."
        )
        .unwrap()
    );
    assert!(
        !detect_continuation_menu("The next card is the directory port. Starting it.").unwrap()
    );
}

#[test]
fn continuation_menu_suppressed_for_legit_stop_and_ask() {
    assert!(
        !detect_continuation_menu(
            "you asked me to choose, so: continue to W4 or switch to Jacobs Ladder Marketing?"
        )
        .unwrap()
    );
    assert!(
        !detect_continuation_menu(
            "this is genuinely ambiguous and changes the outcome — continue or switch?"
        )
        .unwrap()
    );
    assert!(
        !detect_continuation_menu(
            "I need a credential to proceed; want me to continue or switch once you provide it?"
        )
        .unwrap()
    );
    assert!(
        !detect_continuation_menu("the continuation_menu detector blocks choice-offering").unwrap()
    );
}
