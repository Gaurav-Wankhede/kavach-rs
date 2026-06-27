use super::is_secret;

#[test]
fn secret_name_contains_pass() {
    assert!(is_secret("db_password"));
}

#[test]
fn secret_name_contains_api() {
    assert!(is_secret("api_key"));
}

#[test]
fn secret_name_contains_tok() {
    assert!(is_secret("token"));
}

#[test]
fn non_secret_database_url() {
    assert!(!is_secret("DATABASE_URL"));
}

#[test]
fn non_secret_retry_count() {
    assert!(!is_secret("retry_count"));
}
