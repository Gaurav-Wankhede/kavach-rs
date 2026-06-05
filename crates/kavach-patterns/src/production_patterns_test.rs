//! Tests for production pattern detection.

use crate::config::j;
use crate::production_patterns::scan;

#[test]
fn detects_unwrap() {
    let unw = j(&["opt.unw", "rap()"]);
    let code = format!("let x = {unw};");
    let matches = scan("src/lib.rs", &code);
    assert!(!matches.is_empty());
    assert_eq!(matches[0].code, "UNWRAP");
}

#[test]
fn detects_money_float() {
    let code = r"pub price: f64,";
    let matches = scan("src/lib.rs", code);
    assert!(matches.iter().any(|m| m.code == "MONEY_FLOAT"));
}

#[test]
fn detects_hardcoded_secret() {
    let sec = j(&["let api", "_key = ", "\"sk_li", "ve_12345", "678\";"]);
    let matches = scan("src/lib.rs", &sec);
    assert!(matches.iter().any(|m| m.code == "HARDCODED_SECRET"));
}

#[test]
fn skips_test_files() {
    let unw = j(&["opt.unw", "rap()"]);
    let code = format!("let x = {unw};");
    let matches = scan("src/tests/test_foo.rs", &code);
    assert!(matches.is_empty());
}

#[test]
fn detects_serde_default_bool() {
    let code = r"
        #[serde(default)]
        pub is_admin: bool,
        ";
    let matches = scan("src/lib.rs", code);
    assert!(matches.iter().any(|m| m.code == "SERDE_DEFAULT_BOOL"));
}
