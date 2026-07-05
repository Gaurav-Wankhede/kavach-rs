use super::*;

#[test]
fn blocks_handler_without_gateway_layer() {
    let content = r#"
use axum::Router;
use axum::routing::get;

pub(crate) fn routes() -> Router {
    Router::new().route("/orders", get(get_orders))
}
"#;
    let result = check("src/handlers/order.rs", content);
    assert!(result.is_some());
    let msg = result.unwrap_or_default();
    assert!(msg.contains("[API_GATEWAY] missing gateway layer"));
    assert!(msg.contains("Missing gateway layer"));
}

#[test]
fn allows_handler_with_auth() {
    let content = r#"
use axum::Router;
use crate::middleware::auth;

pub(crate) fn routes() -> Router {
    Router::new()
        .route("/orders", get(get_orders))
        .layer(auth::layer())
}
"#;
    let result = check("src/handlers/order.rs", content);
    assert!(result.is_none());
}

#[test]
fn formats_advisory_for_protocol_leakage() {
    let content = r"
use axum::Router;
use tonic::Request;
use crate::auth;

pub async fn handler() {}
";
    let result = format_advisory("src/handlers/order.rs", content);
    assert!(result.is_some());
    let msg = result.unwrap_or_default();
    assert!(msg.contains("[API_GATEWAY_ADVISORY]"));
    assert!(msg.contains("ProtocolLeakage"));
}
