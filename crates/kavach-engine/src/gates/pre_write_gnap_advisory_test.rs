//! Auth-relevance detection + advisory presence/absence coverage.
use super::advisory::advisory;
use super::detect::is_auth_related;

#[test]
fn detects_auth_path() {
    assert!(is_auth_related("src/auth/handler.rs", ""));
    assert!(is_auth_related("src/gnap/grants.rs", ""));
    assert!(is_auth_related("src/token/rotate.rs", ""));
}

#[test]
fn detects_auth_content() {
    assert!(is_auth_related(
        "src/api.rs",
        "let access_token = response.token;"
    ));
    assert!(is_auth_related("src/handler.rs", "Authorization: Bearer"));
    assert!(is_auth_related("src/client.rs", "Signature-Input: sig1="));
}

#[test]
fn ignores_non_auth() {
    assert!(!is_auth_related("src/utils.rs", "fn parse_json() {}"));
    assert!(!is_auth_related(
        "src/models.rs",
        "struct User { name: String }"
    ));
}

#[test]
fn advisory_returns_some_for_auth() {
    let result = advisory("src/auth/handler.rs", "fn handle_login() {}");
    assert!(result.is_some());
    assert!(result.as_ref().is_some_and(|s| s.contains("GNAP_SPEC_REF")));
}

#[test]
fn advisory_returns_none_for_non_auth() {
    let result = advisory("src/utils.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");
    assert!(result.is_none());
}
