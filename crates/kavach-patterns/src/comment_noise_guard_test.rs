// SOURCE: decision.gate.research-first-vs-comment-bloat-newfile-deadlock-2026-06-30

use super::advise;

#[test]
fn short_precise_block_is_clean() {
    let c = "fn f() {}\n// one\n// two\n// three\nfn g() {}\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn long_rationale_paragraph_flagged() {
    let prose = "x".repeat(70);
    let c = format!("fn f() {{}}\n// {prose}\n// b\n// c\n// d\n// e\n// f\nfn g() {{}}\n");
    assert!(advise("src/x.rs", &c).is_some());
}

#[test]
fn long_run_of_terse_markers_is_clean() {
    let c = "// a\n// b\n// c\n// d\n// e\n// f\n// g\n// h\nfn f() {}\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn split_to_evade_short_lines_now_fires() {
    let frag = "// rationale fragment about forty chars\n".repeat(8);
    let c = format!("fn f() {{}}\n{frag}fn g() {{}}\n");
    assert!(advise("src/x.rs", &c).is_some());
}

#[test]
fn one_line_comment_is_clean() {
    let c = "// just one\nfn f() {}\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn two_line_comment_is_clean() {
    let c = "// one\n// two\nfn f() {}\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn module_header_and_safety_exempt() {
    let c = "//! header\n//! more\n//! lines\nfn f() {}\n// SAFETY: a\n// SAFETY: b\n// SAFETY: c\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn source_marker_run_stays_exempt() {
    let url = "https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html";
    let c = format!("//! New child module. SOURCE: {url}\n//! second line of doc here for the split\nfn f() {{}}\n");
    assert!(advise("src/x.rs", &c).is_none());
}

#[test]
fn long_doc_comment_wall_flagged() {
    let prose = "z".repeat(70);
    let c = format!("/// {prose}\n/// b\n/// c\n/// d\n/// e\n/// f\npub fn g() {{}}\n");
    assert!(advise("src/x.rs", &c).is_some());
}

#[test]
fn long_module_doc_wall_flagged() {
    let prose = "q".repeat(70);
    let c = format!("//! {prose}\n//! b\n//! c\n//! d\n//! e\n//! f\nfn f() {{}}\n");
    assert!(advise("src/x.rs", &c).is_some());
}

#[test]
fn short_doc_comment_is_clean() {
    let c = "/// adds two\npub fn add(a: i64, b: i64) -> i64 { a + b }\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn safety_run_stays_exempt() {
    let c = "// SAFETY: a\n// SAFETY: b\n// SAFETY: c\n// SAFETY: d\n// SAFETY: e\n// SAFETY: f\nfn f() {}\n";
    assert!(advise("src/x.rs", c).is_none());
}

#[test]
fn non_code_file_clean() {
    let c = "// a\n// b\n// c\n";
    assert!(advise("notes.md", c).is_none());
}

#[test]
fn python_short_hash_block_is_clean() {
    let c = "def f():\n    # one\n    # two\n    # three\n    pass\n";
    assert!(advise("x.py", c).is_none());
}

#[test]
fn python_long_prose_paragraph_flagged() {
    let prose = "y".repeat(70);
    let c = format!("def f():\n    # {prose}\n    # b\n    # c\n    # d\n    # e\n    # f\n    pass\n");
    assert!(advise("x.py", &c).is_some());
}

#[test]
fn sql_short_dash_block_is_clean() {
    let c = "-- one\n-- two\n-- three\nSELECT 1;\n";
    assert!(advise("x.sql", c).is_none());
}

#[test]
fn rust_attribute_run_not_flagged() {
    let c = "#[derive(Debug)]\n#[serde(rename = \"x\")]\n#[allow(dead_code)]\nstruct S;\n";
    assert!(advise("x.rs", c).is_none());
}

#[test]
fn long_single_comment_flagged() {
    let long = "x".repeat(120);
    let c = format!("// {long}\nfn f() {{}}\n");
    assert!(advise("x.rs", &c).is_some());
}

#[test]
fn short_single_comment_clean() {
    let c = "// short note\nfn f() {}\n";
    assert!(advise("x.rs", c).is_none());
}

#[test]
fn shebang_plus_directives_not_flagged() {
    let c = "#!/usr/bin/env python\n#include <x>\n#define Y 1\nint main(){}\n";
    assert!(advise("x.c", c).is_none());
}
