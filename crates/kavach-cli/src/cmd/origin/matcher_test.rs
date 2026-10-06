use super::sites_in;
use crate::cmd::origin::site::Kind;

fn kinds_ext(name: &str, file: &str, src: &str) -> Vec<Kind> {
    sites_in(name, file, src)
        .into_iter()
        .map(|s| s.kind)
        .collect()
}

fn kinds(name: &str, src: &str) -> Vec<Kind> {
    kinds_ext(name, "x.rs", src)
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
    let k = kinds("EnvVar", "enum Foo {\n    EnvVar,\n}");
    assert!(k.contains(&Kind::Variant));
}

#[test]
fn handles_invalid_regex_pattern_gracefully() {
    let sites = sites_in("X", "x.rs", "let [invalid( = 5;");
    assert!(sites.is_empty());
}

#[test]
fn finds_sql_schema_table() {
    let k = kinds_ext("users", "schema.sql", "CREATE TABLE IF NOT EXISTS users (id UUID PRIMARY KEY);");
    assert!(k.contains(&Kind::Schema));
}

#[test]
fn finds_markdown_doc_anchor() {
    let k = kinds_ext("Architecture", "README.md", "# Architecture Overview\nSome text");
    assert!(k.contains(&Kind::DocAnchor));
}

#[test]
fn finds_yaml_and_json_config_fields() {
    let k1 = kinds_ext("database_url", "config.yaml", "database_url: postgres://localhost/db");
    assert!(k1.contains(&Kind::ConfigField));

    let k2 = kinds_ext("port", "config.json", "{\n  \"port\": 8080\n}");
    assert!(k2.contains(&Kind::ConfigField));
}

#[test]
fn finds_polyglot_functions() {
    let py = kinds_ext("execute_query", "db.py", "def execute_query(conn, sql):\n    pass\n");
    assert!(py.contains(&Kind::Function));

    let ts = kinds_ext("fetchUser", "api.ts", "export const fetchUser = async (id: string) => {\n    return id;\n};\n");
    assert!(ts.contains(&Kind::Function));
}

#[test]
fn ast_ignores_comments_and_strings() {
    let rs = sites_in("fake_fn", "test.rs", "// fn fake_fn() {}\nlet s = \"fn fake_fn() {}\";\n");
    println!("DEBUG SITES: {rs:?}");
    assert!(rs.is_empty(), "comments and strings must never match as declarations");
}

#[test]
fn finds_node_process_env() {
    let k = kinds_ext("SECRET_KEY", "app.js", "const key = process.env.SECRET_KEY || 'default';");
    assert!(k.contains(&Kind::EnvVar));
}
