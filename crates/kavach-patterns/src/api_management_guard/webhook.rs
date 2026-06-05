use super::patterns::PATTERNS;
use super::types::{ApiSeverity, ApiViolation};

pub(super) fn check_webhook_boundary(
    content: &str,
    violations: &mut Vec<ApiViolation>,
    has_signature_verify: bool,
    has_timestamp_window: bool,
) {
    if PATTERNS.get(9).is_some_and(|p| p.is_match(content)) && !has_signature_verify {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "webhook without signature verification",
            fix: "Verify signature via SDK.",
            line: 0,
        });
    }
    if PATTERNS.get(10).is_some_and(|p| p.is_match(content)) && !has_timestamp_window {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "webhook without replay window",
            fix: "Reject events older than 5 min via tolerance window.",
            line: 0,
        });
    }
}
