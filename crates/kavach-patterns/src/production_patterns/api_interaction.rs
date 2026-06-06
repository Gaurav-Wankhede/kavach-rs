//! API interaction anti-patterns.

use super::types::{Severity, mk};

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    vec![
        (
            mk(r"(?:reqwest|hyper)::Client::builder\(\)"),
            "CHECK_TIMEOUT",
            "HTTP client builder — verify .timeout() is set",
            Severity::P2Medium,
        ),
        (
            mk(r"\.send\(\)\.await\?;"),
            "NO_RETRY",
            "HTTP call — add retry with exponential backoff",
            Severity::P1High,
        ),
        (
            mk(r"(?:http|https)://[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}"),
            "HARDCODED_URL",
            "Hardcoded URL — use env var or config",
            Severity::P1High,
        ),
        (
            mk(r"(?s)(?:loop|while)\s*\{[^}]*\.send\(\)\.await"),
            "NO_CIRCUIT",
            "Retry loop without circuit breaker — add failure threshold",
            Severity::P1High,
        ),
        (
            mk(r"\.send\(\)\.await\?\.bytes\(\)"),
            "NO_STATUS_CHECK",
            "Response status not checked — verify .status().is_success()",
            Severity::P1High,
        ),
        (
            mk(r"\.bytes\(\)\.await\?"),
            "NO_STREAM",
            "Full body load — use .bytes_stream() for large responses",
            Severity::P2Medium,
        ),
        (
            mk(r"Client::new\(\)\.(?:get|post|put|delete)\("),
            "NO_USER_AGENT",
            "HTTP without User-Agent — add .header(USER_AGENT)",
            Severity::P2Medium,
        ),
        (
            mk(r"\.query\(\s*&\[.*(?:token|key|secret|password)"),
            "SECRET_IN_QUERY",
            "Sensitive data in query params — use headers or body",
            Severity::P0Critical,
        ),
    ]
}
