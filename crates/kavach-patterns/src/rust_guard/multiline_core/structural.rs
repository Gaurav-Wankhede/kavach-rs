//! Structural multiline arms (indices 11,12,20-44): expect/clone, serde-default,
//! empty-body, status-code, encapsulation, abstraction, validation, router
//! protocol/body-limit, lock-across-await, HTTP timeout, DSA, god-class.
//!
//! Simple `if m(idx)` arms are data-driven via `SIMPLE`; the context-sensitive
//! arms (counts, router combinations, validation/http-client content checks) stay
//! explicit. Splitting the table out is the canonical `too_many_lines` remedy.
//! SOURCE: <https://rust-lang.github.io/rust-clippy/master/index.html> (`too_many_lines`)
use crate::severity::{Severity, Violation};
use regex::Regex;

fn one(v: &mut Vec<Violation>, sev: Severity, pat: &str, fix: &str) {
    v.push(Violation::new(sev, pat, fix, 0));
}

/// `(regex index, severity, pattern, fix)` for arms that fire on a single match.
const SIMPLE: &[(usize, Severity, &str, &str)] = {
    use Severity::{P0Block, P1Advisory};
    &[
        (
            20,
            P0Block,
            "#[serde(default)] on bool",
            "Use Option<bool> — serde default on bool creates privilege escalation vector",
        ),
        (
            21,
            P0Block,
            "empty function body",
            "Implement the function — empty bodies silently pass as complete",
        ),
        (
            22,
            P0Block,
            "200 OK with error",
            "Return proper 4xx/5xx status code — 200 with error body breaks clients",
        ),
        (
            23,
            P0Block,
            "500 for validation",
            "Use 400/422 for validation errors — 500 indicates server bug, not bad input",
        ),
        (
            11,
            P0Block,
            "expect-generic",
            "Replace .expect() with ? or map_err for structured errors",
        ),
        (
            25,
            P1Advisory,
            "Vec without capacity hint",
            "Use Vec::with_capacity(n) when size is known — avoids repeated reallocations",
        ),
        (
            26,
            P1Advisory,
            "concrete type in param",
            "Accept &[T] instead of &Vec<T>, &str instead of &String — trait-based params",
        ),
        (
            27,
            P1Advisory,
            "manual type dispatch",
            "Use trait objects or generics instead of downcasting — let the type system dispatch",
        ),
        (
            29,
            P1Advisory,
            "bool parameter",
            "Replace bool param with enum — foo(true) is unclear, foo(Mode::Fast) is self-documenting",
        ),
        (
            36,
            P0Block,
            "lock held across await",
            "Release lock before .await — use tokio::sync::Mutex or scope the guard",
        ),
        (
            37,
            P1Advisory,
            "HTTP call without timeout",
            "Wrap with tokio::time::timeout() — unbounded awaits cause thread starvation",
        ),
        (
            39,
            P0Block,
            "linear search in loop",
            "Use HashSet for O(1) lookup instead of Vec::contains() in a loop",
        ),
        (
            40,
            P1Advisory,
            "string allocation in loop",
            "Pre-allocate with String::with_capacity() or use write! macro",
        ),
        (
            41,
            P1Advisory,
            "unbounded push in handler",
            "Limit collection size in handlers — unbounded allocation is a DoS vector (CVE-2026-26061)",
        ),
        (
            43,
            P1Advisory,
            "chatty sequential awaits",
            "Use tokio::join! or futures::join! for parallel calls — sequential awaits multiply latency",
        ),
    ]
};

pub(super) fn scan(r: &[Regex], content: &str, v: &mut Vec<Violation>) {
    use Severity::{P0Block, P1Advisory};
    let m = |idx: usize| r.get(idx).is_some_and(|re| re.is_match(content));
    let count = |idx: usize| r.get(idx).map_or(0, |re| re.find_iter(content).count());

    for &(idx, sev, pat, fix) in SIMPLE {
        if m(idx) {
            one(v, sev, pat, fix);
        }
    }

    if count(12) > 10 {
        one(
            v,
            Severity::P2Warning,
            "excessive-clone",
            "Borrow instead of cloning. Invoke /rust for borrow patterns",
        );
    }
    if count(24) > 3 {
        one(
            v,
            P1Advisory,
            "pub fields (encapsulation)",
            "Make fields private, add constructor returning Result, use accessor methods",
        );
    }
    if m(28)
        && !content.contains("validate")
        && !content.contains("check_")
        && !content.contains("verify")
        && !content.contains("is_valid")
    {
        one(
            v,
            P1Advisory,
            "handler without input validation",
            "Validate input at API boundary — use validator crate or manual checks before processing",
        );
    }
    let has_router = m(35);
    if has_router && !m(30) && !m(31) && !m(32) && !m(33) && !m(34) {
        one(
            v,
            P1Advisory,
            "REST-only router",
            "Consider WebSocket, SSE, GraphQL, gRPC, or HTTP/3 — REST is not the only protocol",
        );
    }
    if has_router && !m(38) {
        one(
            v,
            P1Advisory,
            "router without body limit",
            "Add .layer(DefaultBodyLimit::max(bytes)) — unbounded body = DoS (CVE-2026-27729)",
        );
    }
    let has_http_client = content.contains("ClientBuilder") || content.contains("Client::new");
    if has_http_client && !m(42) {
        one(
            v,
            P0Block,
            "HTTP client no timeout",
            "Add .timeout(Duration::from_secs(30)) to ClientBuilder — no-timeout clients cause cascading failures",
        );
    }
    if count(44) > 15 {
        one(
            v,
            P1Advisory,
            "god class (too many functions)",
            "Split into smaller modules — >15 functions suggests mixed responsibilities",
        );
    }
}
