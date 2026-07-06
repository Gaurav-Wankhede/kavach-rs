//! Pure `resume_block` decision tests: the verified/done->None boundary, the
//! empty-harness suffix omission, and the empty-key->None boundary.
use super::resume_block;

#[test]
fn empty_key_yields_none() {
    assert_eq!(resume_block("", "Title", "in_progress", ""), None);
}

#[test]
fn verified_status_yields_none() {
    assert_eq!(resume_block("card-1", "Title", "verified", "tdd"), None);
}

#[test]
fn done_status_yields_none() {
    assert_eq!(resume_block("card-1", "Title", "done", "tdd"), None);
}

#[test]
fn in_progress_yields_block_with_harness_suffix() {
    let block = resume_block("card-1", "Title", "in_progress", "tdd").unwrap();
    assert!(block.starts_with("[RESUME]"));
    assert!(block.contains("card-1 — Title [in_progress]"));
    assert!(block.contains("harness=tdd"));
}

#[test]
fn empty_harness_omits_suffix() {
    let block = resume_block("card-1", "Title", "in_progress", "").unwrap();
    assert!(block.contains("card-1 — Title [in_progress]"));
    assert!(!block.contains("harness="));
}
