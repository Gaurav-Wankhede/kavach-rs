use super::patterns::PATTERNS;
use super::types::{ApiSeverity, ApiViolation};

pub(super) fn check_database_boundary(
    content: &str,
    violations: &mut Vec<ApiViolation>,
    has_tenant_filter: bool,
    has_rls_context: bool,
) {
    if PATTERNS.get(11).is_some_and(|p| p.is_match(content)) && !has_tenant_filter {
        violations.push(ApiViolation {
            severity: ApiSeverity::P0Block,
            pattern: "query without tenant filter",
            fix: "Add WHERE tenant_id = $1.",
            line: 0,
        });
    }
    if PATTERNS.get(16).is_some_and(|p| p.is_match(content)) && !has_rls_context {
        violations.push(ApiViolation {
            severity: ApiSeverity::P1Advisory,
            pattern: "transaction without RLS context",
            fix: "Add SET LOCAL app.tenant_id inside transaction.",
            line: 0,
        });
    }
}
