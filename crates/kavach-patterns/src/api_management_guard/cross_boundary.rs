use super::patterns::PATTERNS;
use super::types::{ApiSeverity, ApiViolation};

pub(super) fn check_cross_boundary(
    content: &str,
    violations: &mut Vec<ApiViolation>,
    has_jwt_verify: bool,
) {
    if PATTERNS.get(17).is_some_and(|p| p.is_match(content))
        && !content.contains(".timeout(")
        && !content.contains("timeout:")
    {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "third-party SDK without timeout",
            fix: "Configure SDK timeout (5s-30s).",
            line: 0,
        });
    }
    if PATTERNS.get(18).is_some_and(|p| p.is_match(content)) {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "API credential literal in source",
            fix: "Move to env var / secrets manager.",
            line: 0,
        });
    }
    if PATTERNS.get(19).is_some_and(|p| p.is_match(content)) && !has_jwt_verify {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "JWT/PASETO decode without verify",
            fix: "Always verify signature + claims. OWASP API2.",
            line: 0,
        });
    }
}
