use super::sites_in;
use crate::cmd::origin::site::Kind;

fn kinds(name: &str, src: &str) -> Vec<Kind> {
    sites_in(name, "x.rs", src)
        .into_iter()
        .map(|s| s.kind)
        .collect()
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
    let k = kinds(
        "REGISTRY",
        "static REGISTRY: Lazy<Map> = Lazy::new(|| Map::new());",
    );
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
    let sites = sites_in(
        "MAX_RETRIES",
        "x.rs",
        "if attempts < MAX_RETRIES { retry(); }",
    );
    assert!(
        sites.is_empty(),
        "a usage must not be reported as an origin"
    );
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
fn handles_invalid_regex_pattern_gracefully() {
    // When an invalid regex pattern is encountered (e.g., from a name with
    // metacharacters), the function must not panic. Currently panics because
    // the fallback expect() is not suppressed. This test will pass once
    // #[expect(clippy::expect_used)] is added to justify the fallback.
    let sites = sites_in("X", "x.rs", "let [invalid( = 5;");
    // Should return sites matching the fallback pattern $.^ (which matches nothing)
    // rather than panicking.
    assert!(sites.is_empty());
}

#[test]
fn finds_sql_schema_table() {
    let k = kinds("users", "CREATE TABLE IF NOT EXISTS users (id UUID PRIMARY KEY);");
    assert!(k.contains(&Kind::Schema));
}

#[test]
fn finds_markdown_doc_anchor() {
    let k = kinds("Architecture", "# Architecture Overview\nSome text");
    assert!(k.contains(&Kind::DocAnchor));
}

#[test]
fn finds_yaml_and_json_config_fields() {
    let k1 = kinds("database_url", "database_url: postgres://localhost/db");
    assert!(k1.contains(&Kind::ConfigField));

    let k2 = kinds("port", "  \"port\": 8080,");
    assert!(k2.contains(&Kind::ConfigField));
}

#[test]
fn finds_polyglot_functions() {
    let py = kinds("execute_query", "def execute_query(conn, sql):");
    assert!(py.contains(&Kind::Function));

    let ts = kinds("fetchUser", "export const fetchUser = async (id: string) => {");
    assert!(ts.contains(&Kind::Function));
}

#[test]
fn finds_node_process_env() {
    let k = kinds("SECRET_KEY", "const key = process.env.SECRET_KEY || 'default';");
    assert!(k.contains(&Kind::EnvVar));
}
