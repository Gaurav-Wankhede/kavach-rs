use super::*;

#[test]
fn test_sensitive() {
    assert!(is_sensitive(".env.local"));
    assert!(!is_sensitive("main.rs"));
}

#[test]
fn test_code_file() {
    assert!(is_code_file("a.rs"));
    assert!(!is_code_file("a.md"));
}

#[test]
fn test_infra() {
    assert!(is_infra_file("Dockerfile"));
    assert!(!is_infra_file("Cargo.toml"));
}
