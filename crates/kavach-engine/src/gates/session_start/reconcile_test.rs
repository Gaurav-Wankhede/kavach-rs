//! E7 reconcile-predicate tests (card VERIFY clause): prove
//! dirty+in_progress+no-status-cmd => `ResumeVerify`, and every other
//! combination => `ReDispatch`. Pure — no git spawn, no RPC.
use super::{ReconcileAction, reconcile_action, touched_paths_from_card};

fn paths(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn seam_case_resumes_verify() {
    // in_progress + no status cmd + dirty file overlaps the card's TOUCHES path.
    let porcelain = " M crates/kavach-engine/src/gates/session_start/reconcile.rs\n";
    let card = paths(&["reconcile.rs"]);
    assert_eq!(
        reconcile_action(true, false, porcelain, &card),
        ReconcileAction::ResumeVerify,
    );
}

#[test]
fn status_cmd_already_recorded_redispatches() {
    // The transition WAS recorded → no seam, even with a matching dirty file.
    let porcelain = " M reconcile.rs\n";
    assert_eq!(
        reconcile_action(true, true, porcelain, &paths(&["reconcile.rs"])),
        ReconcileAction::ReDispatch,
    );
}

#[test]
fn card_not_in_progress_redispatches() {
    let porcelain = " M reconcile.rs\n";
    assert_eq!(
        reconcile_action(false, false, porcelain, &paths(&["reconcile.rs"])),
        ReconcileAction::ReDispatch,
    );
}

#[test]
fn clean_tree_redispatches() {
    assert_eq!(
        reconcile_action(true, false, "", &paths(&["reconcile.rs"])),
        ReconcileAction::ReDispatch,
    );
}

#[test]
fn no_overlap_redispatches() {
    // Dirty tree exists but touches a DIFFERENT file than the card declares.
    let porcelain = " M crates/other/src/unrelated.rs\n";
    assert_eq!(
        reconcile_action(true, false, porcelain, &paths(&["reconcile.rs"])),
        ReconcileAction::ReDispatch,
    );
}

#[test]
fn empty_card_paths_never_false_resume() {
    // No TOUCHES: hint → cannot prove overlap → ReDispatch (fail-safe).
    let porcelain = " M reconcile.rs\n";
    assert_eq!(
        reconcile_action(true, false, porcelain, &[]),
        ReconcileAction::ReDispatch,
    );
}

#[test]
fn rename_destination_path_overlaps() {
    // A renamed file's DESTINATION basename is the live edit; it must overlap.
    let porcelain = "R  src/old.rs -> crates/x/src/reconcile.rs\n";
    assert_eq!(
        reconcile_action(true, false, porcelain, &paths(&["reconcile.rs"])),
        ReconcileAction::ResumeVerify,
    );
}

#[test]
fn untracked_file_overlaps() {
    // `??` (untracked) lines also count as dirty work belonging to the card.
    let porcelain = "?? crates/x/src/reconcile.rs\n";
    assert_eq!(
        reconcile_action(true, false, porcelain, &paths(&["reconcile.rs"])),
        ReconcileAction::ResumeVerify,
    );
}

#[test]
fn touches_absent_yields_empty() {
    assert!(touched_paths_from_card("title\nDEPENDS_ON: x\nbody\n").is_empty());
}

#[test]
fn touches_whitespace_and_comma_separated() {
    let card = "title\nTOUCHES: a/b.rs  c.rs,  d/e.rs\nmore\n";
    assert_eq!(
        touched_paths_from_card(card),
        paths(&["a/b.rs", "c.rs", "d/e.rs"]),
    );
}

#[test]
fn touches_first_line_wins() {
    let card = "TOUCHES: first.rs\nTOUCHES: second.rs\n";
    assert_eq!(touched_paths_from_card(card), paths(&["first.rs"]));
}

#[test]
fn touches_empty_value_yields_empty() {
    assert!(touched_paths_from_card("TOUCHES:   \n").is_empty());
}
