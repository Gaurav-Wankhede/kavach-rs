use crate::rust_196_guard::detect;

#[test]
fn detects_legacy_mod_rs() {
    let v = detect("src/utils/mod.rs", "pub fn helper() {}");
    assert!(v.iter().any(|x| x.pattern == "legacy mod.rs file"));
}

#[test]
fn allows_modern_module_layout() {
    let v = detect("src/utils.rs", "pub mod helpers;");
    assert!(!v.iter().any(|x| x.pattern == "legacy mod.rs file"));
}

#[test]
fn detects_edition_2018() {
    let v = detect(
        "Cargo.toml",
        "[package]\nname = \"foo\"\nedition = \"2018\"",
    );
    assert!(v.iter().any(|x| x.pattern == "stale Rust edition"));
}

#[test]
fn detects_edition_2021() {
    let v = detect("Cargo.toml", "[package]\nedition = \"2021\"");
    assert!(v.iter().any(|x| x.pattern == "stale Rust edition"));
}

#[test]
fn allows_edition_2024() {
    let v = detect("Cargo.toml", "[package]\nedition = \"2024\"");
    assert!(!v.iter().any(|x| x.pattern == "stale Rust edition"));
}

#[test]
fn detects_cfg_if_dependency() {
    let v = detect("Cargo.toml", "[dependencies]\ncfg-if = \"1.0\"");
    assert!(v.iter().any(|x| x.pattern == "cfg-if dependency"));
}

#[test]
fn detects_async_trait_attribute() {
    let v = detect(
        "src/repo.rs",
        "#[async_trait]\npub trait Repo { async fn fetch(&self); }",
    );
    assert!(v.iter().any(|x| x.pattern == "async_trait attribute"));
}

#[test]
fn detects_static_mut() {
    let v = detect("src/counter.rs", "static mut COUNTER: u32 = 0;");
    assert!(v.iter().any(|x| x.pattern == "static mut"));
}

#[test]
fn detects_block_on_in_async() {
    let v = detect(
        "src/handler.rs",
        "async fn handler() { let x = futures::executor::block_on(fetch()); }",
    );
    assert!(v.iter().any(|x| x.pattern == "block_on in async fn"));
}

#[test]
fn detects_cfg_if_macro() {
    let v = detect(
        "src/platform.rs",
        "cfg_if! { if #[cfg(unix)] { fn x() {} } else { fn x() {} } }",
    );
    assert!(v.iter().any(|x| x.pattern == "cfg_if! macro"));
}

#[test]
fn detects_box_dyn_any() {
    let v = detect("src/store.rs", "fn store(v: Box<dyn Any + Send>) {}");
    assert!(v.iter().any(|x| x.pattern == "Box<dyn Any>"));
}

#[test]
fn detects_debug_on_sensitive_type() {
    let v = detect(
        "src/user.rs",
        "#[derive(Debug, Clone)]\npub struct User { pub name: String, pub password: String }",
    );
    assert!(v.iter().any(|x| x.pattern == "Debug on sensitive type"));
}

#[test]
fn detects_env_var_unwrap() {
    let v = detect(
        "src/config.rs",
        "let url = std::env::var(\"DATABASE_URL\").unwrap();",
    );
    assert!(v.iter().any(|x| x.pattern == "env::var unwrap at runtime"));
}

#[test]
fn detects_self_as_struct_import() {
    let v = detect("src/lib.rs", "use crate::types::Widget::{self as W};");
    assert!(
        v.iter()
            .any(|x| x.pattern == "{self as Name} struct/enum import")
    );
}

#[test]
fn allows_self_glob_module_import() {
    let v = detect("src/lib.rs", "use std::io::{self, Read};");
    assert!(
        !v.iter()
            .any(|x| x.pattern == "{self as Name} struct/enum import")
    );
}

#[test]
fn detects_duplicate_link_attrs() {
    let v = detect(
        "src/ffi.rs",
        "#[link_name = \"foo\"]\n#[link_name = \"bar\"]\nextern \"C\" { fn f(); }",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "duplicate export_name/link_name/link_section")
    );
}

#[test]
fn detects_nested_if_let_pyramid() {
    let v = detect(
        "src/lib.rs",
        "if let Some(a) = x { if let Some(b) = y { if let Some(c) = z { if let Some(d) = w { } } } }",
    );
    assert!(v.iter().any(|x| x.pattern == "nested if let pyramid"));
}

#[test]
fn detects_vec_i32_indices() {
    let v = detect(
        "src/lib.rs",
        "let mut indices: Vec<i32> = Vec::new(); // indices into the slice",
    );
    assert!(v.iter().any(|x| x.pattern == "Vec<i32> for indices"));
}

#[test]
fn detects_str_for_path() {
    let v = detect(
        "src/config.rs",
        "pub fn read_config(path: &str) -> String { String::new() }",
    );
    assert!(v.iter().any(|x| x.pattern == "&str for filesystem path"));
}

#[test]
fn allows_path_param() {
    let v = detect(
        "src/config.rs",
        "pub fn read_config(path: &Path) -> String { String::new() }",
    );
    assert!(!v.iter().any(|x| x.pattern == "&str for filesystem path"));
}

#[test]
fn detects_unchecked_money_arithmetic() {
    let v = detect(
        "src/billing.rs",
        "let total_cents = price_cents + tax_cents;",
    );
    assert!(v.iter().any(|x| x.pattern == "unchecked cents arithmetic"));
}

#[test]
fn allows_checked_money_arithmetic() {
    let v = detect(
        "src/billing.rs",
        "let total_cents = price_cents.checked_add(tax_cents).ok_or(Err)?;",
    );
    assert!(!v.iter().any(|x| x.pattern == "unchecked cents arithmetic"));
}

#[test]
fn test_file_skipped() {
    let v = detect("/project/tests/integration.rs", "static mut TEST: u32 = 0;");
    assert!(v.is_empty());
}

#[test]
fn detects_assert_matches_idiom() {
    let v = detect("src/x.rs", "assert!(matches!(roll(), 1..=6));");
    assert!(v.iter().any(|x| x.pattern == "assert!(matches!) over assert_matches!"));
}

#[test]
fn detects_double_negation() {
    let v = detect("src/x.rs", "let y = - -x;");
    assert!(v.iter().any(|x| x.pattern == "double negation"));
}

#[test]
fn detects_manual_range_struct() {
    let v = detect(
        "src/x.rs",
        "struct Span {\n    start: usize,\n    end: usize,\n}",
    );
    assert!(v.iter().any(|x| x.pattern == "manual range struct over core::range"));
}
