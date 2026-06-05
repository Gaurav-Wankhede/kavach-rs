use super::backend_flags::BackendFlags;
use super::patterns::PATTERNS;
use super::types::{ApiSeverity, ApiViolation};
use regex::Regex;

pub(super) fn check_backend_boundary(
    content: &str,
    violations: &mut Vec<ApiViolation>,
    flags: BackendFlags,
) {
    let version_re = Regex::new(r#"["']/v\d+/"#).ok();
    let has_versioned = version_re.as_ref().is_some_and(|r| r.is_match(content));
    if PATTERNS.get(2).is_some_and(|p| p.is_match(content)) && !has_versioned {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "route without version prefix",
            fix: "Prefix routes with /v1/, /v2/.",
            line: 0,
        });
    }
    if PATTERNS.get(3).is_some_and(|p| p.is_match(content)) && !flags.has_pagination() {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "list endpoint without pagination",
            fix: "Add limit + cursor params.",
            line: 0,
        });
    }
    if PATTERNS.get(4).is_some_and(|p| p.is_match(content)) {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "untyped JSON body",
            fix: "Use typed DTO with #[derive(Deserialize, Validate)].",
            line: 0,
        });
    }
    if PATTERNS.get(5).is_some_and(|p| p.is_match(content)) {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "DB row leaked in API response",
            fix: "Map to a Response DTO. OWASP API3.",
            line: 0,
        });
    }
    if PATTERNS.get(12).is_some_and(|p| p.is_match(content)) && !flags.has_rate_limit() {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "router without rate limit",
            fix: "Add tower_governor::GovernorLayer.",
            line: 0,
        });
    }
    if PATTERNS.get(6).is_some_and(|p| p.is_match(content)) && !flags.has_openapi() {
        violations.push(ApiViolation {
            severity: ApiSeverity::P1Advisory,
            pattern: "handler without OpenAPI annotation",
            fix: "Add #[utoipa::path(...)].",
            line: 0,
        });
    }
    let id_match_in_real_code = PATTERNS.get(14).is_some_and(|p| {
        p.find_iter(content).any(|m| {
            let start = m.start();
            let line_start = content
                .get(..start)
                .and_then(|s| s.rfind('\n'))
                .map_or(0, |i| i.saturating_add(1));
            let line_prefix = content.get(line_start..start).map_or("", str::trim_start);
            !line_prefix.starts_with("//") && !line_prefix.starts_with('*')
        })
    });
    if id_match_in_real_code {
        violations.push(ApiViolation {
            severity: ApiSeverity::P1Advisory,
            pattern: "auto-increment ID exposed",
            fix: "Use UUID/ULID. OWASP API1: BOLA.",
            line: 0,
        });
    }
    if PATTERNS.get(13).is_some_and(|p| p.is_match(content)) && !flags.has_problem_details() {
        violations.push(ApiViolation {
            severity: ApiSeverity::P1Advisory,
            pattern: "non-standard error response",
            fix: "Use RFC 9457 Problem Details.",
            line: 0,
        });
    }
    if PATTERNS.get(15).is_some_and(|p| p.is_match(content)) && !flags.has_idempotency_key() {
        violations.push(ApiViolation {
            severity: ApiSeverity::P1Advisory,
            pattern: "POST/PUT without idempotency key",
            fix: "Accept Idempotency-Key header.",
            line: 0,
        });
    }
}
