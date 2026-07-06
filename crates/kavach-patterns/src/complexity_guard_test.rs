use super::*;

#[test]
fn small_file_passes() {
    assert!(check("src/main.rs", "fn main() {\n    let x = 1;\n}\n").is_none());
}

#[test]
fn deep_nesting_detected() {
    let mut c = String::from("fn f() {\n");
    for _ in 0..7 {
        c.push_str("  if true {\n");
    }
    c.push_str("    let x = 1;\n");
    for _ in 0..7 {
        c.push_str("  }\n");
    }
    c.push_str("}\n");
    assert!(check("src/deep.rs", &c).is_some());
}

#[test]
fn skips_test_files() {
    let c = "let x = 1;\n".repeat(210);
    assert!(check("src/tests/big.rs", &c).is_none());
}

#[test]
fn counts_functions() {
    use std::fmt::Write;
    let mut c = String::with_capacity(300);
    for i in 0..20 {
        writeln!(c, "fn f{i}() {{}}").ok();
    }
    assert_eq!(analyze(&c).fn_count, 20);
}
