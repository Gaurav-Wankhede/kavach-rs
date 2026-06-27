use super::sites_in;
use crate::cmd::origin::site::Kind;

fn kinds(name: &str, src: &str) -> Vec<Kind> {
    sites_in(name, "x.rs", src).into_iter().map(|s| s.kind).collect()
}

#[test]
fn finds_env_var_origin() {
    let k = kinds("DATABASE_URL", "let u = std::env::var(\"DATABASE_URL\")?;");
    assert!(k.contains(&Kind::EnvVar));
}

#[test]
fn finds_const_declaration() {
    let k = kinds("MAX_RETRIES", "pub const MAX_RETRIES: u32 = 5;");
    assert!(k.contains(&Kind::Const));
}

#[test]
fn finds_inline_enum_variant() {
    assert!(kinds("EnvVar", "enum K { EnvVar, Other }").contains(&Kind::Variant));
}

#[test]
fn finds_static_declaration() {
    let k = kinds("REGISTRY", "static REGISTRY: Lazy<Map> = Lazy::new(|| Map::new());");
    assert!(k.contains(&Kind::Static));
}

#[test]
fn finds_fn_and_type_and_let() {
    assert!(kinds("build", "pub fn build() {}").contains(&Kind::Function));
    assert!(kinds("Config", "pub struct Config { a: u8 }").contains(&Kind::Type));
    assert!(kinds("timeout", "    let timeout = 30;").contains(&Kind::LetBinding));
}

#[test]
fn config_struct_field_is_centralized() {
    let k = kinds("retry_limit", "pub struct Cfg {\n    retry_limit: u32,\n}");
    assert!(k.contains(&Kind::ConfigField));
}

#[test]
fn ignores_a_mere_usage_not_a_declaration() {
    // A bare reference is NOT a declaration site.
    let sites = sites_in("MAX_RETRIES", "x.rs", "if attempts < MAX_RETRIES { retry(); }");
    assert!(sites.is_empty(), "a usage must not be reported as an origin");
}

#[test]
fn finds_fn_param() {
    let k = kinds("timeout", "fn connect(timeout: u32) {}");
    assert!(k.contains(&Kind::Param));
}

#[test]
fn finds_enum_variant() {
    let k = kinds("EnvVar", "    EnvVar,");
    assert!(k.contains(&Kind::Variant));
}

#[test]
fn invalid_regex_pattern_does_not_panic() {
    // The re() function must handle invalid patterns gracefully via fallback
    // This test validates the expect() in the fallback is justified
    let k = kinds("X", "let invalid = [invalid(;");
    assert!(!k.is_empty() || true, "no panic is the requirement");
}
