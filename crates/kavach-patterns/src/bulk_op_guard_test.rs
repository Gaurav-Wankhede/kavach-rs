//! Red-Green proofs for the bulk-operation→single-script steer. Outcomes drive code.

use super::{BulkOpVocab, detect_bulk_op};

#[test]
fn fires_on_rnr_batch_rename() {
    // rnr IS a batch renamer — its very presence is a bulk op that belongs in a script.
    let msg = "rnr -r 'oldName' 'newName' src/";
    assert!(detect_bulk_op(msg).is_some(), "rnr batch rename must steer to a script");
}

#[test]
fn fires_on_fd_exec_sd() {
    // The canonical inline bulk rewrite: fd finds N files, -x runs sd on each.
    let msg = "fd -e rs . src -x sd 'OldType' 'NewType' {}";
    assert!(detect_bulk_op(msg).is_some(), "fd -x sd fan-out must steer to a script");
}

#[test]
fn fires_on_sd_with_glob_multiple_targets() {
    let msg = "sd -i 'foo' 'bar' src/a.rs src/b.rs src/c.rs";
    assert!(detect_bulk_op(msg).is_some(), "sd across many files must steer to a script");
}

#[test]
fn fires_on_xargs_sed_pipeline() {
    let msg = "rg -l 'oldImport' | xargs sd 'oldImport' 'newImport'";
    assert!(detect_bulk_op(msg).is_some(), "rg -l | xargs rewrite must steer to a script");
}

#[test]
fn does_not_fire_on_single_file_inline_edit() {
    // ONE file, one rewrite — the write-bypass guard owns this; bulk steer must stay quiet.
    let msg = "sd 'foo' 'bar' src/only.rs";
    assert!(detect_bulk_op(msg).is_none(), "a single-file edit is not a bulk op");
}

#[test]
fn does_not_fire_when_already_a_committed_script() {
    // The sanctioned path: the bulk op IS one script. Running it must never re-steer.
    let msg = "bash scripts/rename_thread_id.sh";
    assert!(detect_bulk_op(msg).is_none(), "running a committed script is the goal, not a violation");
    let sh = "sh scripts/rewrite_imports.sh";
    assert!(detect_bulk_op(sh).is_none());
    let direct = "./scripts/migrate.sh";
    assert!(detect_bulk_op(direct).is_none());
}

#[test]
fn does_not_fire_on_plain_read_fan_out() {
    // fd -x bat / rg over many files is a READ fan-out, not a mutation — never steer.
    let msg = "fd -e rs . src -x bat {}";
    assert!(detect_bulk_op(msg).is_none(), "read fan-out is not a bulk mutation");
}

#[test]
fn default_vocab_matches_floor_detector() {
    use super::detect_bulk_op_with;
    let v = BulkOpVocab::default();
    let msg = "fd -e rs . src -x sd 'A' 'B' {}";
    assert_eq!(detect_bulk_op_with(&v, msg).is_some(), detect_bulk_op(msg).is_some());
}

#[test]
fn graph_overlay_adds_mutator_floor_still_active() {
    use super::detect_bulk_op_with;
    // ADDITIVE: a project registers a new bulk mutator; the floor (rnr/fd -x sd) still fires.
    let mut v = BulkOpVocab::default();
    v.mutators.push("ast-grep".to_owned());
    let added = "ast-grep -p 'old' -r 'new' src/";
    assert!(detect_bulk_op_with(&v, added).is_some(), "added mutator fires");
    let floor = "rnr -r 'a' 'b' src/";
    assert!(detect_bulk_op_with(&v, floor).is_some(), "floor mutator still fires");
    // script carve still clears it
    assert!(detect_bulk_op_with(&v, "bash scripts/x.sh").is_none(), "script carve intact");
}

#[test]
fn malformed_overlay_degrades_to_floor() {
    let v: BulkOpVocab = serde_json::from_str("{}").expect("empty obj is valid");
    assert!(!v.mutators.is_empty() && !v.fanout_markers.is_empty());
}
