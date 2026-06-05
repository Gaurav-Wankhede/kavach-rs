use super::patterns::PATTERNS;
use super::types::{ApiSeverity, ApiViolation};

pub(super) fn check_frontend_boundary(
    content: &str,
    violations: &mut Vec<ApiViolation>,
    has_authfetch: bool,
) {
    if PATTERNS.first().is_some_and(|p| p.is_match(content)) && !has_authfetch {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "frontend bare fetch()",
            fix: "Use authFetch/apiClient wrapper. Bare fetch() bypasses auth + AIP-4222 trace.",
            line: 0,
        });
    }
    if PATTERNS.get(1).is_some_and(|p| p.is_match(content)) {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "hardcoded API URL in frontend",
            fix: "Use env-configured base URL.",
            line: 0,
        });
    }
}
