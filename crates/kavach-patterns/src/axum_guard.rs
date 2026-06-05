// split: Single-module gate file. Tests intentionally contain async fn samples in regex strings.
//! Axum 0.8 Production Gate — Native Async Traits + Anti-Pattern Detection
//!
//! SOURCES (verified 2026-05):
//! - <https://docs.rs/axum/latest/axum/error_handling/index.html>
//! - <https://docs.rs/axum/latest/axum/extract/index.html>
//! - <https://docs.rs/axum/latest/axum/middleware/index.html>
//! - <https://oneuptime.com/blog/post/2026-01-07-rust-axum-rest-api/view>

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::exhaustive_enums, reason = "fixed P0/P1/P2 severity tiers")]
pub enum AxumSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[expect(clippy::exhaustive_structs, reason = "internal-only DTO; fixed fields")]
pub struct AxumViolation {
    pub severity: AxumSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

fn init_patterns() -> Vec<Regex> {
    [
        r"#\[async_trait\][\s\S]{0,100}impl[^{]*From(?:Request|RequestParts)",
        r#"\.(?:get|post|put|delete|patch)\([^)]*"/[^"]*:\w+"#,
        r"Json<serde_json::Value>|Json<Value>",
        r"impl\s+IntoResponse[^}]*\{[^}]*(?:tracing::|log::|println!|eprintln!)",
        r"(?s)\.layer\([\s\S]*?\)[\s\S]*?\.layer\([\s\S]*?\)[\s\S]*?\.layer\(",
        r"Router::new\(\)",
        r"async\s+fn\s+\w+[^{]*\{[^}]*spawn_blocking",
        r"async\s+fn\s+\w+[^{]*\{[^}]*(?:futures::executor::block_on|tokio::runtime::Handle::block_on)",
        r"TimeoutLayer::",
        r"async\s+fn\s+\w+[^{]*->\s*(?:Result<[^,]+,\s*anyhow::Error>|anyhow::Result)",
        r"(?:Extension|State)<[^>]+>\)\s*[^{]*\{[^}]*\.unwrap\(\)",
        r"async\s+fn\s+(?:get|post|put|delete|patch|create|update|list|fetch)_\w+",
        r"TraceLayer::",
        r"SocketAddr::from\(\(\[0,\s*0,\s*0,\s*0\][^)]*\)\)",
        r"async\s+fn\s+\w+\([^)]*\bbody:\s*String\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
}

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(init_patterns);

/// Detect Axum 0.8 production anti-patterns
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "pattern detection logic is cohesive; splitting would fragment control flow across P0/P1/P2 tiers"
)]
pub fn detect(file_path: &str, content: &str) -> Vec<AxumViolation> {
    if !is_axum_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let mut violations = Vec::new();
    let patterns = &**PATTERNS;

    // P0 violations
    if patterns.first().is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "async_trait on FromRequest",
            fix: "Axum 0.8 uses native async traits. Remove #[async_trait] from FromRequest/FromRequestParts impls.",
            line: 0,
        });
    }
    if patterns.get(1).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "old path syntax /:id",
            fix: "Axum 0.8 uses /{id} syntax. Replace /:param with /{param} in route paths.",
            line: 0,
        });
    }
    if patterns.get(2).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "Json<Value> without schema",
            fix: "Use typed DTO: Json<MyRequest> with #[derive(Deserialize, Validate)]. Untyped JSON bypasses validation.",
            line: 0,
        });
    }
    if patterns.get(3).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "logging in IntoResponse",
            fix: "Insert error into response.extensions(); log via middleware. IntoResponse must be pure.",
            line: 0,
        });
    }
    if patterns.get(7).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "block_on in async handler",
            fix: "Never block_on inside async fn — causes deadlock. Use .await or spawn_blocking for sync work.",
            line: 0,
        });
    }
    if patterns.get(10).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "unwrap on extractor",
            fix: "Handle Extension/State extraction with ? — unwrap panics return 500 with no error context.",
            line: 0,
        });
    }
    if patterns.get(14).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P0Block,
            pattern: "raw String body handler",
            fix: "Use typed extractor: Json<T>, Form<T>, or Bytes with size limits. String body bypasses validation.",
            line: 0,
        });
    }

    // P1 advisories
    if patterns.get(4).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P1Advisory,
            pattern: "repeated .layer() calls",
            fix: "Use tower::ServiceBuilder for multiple layers. Repeated .layer() reverses order.",
            line: 0,
        });
    }
    if patterns.get(9).is_some_and(|p| p.is_match(content)) {
        violations.push(AxumViolation {
            severity: AxumSeverity::P1Advisory,
            pattern: "anyhow in handler",
            fix: "Use thiserror for typed errors with IntoResponse impl. anyhow loses error variant info.",
            line: 0,
        });
    }
    let has_blocking_justification = content.contains("// CPU-BOUND:")
        || content.contains("// IO-BOUND:")
        || content.contains("// BLOCKING_SYNC:");
    if patterns.get(6).is_some_and(|p| p.is_match(content)) && !has_blocking_justification {
        violations.push(AxumViolation {
            severity: AxumSeverity::P1Advisory,
            pattern: "spawn_blocking without justification",
            fix: "Add // CPU-BOUND:, // IO-BOUND:, or // BLOCKING_SYNC: <reason> comment to classify the work.",
            line: 0,
        });
    }

    let has_router = content.contains("Router::new()");
    if has_router {
        let has_body_limit =
            content.contains("DefaultBodyLimit::") || content.contains("RequestBodyLimitLayer");
        if !has_body_limit {
            violations.push(AxumViolation {
                severity: AxumSeverity::P1Advisory,
                pattern: "router without body limit",
                fix: "Add .layer(DefaultBodyLimit::max(2_000_000)) — unbounded body = DoS (CVE-2026-27729).",
                line: 0,
            });
        }
        if !content.contains("TimeoutLayer::") {
            violations.push(AxumViolation {
                severity: AxumSeverity::P1Advisory,
                pattern: "router without timeout",
                fix: "Add .layer(TimeoutLayer::new(Duration::from_secs(30))) — handlers without timeout exhaust threads.",
                line: 0,
            });
        }
        if !content.contains("TraceLayer::") {
            violations.push(AxumViolation {
                severity: AxumSeverity::P1Advisory,
                pattern: "router without tracing",
                fix: "Add .layer(TraceLayer::new_for_http()) — production handlers must emit request spans.",
                line: 0,
            });
        }
    }

    if patterns.get(13).is_some_and(|p| p.is_match(content)) && !content.contains("// EXPOSE:") {
        violations.push(AxumViolation {
            severity: AxumSeverity::P1Advisory,
            pattern: "bind 0.0.0.0 without comment",
            fix: "Add // EXPOSE: <reason> comment. 0.0.0.0 binds all interfaces — confirm intentional.",
            line: 0,
        });
    }

    // P2 warnings
    if patterns.get(11).is_some_and(|p| p.is_match(content))
        && !content.contains("#[instrument")
        && !content.contains("#[tracing::instrument")
    {
        violations.push(AxumViolation {
            severity: AxumSeverity::P2Warning,
            pattern: "handler without instrument",
            fix: "Add #[tracing::instrument(skip(state))] for structured logging with request context.",
            line: 0,
        });
    }

    violations
}

fn is_axum_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    content.contains("axum::")
        || content.contains("use axum")
        || content.contains("Router::new")
        || content.contains("FromRequest")
        || content.contains("IntoResponse")
        || content.contains("axum_extra")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_async_trait_on_from_request() {
        let code = "#[async_trait]\nimpl<S> FromRequestParts<S> for AuthUser {\n    type Rejection = AuthError;\n}";
        let v = detect("src/extractors.rs", code);
        assert!(v.iter().any(|x| x.pattern == "async_trait on FromRequest"));
    }

    #[test]
    fn detects_old_path_syntax() {
        let code = "use axum::Router;\nlet app = Router::new().get(\"/users/:id\", get_user);";
        let v = detect("src/routes.rs", code);
        assert!(v.iter().any(|x| x.pattern == "old path syntax /:id"));
    }

    #[test]
    fn detects_raw_json_value() {
        let code = "use axum::Json;\nfn handler(Json(payload): Json<serde_json::Value>) {}";
        let v = detect("src/handlers.rs", code);
        assert!(v.iter().any(|x| x.pattern == "Json<Value> without schema"));
    }

    #[test]
    fn detects_logging_in_into_response() {
        let code = "impl IntoResponse for MyError {\n    fn into_response(self) -> Response {\n        tracing::error!(\"err\");\n        StatusCode::INTERNAL_SERVER_ERROR.into_response()\n    }\n}";
        let v = detect("src/error.rs", code);
        assert!(v.iter().any(|x| x.pattern == "logging in IntoResponse"));
    }

    #[test]
    fn detects_repeated_layer_calls() {
        let code = "use axum::Router; let app = Router::new().layer(CorsLayer::new()).layer(TraceLayer::new()).layer(TimeoutLayer::new(d));";
        let v = detect("src/main.rs", code);
        assert!(v.iter().any(|x| x.pattern == "repeated .layer() calls"));
    }

    #[test]
    fn detects_router_without_body_limit() {
        let code = "use axum::Router;\nlet app = Router::new();";
        let v = detect("src/main.rs", code);
        assert!(v.iter().any(|x| x.pattern == "router without body limit"));
    }

    #[test]
    fn allows_router_with_full_layers() {
        let code = "use axum::{Router, extract::DefaultBodyLimit};\nlet app = Router::new().layer(DefaultBodyLimit::max(2_000_000)).layer(TimeoutLayer::new(d)).layer(TraceLayer::new_for_http());";
        let v = detect("src/main.rs", code);
        assert!(!v.iter().any(|x| x.pattern == "router without body limit"));
    }

    #[test]
    fn detects_bind_0_0_0_0_without_comment() {
        let code = "use axum::Router;\nlet addr = SocketAddr::from(([0, 0, 0, 0], 8080));";
        let v = detect("src/main.rs", code);
        assert!(
            v.iter()
                .any(|x| x.pattern == "bind 0.0.0.0 without comment")
        );
    }

    #[test]
    fn allows_bind_with_expose_comment() {
        let code = "// EXPOSE: production server\nuse axum::Router;\nlet addr = SocketAddr::from(([0, 0, 0, 0], 8080));";
        let v = detect("src/main.rs", code);
        assert!(
            !v.iter()
                .any(|x| x.pattern == "bind 0.0.0.0 without comment")
        );
    }

    #[test]
    fn non_axum_file_skipped() {
        let code = "fn main() { println!(\"hello\"); }";
        let v = detect("src/cli.rs", code);
        assert!(v.is_empty());
    }
}
