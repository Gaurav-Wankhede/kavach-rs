// split: test module — test strings contain async fn signatures
//! Tests for API Gateway pattern detection.

use super::checks::{Severity, ViolationKind, detect};

#[test]
fn detects_missing_gateway_layer_rust() {
    let content = "use axum::Router;\nuse axum::routing::get;\n\npub fn routes() -> Router {\n    Router::new().route(\"/orders\", get(get_orders))\n}\n";
    let violations = detect("src/handlers/order.rs", content);
    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations.first().map(|v| v.kind),
        Some(ViolationKind::MissingGatewayLayer)
    );
}

#[test]
fn allows_handler_with_auth_middleware_rust() {
    let content = "use axum::Router;\nuse crate::middleware::auth;\n\npub fn routes() -> Router {\n    Router::new().route(\"/orders\", get(get_orders)).layer(auth::layer())\n}\n";
    let violations = detect("src/handlers/order.rs", content);
    assert!(!violations.iter().any(|v| v.severity == Severity::P0Block));
}

#[test]
fn detects_protocol_leakage_tonic_in_http() {
    let content = "use axum::Router;\nuse tonic::Request;\nuse crate::middleware::auth;\n";
    let violations = detect("src/handlers/order.rs", content);
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.kind == ViolationKind::ProtocolLeakage)
            .count(),
        1
    );
}

#[test]
fn detects_protocol_leakage_kafka_in_http() {
    let content = "use axum::Router;\nuse rdkafka::producer::FutureProducer;\nuse crate::auth;\n";
    let violations = detect("src/handlers/event.rs", content);
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.kind == ViolationKind::ProtocolLeakage)
            .count(),
        1
    );
}

#[test]
fn detects_missing_aggregation_many_service_calls() {
    let content = "use axum::Json;\nuse crate::auth;\n\nlet p = product_service.get(id).await?;\nlet r = review_service.list(id).await?;\nlet i = inventory_service.check(id).await?;\nlet x = recommendation_service.related(id).await?;\n";
    let violations = detect("src/handlers/product.rs", content);
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.kind == ViolationKind::MissingAggregation)
            .count(),
        1
    );
}

#[test]
fn allows_few_service_calls() {
    let content = "use axum::Json;\nuse crate::auth;\n\nlet o = order_service.get(id).await?;\nlet c = customer_service.get(cid).await?;\n";
    let violations = detect("src/handlers/order.rs", content);
    assert!(
        !violations
            .iter()
            .any(|v| v.kind == ViolationKind::MissingAggregation)
    );
}

#[test]
fn skips_non_handler_files() {
    let content =
        "use axum::Router;\npub fn routes() -> Router { Router::new().route(\"/\", get(h)) }\n";
    let violations = detect("src/lib.rs", content);
    assert!(violations.is_empty());
}

#[test]
fn skips_test_files() {
    let content =
        "use axum::Router;\npub fn routes() -> Router { Router::new().route(\"/\", get(h)) }\n";
    let violations = detect("src/handlers/order_test.rs", content);
    assert!(violations.is_empty());
}

#[test]
fn detects_typescript_handler_without_auth() {
    let content = "import express from 'express';\nconst app = express();\napp.get('/orders', (req, res) => { res.json([]); });\n";
    let violations = detect("src/api/orders.ts", content);
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.severity == Severity::P0Block)
            .count(),
        1
    );
}

#[test]
fn allows_typescript_handler_with_auth() {
    let content = "import express from 'express';\nimport { authenticate } from './middleware';\napp.get('/orders', authenticate, (req, res) => {});\n";
    let violations = detect("src/api/orders.ts", content);
    assert!(!violations.iter().any(|v| v.severity == Severity::P0Block));
}

#[test]
fn detects_python_handler_without_auth() {
    let content = "from flask import Flask\napp = Flask(__name__)\n@app.route('/orders')\ndef get_orders(): return jsonify(orders)\n";
    let violations = detect("src/views.py", content);
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.severity == Severity::P0Block)
            .count(),
        1
    );
}

#[test]
fn allows_python_handler_with_auth() {
    let content = "from flask import Flask\nfrom auth import login_required\napp = Flask(__name__)\n@app.route('/orders')\n@login_required\ndef get_orders(): return jsonify(orders)\n";
    let violations = detect("src/views.py", content);
    assert!(!violations.iter().any(|v| v.severity == Severity::P0Block));
}
