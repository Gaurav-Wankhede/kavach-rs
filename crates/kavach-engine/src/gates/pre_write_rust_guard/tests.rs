//! Block on production `unwrap`, pass on clean/Result code, skip test files,
//! and advisory emission for `dbg!`.
use super::{check, format_advisory};

fn j(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn should_block_when_unwrap_in_production_code() {
    let code = j(&["let x = foo.unw", "rap();"]);
    assert!(check("src/lib.rs", &code).is_some());
}

#[test]
fn should_allow_when_clean_arithmetic() {
    assert!(check("src/lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }").is_none());
}

#[test]
fn should_allow_when_result_used() {
    assert!(
        check(
            "src/lib.rs",
            "fn add(a: i32, b: i32) -> Result<i32, Error> { Ok(a + b) }"
        )
        .is_none()
    );
}

#[test]
fn should_skip_test_files() {
    let code = j(&["foo.unw", "rap()"]);
    assert!(check("tests/test_api.rs", &code).is_none());
}

#[test]
fn should_emit_advisory_for_dbg_macro() {
    assert!(format_advisory("src/lib.rs", "fn f(x: i32) -> i32 { dbg!(x) }").is_some());
}

#[test]
fn should_emit_no_advisory_for_clean_code() {
    assert!(format_advisory("src/lib.rs", "fn f() {}").is_none());
}
