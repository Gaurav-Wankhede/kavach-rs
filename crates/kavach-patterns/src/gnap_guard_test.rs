// SOURCE: https://doc.rust-lang.org/book/ch11-03-test-organization.html (Rust test organization)

use super::{check, detect};

#[test]
fn blocks_bearer_header() {
    let code = r#"headers.insert("Authorization: Bearer " + token);"#;
    let f = detect("src/api.rs", code);
    assert!(!f.is_empty());
    assert!(f.first().is_some_and(|x| x.fix.contains("httpsig")));
}

#[test]
fn blocks_client_id_assignment() {
    let code = r#"let client_id = "abc123";"#;
    let f = detect("src/auth.rs", code);
    assert!(!f.is_empty());
    assert!(f.first().is_some_and(|x| x.fix.contains("GNAP")));
}

#[test]
fn blocks_client_secret_assignment() {
    let code = r#"client_secret: env::var("SECRET"),"#;
    let f = detect("src/oauth.rs", code);
    assert!(!f.is_empty());
}

#[test]
fn allows_client_secret_type_declaration() {
    let code = r"pub client_secret: Option<String>,";
    let f = detect("src/dto.rs", code);
    assert!(f.is_empty(), "Type declaration should not be flagged");
}

#[test]
fn allows_client_id_type_declaration() {
    let code = r"client_id: String,";
    let f = detect("src/models.rs", code);
    assert!(f.is_empty(), "Type declaration should not be flagged");
}

#[test]
fn blocks_oauth_redirect_uri() {
    let code = r#"redirect_uri: "https://app.example.com/callback","#;
    let f = detect("src/auth.rs", code);
    assert!(!f.is_empty());
    assert!(f.first().is_some_and(|x| x.fix.contains("interact.finish")));
}

#[test]
fn blocks_grant_type() {
    let code = r"grant_type=authorization_code";
    let f = detect("src/oauth.rs", code);
    assert!(!f.is_empty());
}

#[test]
fn blocks_oauth_scope() {
    let code = r#"scope = "read write profile""#;
    let f = detect("src/auth.rs", code);
    assert!(!f.is_empty());
    assert!(f.first().is_some_and(|x| x.fix.contains("access:")));
}

#[test]
fn blocks_localstorage_token() {
    let code = r#"localStorage.setItem("token", accessToken);"#;
    let f = detect("src/app.tsx", code);
    assert!(!f.is_empty());
    assert!(f.first().is_some_and(|x| x.fix.contains("Memory-only")));
}

#[test]
fn blocks_oauth_library() {
    let code = "use oauth2::Client;";
    let f = detect("src/auth.rs", code);
    assert!(!f.is_empty());
}

#[test]
fn allows_gnap_exempt_comment() {
    let code = r#"// gnap-exempt: third-party API requires Bearer
let header = "Authorization: Bearer " + token;"#;
    let f = detect("src/external.rs", code);
    // First line exempt, second line should still be caught
    assert!(f.len() <= 1);
}

#[test]
fn allows_test_files() {
    let code = r#"client_id = "test-id";"#;
    assert!(detect("src/tests/auth_test.rs", code).is_empty());
}

#[test]
fn allows_stripe_exemption() {
    let code = r"stripe::client_id = key;";
    assert!(detect("src/payments.rs", code).is_empty());
}

#[test]
fn skips_patterns_crate() {
    let code = r#"client_id = "example";"#;
    assert!(detect("kavach-patterns/src/gnap_guard.rs", code).is_empty());
}

#[test]
fn check_returns_block_message() {
    let code = r"Authorization: Bearer token123";
    let result = check("src/api.rs", code);
    assert!(result.is_some());
    assert!(
        result
            .as_ref()
            .is_some_and(|m| m.contains("[GNAP_SAFETY]"))
    );
}

#[test]
fn blocks_refresh_token() {
    let code = r"refresh_token: stored_refresh,";
    let f = detect("src/auth.rs", code);
    assert!(!f.is_empty());
    assert!(f.first().is_some_and(|x| x.fix.contains("manage.uri")));
}

#[test]
fn blocks_client_secret_literal() {
    let code = r#"client_secret: "hardcoded_secret""#;
    let f = detect("src/config.rs", code);
    assert!(!f.is_empty(), "Literal client_secret should be blocked");
}

#[test]
fn allows_stripe_client_secret_dto() {
    let code = r"pub client_secret: Option<String>,";
    let f = detect("src/payments/stripe_dto.rs", code);
    assert!(f.is_empty(), "Stripe DTO field should not be flagged");
}
