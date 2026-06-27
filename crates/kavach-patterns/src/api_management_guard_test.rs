//! Sidecar tests for `api_management_guard` (included by the parent as
//! `#[path] mod tests;`, so this file is the module body — no inner `mod tests`).
use crate::api_management_guard::detect;
fn live_credential_string() -> String {
    let prefix = format!("{}_{}", "sk", "live");
    let suffix = "_abcdef0123456789xyz";
    let mut s = String::from("const KEY: &str = \"");
    s.push_str(&prefix);
    s.push_str(suffix);
    s.push_str("\";");
    s
}
#[test]
fn detects_frontend_bare_fetch() {
    let v = detect(
        "src/components/Profile.tsx",
        "export const P = () => { fetch('/api/me').then(r => r.json()); return <div alt='' />; };",
    );
    assert!(v.iter().any(|x| x.pattern == "frontend bare fetch()"));
}
#[test]
fn allows_authfetch_wrapper() {
    let v = detect(
        "src/components/Profile.tsx",
        "import { authFetch } from '@/lib'; export const P = () => { authFetch('/api/me'); return <div alt='' />; };",
    );
    assert!(!v.iter().any(|x| x.pattern == "frontend bare fetch()"));
}
#[test]
fn detects_hardcoded_api_url() {
    let v = detect(
        "src/components/Card.tsx",
        "const URL = 'https://api.example.com/users';",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "hardcoded API URL in frontend")
    );
}
#[test]
fn detects_route_without_version() {
    let v = detect(
        "src/handlers/users.rs",
        "use axum::Router; let r = Router::new().get(\"/users\", list_users);",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "route without version prefix")
    );
}
#[test]
fn allows_versioned_route() {
    let v = detect(
        "src/handlers/users.rs",
        "use axum::Router; let r = Router::new().layer(GovernorLayer::new()).get(\"/v1/users\", list);",
    );
    assert!(
        !v.iter()
            .any(|x| x.pattern == "route without version prefix")
    );
}
#[test]
fn detects_list_without_pagination() {
    let v = detect(
        "src/handlers/users.rs",
        "use axum::Router; pub async fn list_users() -> Result<Json<Vec<User>>> { Ok(Json(vec![])) }",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "list endpoint without pagination")
    );
}
#[test]
fn detects_untyped_json_body() {
    let v = detect(
        "src/handlers/x.rs",
        "use axum::Router; pub fn h(Json(p): Json<serde_json::Value>) {}",
    );
    assert!(v.iter().any(|x| x.pattern == "untyped JSON body"));
}
#[test]
fn detects_db_row_leaked() {
    let v = detect(
        "src/handlers/u.rs",
        "use axum::Router; pub async fn get_user() -> Result<Json<User>> { todo!() }",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "DB row leaked in API response")
    );
}
#[test]
fn detects_router_without_rate_limit() {
    let v = detect(
        "src/handlers/main.rs",
        "use axum::Router; let app = Router::new().route(\"/v1/x\", get(h));",
    );
    assert!(v.iter().any(|x| x.pattern == "router without rate limit"));
}
#[test]
fn allows_router_with_rate_limit() {
    let v = detect(
        "src/handlers/main.rs",
        "use axum::Router; let app = Router::new().layer(GovernorLayer::new()).route(\"/v1/x\", get(h));",
    );
    assert!(!v.iter().any(|x| x.pattern == "router without rate limit"));
}
#[test]
fn detects_webhook_without_signature() {
    let v = detect(
        "src/webhooks/stripe.rs",
        "pub async fn stripe_webhook(body: Bytes) -> Result<()> { Ok(()) }",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "webhook without signature verification")
    );
}
#[test]
fn allows_webhook_with_signature_verify() {
    let v = detect(
        "src/webhooks/stripe.rs",
        "pub async fn stripe_webhook(body: Bytes) -> Result<()> { Stripe::Webhook::construct_event(&body, sig, secret, tolerance)?; Ok(()) }",
    );
    assert!(
        !v.iter()
            .any(|x| x.pattern == "webhook without signature verification")
    );
}
#[test]
fn detects_query_without_tenant() {
    let v = detect(
        "src/db/users.rs",
        "sqlx::query(\"SELECT name FROM users WHERE id = $1\")",
    );
    assert!(v.iter().any(|x| x.pattern == "query without tenant filter"));
}
#[test]
fn allows_query_with_tenant() {
    let v = detect(
        "src/db/users.rs",
        "sqlx::query(\"SELECT name FROM users WHERE tenant_id = $1 AND id = $2\")",
    );
    assert!(!v.iter().any(|x| x.pattern == "query without tenant filter"));
}
#[test]
fn detects_cors_wildcard() {
    let v = detect(
        "workers/api-gateway.ts",
        "addEventListener('fetch', e => e.respondWith(new Response('', { headers: { 'Access-Control-Allow-Origin': '*' } })));",
    );
    assert!(v.iter().any(|x| x.pattern == "CORS wildcard origin"));
}
#[test]
fn detects_credential_in_source() {
    let code = live_credential_string();
    let v = detect("src/config.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.pattern == "API credential literal in source")
    );
}
#[test]
fn detects_third_party_sdk_no_timeout() {
    let v = detect(
        "src/handlers/x.rs",
        "use axum::Router; let s = stripe::Client::new(key);",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "third-party SDK without timeout")
    );
}
#[test]
fn detects_jwt_decode_without_verify() {
    let v = detect(
        "src/handlers/auth.rs",
        "use axum::Router; let claims = jsonwebtoken::decode::<Claims>(&token, &key, &Validation::new(Algorithm::HS256));",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "JWT/PASETO decode without verify")
    );
}
#[test]
fn allows_jwt_with_validation() {
    let v = detect(
        "src/handlers/auth.rs",
        "use axum::Router; let mut val = Validation::default(); val.validate_required_claims(&[]); let claims = jsonwebtoken::decode::<Claims>(&token, &key, &val);",
    );
    assert!(
        !v.iter()
            .any(|x| x.pattern == "JWT/PASETO decode without verify")
    );
}
#[test]
fn non_api_file_skipped() {
    let v = detect("README.md", "fetch('/api/x')");
    assert!(v.is_empty());
}
#[test]
fn test_file_skipped() {
    let v = detect("/project/tests/api_test.rs", "let r = Router::new();");
    assert!(v.is_empty());
}
#[test]
fn tsx_in_api_dir_classified_backend() {
    let v = detect(
        "src/api/handlers/users.tsx",
        "use axum::Router; let r = Router::new().get(\"/users\", h);",
    );
    assert!(
        v.iter()
            .any(|x| x.pattern == "route without version prefix")
    );
}
#[test]
fn auto_id_in_comment_not_flagged() {
    let code = "use axum::Router;\n// Example: struct UserResponse { id: i64, name: String }\npub fn ok() {}";
    let v = detect("src/handlers/x.rs", code);
    assert!(!v.iter().any(|x| x.pattern == "auto-increment ID exposed"));
}
#[test]
fn auto_id_in_code_flagged() {
    let code = "use axum::Router;\npub struct UserResponse { pub id: i64, pub name: String }";
    let v = detect("src/handlers/x.rs", code);
    assert!(v.iter().any(|x| x.pattern == "auto-increment ID exposed"));
}
#[test]
fn detects_short_aws_credential() {
    // Built at runtime to avoid scanner trip on this file's source.
    let prefix = format!("{}{}", "AK", "IA");
    let body = "1234567890";
    let code = format!("const KEY: &str = \"{prefix}{body}\";");
    let v = detect("src/config.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.pattern == "API credential literal in source")
    );
}
