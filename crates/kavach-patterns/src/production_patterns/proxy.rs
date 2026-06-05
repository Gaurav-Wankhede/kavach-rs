//! Proxy/gateway anti-patterns.

use super::types::{Severity, mk};

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    vec![
        (
            mk(r"(?:X-Forwarded-For|X-Real-IP).*(?:\.parse|\.to_string)"),
            "TRUST_FORWARD",
            "Trusting X-Forwarded headers — validate against known proxies",
            Severity::P1High,
        ),
        (
            mk(r"(?:reqwest|hyper)::get\s*\(\s*\w+\s*\)"),
            "SSRF_UNVALIDATED",
            "HTTP to user URL — validate against allowlist",
            Severity::P0Critical,
        ),
        (
            mk(r"Router::new\(\)\.route\("),
            "CHECK_ROUTER_LAYERS",
            "Router — verify RateLimitLayer and DefaultBodyLimit",
            Severity::P2Medium,
        ),
        (
            mk(r"WebSocketUpgrade"),
            "CHECK_WS_AUTH",
            "WebSocket upgrade — verify auth before upgrade",
            Severity::P1High,
        ),
        (
            mk(r"(?:async\s+fn|pub\s+async\s+fn)\s+\w+_handler\s*\("),
            "CHECK_REQUEST_ID",
            "Handler — verify request_id/trace_id for tracing",
            Severity::P2Medium,
        ),
    ]
}
