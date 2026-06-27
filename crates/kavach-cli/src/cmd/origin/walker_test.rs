use super::declared_name;

#[test]
fn extract_const_name() {
    assert_eq!(declared_name("const DATABASE_URL: &str"), Some("DATABASE_URL".into()));
}

#[test]
fn extract_let_name() {
    assert_eq!(declared_name("let x = 5;"), Some("x".into()));
}

#[test]
fn extract_env_var_name() {
    assert_eq!(declared_name("env::var(\"DB_URL\")"), Some("DB_URL".into()));
}

#[test]
fn no_name_returns_none() {
    assert_eq!(declared_name("// just a comment"), None);
}
