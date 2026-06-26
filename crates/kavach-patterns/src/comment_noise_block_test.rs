//! Proofs for the changed-content-aware comment-bloat BLOCK. A write is denied
//! only when it INTRODUCES new bloat; pre-existing bloat stays editable.

use super::introduces_bloat;

fn bloat_block() -> String {
    let prose = "x".repeat(70);
    format!("// {prose}\n// b\n// c\n// d\n// e\n// f\n")
}

#[test]
fn new_bloat_in_clean_file_is_blocked() {
    let old = "fn f() {}\n";
    let new = format!("fn f() {{}}\n{}", bloat_block());
    assert!(
        introduces_bloat("src/x.rs", old, &new),
        "added a bloat block -> block"
    );
}

#[test]
fn write_creating_a_new_file_with_bloat_is_blocked() {
    // No prior content (fresh Write): any bloat in the new content is introduced.
    let new = format!("fn f() {{}}\n{}", bloat_block());
    assert!(introduces_bloat("src/x.rs", "", &new));
}

#[test]
fn editing_a_file_that_already_had_bloat_is_allowed() {
    // Pre-existing bloat (same count before+after) must NOT wedge the edit.
    let old = format!("fn f() {{}}\n{}", bloat_block());
    let new = format!("fn f() {{}}\n{}fn g() {{}}\n", bloat_block());
    assert!(
        !introduces_bloat("src/x.rs", &old, &new),
        "unchanged bloat count -> allow"
    );
}

#[test]
fn removing_bloat_is_allowed() {
    let old = format!("fn f() {{}}\n{}", bloat_block());
    let new = "fn f() {}\n";
    assert!(
        !introduces_bloat("src/x.rs", &old, new),
        "reducing bloat is never blocked"
    );
}

#[test]
fn clean_edit_on_clean_file_is_allowed() {
    assert!(!introduces_bloat(
        "src/x.rs",
        "fn f() {}\n",
        "fn f() {}\nfn g() {}\n"
    ));
}

#[test]
fn non_source_file_never_blocks() {
    let new = format!("text\n{}", bloat_block());
    assert!(!introduces_bloat("notes.md", "", &new));
}
