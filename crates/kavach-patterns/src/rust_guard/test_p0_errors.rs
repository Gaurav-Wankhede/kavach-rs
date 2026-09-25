//! P0 error-handling detector tests (unwrap/panic/todo/ok-discard/default-fill).
use crate::config::j;
use crate::rust_guard::{RustSeverity, detect};

#[test]
fn p0_unwrap() {
    let code = j(&["let x = foo.unw", "rap();"]);
    let v = detect("src/lib.rs", &code);
    assert!(v.iter().any(|x| x.severity == RustSeverity::P0Block));
}

#[test]
fn p0_panic() {
    let v = detect("src/lib.rs", "panic!(\"boom\")");
    assert!(v.iter().any(|x| x.severity == RustSeverity::P0Block));
}

#[test]
fn clean_allows() {
    let v = detect(
        "src/lib.rs",
        "fn add(a: i32, name: &str) -> Result<i32, Error> { Ok(a) }",
    );
    assert!(v.is_empty());
}

#[test]
fn test_file_skipped() {
    let code = j(&["foo.unw", "rap()"]);
    let v = detect("/project/tests/api.rs", &code);
    assert!(v.is_empty());
}

#[test]
fn p1_dbg() {
    let v = detect("src/lib.rs", "dbg!(x);");
    assert!(v.iter().any(|x| x.severity == RustSeverity::P1Advisory));
}

#[test]
fn p0_unwrap_or() {
    let code = j(&["let x = foo.unw", "rap_or(\"\");"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "unwrap_or")
    );
}

#[test]
fn p0_unwrap_or_default() {
    let code = j(&["let x = foo.unw", "rap_or_default();"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "unwrap_or_default")
    );
}

#[test]
fn p0_ok_discard() {
    let code = j(&["let _ = save().o", "k();"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "result.ok()")
    );
}

#[test]
fn p0_todo() {
    let code = j(&["to", "do!()"]);
    let v = detect("src/lib.rs", &code);
    assert!(v.iter().any(|x| x.severity == RustSeverity::P0Block));
}

#[test]
fn p0_default_fill() {
    let code = j(&["    ..Defa", "ult::default()"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern.contains("Default"))
    );
}

#[test]
fn comment_prose_mentions_do_not_fire() {
    // Joined trigger words in comment prose are not code using them.
    // (Spelled plainly here, not via j(): the split form could never match
    // even on raw text, which would vacate this proof. The carrier fn has a
    // body so only the comment is under test.)
    let code = "// never call unwrap() or panic! here\nfn x() { 1 }";
    let v = detect("src/lib.rs", code);
    assert!(
        v.iter().all(|x| x.severity != RustSeverity::P0Block),
        "got: {v:?}"
    );
}

#[test]
fn string_text_mentions_do_not_fire() {
    let code = "let msg = \"do not unwrap() or panic!\";";
    let v = detect("src/lib.rs", code);
    assert!(
        v.iter().all(|x| x.severity != RustSeverity::P0Block),
        "got: {v:?}"
    );
}
