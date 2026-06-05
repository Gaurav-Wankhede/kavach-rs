use crate::solid_guard::{detect, warn_count};

#[test]
fn axum_srp_handler_builds_router_flagged() {
    let src = r#"
use axum::Router;
async fn x() {}
pub async fn handler() {
    let _ = Router::new().route("/x", axum::routing::get(|| async {}));
}
"#;
    let r = detect("src/handlers/bad.rs", src);
    assert!(
        r.iter()
            .any(|v| v.pattern == "axum-srp-handler-builds-router")
    );
}

#[test]
fn axum_handler_raw_request_flagged() {
    let src = r"
use axum::http::Request;
async fn x() {}
pub async fn handler(req: Request<axum::body::Body>) {}
";
    let r = detect("src/handlers/raw.rs", src);
    assert!(r.iter().any(|v| v.pattern == "axum-handler-raw-request"));
}

#[test]
fn clean_file_no_violations() {
    let src = r"
use sqlx;
pub trait UserRepository { fn find(&self, id: u64); }
pub struct UserService<R: UserRepository> { repo: R }
async fn handler() {}
";
    let r = detect("src/services/user.rs", src);
    assert!(r.is_empty(), "expected no violations, got: {r:?}");
}

#[test]
fn non_rust_file_skipped() {
    let r = detect("src/index.ts", "match provider { Stripe => 1 }");
    assert!(r.is_empty());
}

#[test]
fn test_file_skipped() {
    let src = r"
use sqlx;
async fn x() {}
pub async fn list(pool: &sqlx::PgPool) {}
";
    let r = detect("crate/tests/users.rs", src);
    assert!(r.is_empty());
}

#[test]
fn warn_count_works() {
    let src = r"
use sqlx;
async fn x() {}
pub trait Storage { fn get(&self); fn put(&self); fn delete(&self); }
pub async fn h(pool: &sqlx::PgPool) {}
";
    assert!(warn_count("src/services/x.rs", src) >= 2);
}
