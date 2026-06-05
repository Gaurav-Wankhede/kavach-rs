use crate::solid_guard::detect;

#[test]
fn axum_dip_state_concrete_flagged() {
    let src = r"
use axum::extract::State;
use sqlx;
pub async fn list(State(pool): State<sqlx::PgPool>) {}
";
    let r = detect("src/handlers/users.rs", src);
    assert!(r.iter().any(|v| v.pattern == "axum-dip-state-concrete"));
}

#[test]
fn axum_dip_state_arc_concrete_flagged() {
    let src = r"
use axum::extract::State;
use std::sync::Arc;
use reqwest;
pub async fn ping(State(c): State<Arc<reqwest::Client>>) {}
";
    let r = detect("src/handlers/ping.rs", src);
    assert!(r.iter().any(|v| v.pattern == "axum-dip-state-concrete"));
}

#[test]
fn axum_dip_state_trait_object_ok() {
    let src = r"
use axum::extract::State;
use std::sync::Arc;
async fn x() {}
pub async fn list(State(repo): State<Arc<dyn UserRepository + Send + Sync>>) {}
";
    let r = detect("src/handlers/users.rs", src);
    assert!(!r.iter().any(|v| v.pattern == "axum-dip-state-concrete"));
}
