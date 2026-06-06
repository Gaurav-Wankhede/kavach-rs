//! API Gateway violation detection logic.

use std::path::Path;

/// Severity of an API Gateway violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    /// P0: blocks the write
    P0Block,
    /// P1: advisory only
    P1Advisory,
}

/// Kind of API Gateway violation detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-engine api_gateway_guard; non_exhaustive => E0004"
)]
pub enum ViolationKind {
    /// Handler exposes endpoint without auth/rate-limit middleware
    MissingGatewayLayer,
    /// Internal protocol types (gRPC, Kafka) leak into HTTP handler
    ProtocolLeakage,
    /// Handler makes 3+ separate service calls without aggregation
    MissingAggregation,
}

/// A detected API Gateway violation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Violation {
    pub kind: ViolationKind,
    pub severity: Severity,
    pub message: String,
    pub fix: String,
}

/// Returns true if this file path matches handler/route patterns.
#[must_use]
pub fn is_handler_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    let path_obj = Path::new(path);
    if path_obj
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
        && (lower.contains("handler") || lower.contains("route"))
    {
        return true;
    }
    if (path_obj
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ts"))
        || path_obj
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("tsx")))
        && (lower.contains("/api/")
            || lower.contains("/routes/")
            || lower.contains("/handlers/")
            || lower.contains("handler.")
            || lower.contains("route."))
    {
        return true;
    }
    if path_obj
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("py"))
        && (lower.contains("handler") || lower.contains("route") || lower.contains("views"))
    {
        return true;
    }
    if path_obj
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("go"))
        && (lower.contains("handler") || lower.contains("route"))
    {
        return true;
    }
    false
}

/// Detect API Gateway violations in handler/route file content.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<Violation> {
    if !is_handler_file(file_path) {
        return Vec::new();
    }
    if file_path.contains("test") || file_path.contains("spec") {
        return Vec::new();
    }

    let mut violations = Vec::new();
    let lower = content.to_lowercase();

    if let Some(v) = check_missing_gateway_layer(file_path, &lower) {
        violations.push(v);
    }
    if let Some(v) = check_protocol_leakage(file_path, &lower) {
        violations.push(v);
    }
    if let Some(v) = check_missing_aggregation(file_path, content) {
        violations.push(v);
    }

    violations
}

fn check_missing_gateway_layer(file_path: &str, lower_content: &str) -> Option<Violation> {
    let has_route = lower_content.contains("router::new")
        || lower_content.contains(".route(")
        || lower_content.contains("app.get(")
        || lower_content.contains("app.post(")
        || lower_content.contains("app.put(")
        || lower_content.contains("app.delete(")
        || lower_content.contains("@app.route")
        || lower_content.contains("@router.")
        || lower_content.contains("http.handlefunc");

    if !has_route {
        return None;
    }

    let has_gateway = lower_content.contains("auth")
        || lower_content.contains("authenticate")
        || lower_content.contains("rate_limit")
        || lower_content.contains("ratelimit")
        || lower_content.contains("middleware")
        || lower_content.contains("layer(")
        || lower_content.contains("@require")
        || lower_content.contains("@login_required")
        || lower_content.contains("@authenticated");

    if has_gateway {
        return None;
    }

    Some(Violation {
        kind: ViolationKind::MissingGatewayLayer,
        severity: Severity::P0Block,
        message: format!("Handler without auth/rate-limit middleware: {file_path}"),
        fix: "Add auth middleware import and wire into route chain".to_owned(),
    })
}

fn check_protocol_leakage(file_path: &str, lower_content: &str) -> Option<Violation> {
    let is_http = lower_content.contains("axum::")
        || lower_content.contains("actix")
        || lower_content.contains("warp::")
        || lower_content.contains("express")
        || lower_content.contains("fastify")
        || lower_content.contains("flask")
        || lower_content.contains("fastapi")
        || lower_content.contains("gin.")
        || lower_content.contains("http.handle");

    if !is_http {
        return None;
    }

    let has_grpc = lower_content.contains("tonic::")
        || lower_content.contains("prost::")
        || lower_content.contains("@grpc/grpc-js")
        || lower_content.contains("grpcio");

    let has_kafka = lower_content.contains("rdkafka::")
        || lower_content.contains("kafkajs")
        || lower_content.contains("kafka-python")
        || lower_content.contains("sarama");

    if !has_grpc && !has_kafka {
        return None;
    }

    let protocol = if has_grpc { "gRPC" } else { "Kafka" };
    Some(Violation {
        kind: ViolationKind::ProtocolLeakage,
        severity: Severity::P1Advisory,
        message: format!("{protocol} types in HTTP handler — use DTOs: {file_path}"),
        fix: "Define HTTP-specific DTOs; map protocol types at boundary".to_owned(),
    })
}

fn check_missing_aggregation(file_path: &str, content: &str) -> Option<Violation> {
    let mut service_call_count = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') {
            continue;
        }
        if trimmed.contains("_service.") && trimmed.contains(".await") {
            service_call_count = service_call_count.saturating_add(1);
        }
        if trimmed.contains("Service::") && trimmed.contains(".await") {
            service_call_count = service_call_count.saturating_add(1);
        }
        if trimmed.contains("await fetch(") {
            service_call_count = service_call_count.saturating_add(1);
        }
        if trimmed.starts_with("await ") && trimmed.contains("_service.") {
            service_call_count = service_call_count.saturating_add(1);
        }
    }

    if service_call_count < 3 {
        return None;
    }

    Some(Violation {
        kind: ViolationKind::MissingAggregation,
        severity: Severity::P1Advisory,
        message: format!(
            "{service_call_count} service calls in handler — extract an aggregator service: {file_path}"
        ),
        fix: "Create aggregation service; handler calls aggregator once".to_owned(),
    })
}
