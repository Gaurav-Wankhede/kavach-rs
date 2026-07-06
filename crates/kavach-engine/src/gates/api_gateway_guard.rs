//! API Gateway Guard — pre-write gate for handler/route files.
//! P0 violations (missing gateway layer) = HARD BLOCK.
//! P1 violations (protocol leakage, missing aggregation) = advisory.

use std::fmt::Write as _;

use kavach_patterns::api_gateway::{self, Severity, Violation};

/// Check for P0 violations (`MissingGatewayLayer`).
/// Returns `Some(block_message)` if blocked, None if allowed.
pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    let violations = api_gateway::detect(file_path, content);
    let p0: Vec<&Violation> = violations
        .iter()
        .filter(|v| v.severity == Severity::P0Block)
        .collect();

    if p0.is_empty() {
        return None;
    }

    let mut msg = String::from("[API_GATEWAY] missing gateway layer -> add the gateway (items below) -> retry.\n\n");
    for v in &p0 {
        writeln!(msg, "  file: {file_path}").ok();
        writeln!(msg, "  violation: {}", v.message).ok();
        writeln!(msg, "  fix: {}\n", v.fix).ok();
    }
    msg.push_str("REQUIRED: Add auth/rate-limit middleware before this write can proceed.\n\n");
    // BUGFIX: was a literal `{search_year}` placeholder that never interpolated —
    // resolve the live year so the research query is always current.
    let year = crate::gates::directive_cache::current_year();
    writeln!(
        msg,
        "RESEARCH: WebSearch \"api gateway middleware patterns {year}\""
    )
    .ok();
    msg.push_str("SKILL: Invoke `arch` skill for gateway layer design.\n");
    msg.push_str("FIX: Add auth layer before routes. Use tower middleware for rate limiting.");
    Some(msg)
}

/// Format P1 advisory for additionalContext injection.
pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    let violations = api_gateway::detect(file_path, content);
    let p1: Vec<&Violation> = violations
        .iter()
        .filter(|v| v.severity == Severity::P1Advisory)
        .collect();

    if p1.is_empty() {
        return None;
    }

    let mut msg = String::from("[API_GATEWAY_ADVISORY]\n");
    for v in &p1 {
        let kind_str = match v.kind {
            api_gateway::ViolationKind::ProtocolLeakage => "ProtocolLeakage",
            api_gateway::ViolationKind::MissingAggregation => "MissingAggregation",
            api_gateway::ViolationKind::MissingGatewayLayer => "MissingGatewayLayer",
        };
        writeln!(msg, "  {}: {}", kind_str, v.message).ok();
    }
    msg.push_str("  skill: web-stack (API Gateway section)\n");
    Some(msg)
}

#[cfg(test)]
#[path = "api_gateway_guard_test.rs"]
mod tests;
