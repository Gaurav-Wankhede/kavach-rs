use super::patterns::PATTERNS;
use super::types::{ApiSeverity, ApiViolation};

pub(super) fn check_gateway_boundary(
    content: &str,
    violations: &mut Vec<ApiViolation>,
    has_aip_4222: bool,
) {
    if PATTERNS.get(7).is_some_and(|p| p.is_match(content)) {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "CORS wildcard origin",
            fix: "Use exact-match allow-list.",
            line: 0,
        });
    }
    if PATTERNS.get(8).is_some_and(|p| p.is_match(content)) && !has_aip_4222 {
        violations.push(ApiViolation {
            severity: ApiSeverity::P1Advisory,
            pattern: "gateway proxy missing AIP-4222 headers",
            fix: "Inject X-Request-Id, X-Request-Timestamp, X-Request-Platform.",
            line: 0,
        });
    }
}
