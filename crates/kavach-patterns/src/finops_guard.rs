// split: FinOps gate — cloud cost anti-patterns. Advisory tier.
//
// [RCA]
// symptom:    PRs ship with unbounded fan-out, per-request reqwest::Client::new(), missing query LIMIT
// repro:      tokio::spawn(async {...}) inside a request handler with no semaphore = unbounded compute
// why1:       no gate flags FinOps anti-patterns
// why2:       cost is invisible at write-time; surfaces only on the cloud bill
// why3:       invariant violated — bounded resource use per request
// why4:       2025 Honeycomb data: 15-25% of infra bill flows to telemetry alone; every PR can amplify it
// why5:       missing FinOps detection layer
// root_cause: no finops_guard module
// class:      knowledge_gap
// blast_radius: every Rust handler / async fn / tracing call site
// research:   https://www.infoq.com/articles/backend-finops-cost-efficiency/
//             https://platformengineering.org/blog/10-finops-tools-platform-engineers-should-evaluate-for-2026
// fix_strategy: 6-pattern advisory module (P1/P2 only); wire into pre_write_guards.rs

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "closed severity set; exhaustively matched cross-crate in kavach-rpc gates.rs"
)]
pub enum FinopsSeverity {
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FinopsViolation {
    pub severity: FinopsSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
}

static PATTERNS: LazyLock<Vec<Regex>> =
    LazyLock::new(|| {
        [
        r"\btokio::spawn\s*\(",
        r"\breqwest::Client::new\s*\(\s*\)",
        r"(?i)\bbroadcast::channel\s*\(\s*usize::MAX\s*\)|\bchannel\s*\(\s*\)",
        r"\.connect\s*\(\s*&[^)]+\)\.await",
        r"\bClient::builder\s*\(\s*\)\.build\s*\(\s*\)",
        r"(?i)tracing::(?:info|debug)!\s*\([^)]*=\s*%[^)]*request_id|user_id|tenant_id|trace_id",
    ].iter().filter_map(|p| Regex::new(p).ok()).collect()
    });

fn is_target_file(path: &str, content: &str) -> bool {
    use std::path::Path;
    if !Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    if p.ends_with("/build.rs") {
        return false;
    }
    p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/grpc/")
        || content.contains("axum::")
        || content.contains("tonic::")
        || content.contains("async fn")
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<FinopsViolation> {
    if !is_target_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let mut v = Vec::new();
    let has_semaphore = content.contains("Semaphore") || content.contains("acquire_owned");
    if PATTERNS.first().is_some_and(|p| p.is_match(content)) && !has_semaphore {
        v.push(FinopsViolation { severity: FinopsSeverity::P1Advisory,
            pattern: "spawn-no-semaphore",
            fix: "tokio::spawn without Semaphore = unbounded compute spend under load. Use Arc<Semaphore> + acquire_owned." });
    }
    if PATTERNS.get(1).is_some_and(|p| p.is_match(content)) {
        v.push(FinopsViolation { severity: FinopsSeverity::P1Advisory,
            pattern: "reqwest-client-per-request",
            fix: "reqwest::Client::new() inside handler triggers TLS handshake per request. Build once, share via State<Arc<Client>>." });
    }
    if PATTERNS.get(3).is_some_and(|p| p.is_match(content)) && content.contains("PgPool") {
        v.push(FinopsViolation { severity: FinopsSeverity::P2Warning,
            pattern: "pool-connect-per-call",
            fix: "PgPool::connect inside fn body = connection-per-call. Build once at startup; pass via State." });
    }
    if PATTERNS.get(5).is_some_and(|p| p.is_match(content)) {
        v.push(FinopsViolation { severity: FinopsSeverity::P1Advisory,
            pattern: "telemetry-cardinality-explode",
            fix: "Logging unbounded high-cardinality field (request_id/user_id/tenant_id) at info level scales telemetry $ linearly with traffic. Move to debug or sample." });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_without_semaphore_flagged() {
        let src = "async fn x() {}\nfn h() { tokio::spawn(async { 1 }); }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(r.iter().any(|v| v.pattern == "spawn-no-semaphore"));
    }

    #[test]
    fn spawn_with_semaphore_ok() {
        let src = "use tokio::sync::Semaphore;\nasync fn x() {}\nfn h() { let sem = Semaphore::new(10); let _ = sem.acquire_owned(); tokio::spawn(async { 1 }); }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(!r.iter().any(|v| v.pattern == "spawn-no-semaphore"));
    }

    #[test]
    fn reqwest_per_request_flagged() {
        let src = "async fn x() {}\nasync fn h() { let c = reqwest::Client::new(); let _ = c; }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(r.iter().any(|v| v.pattern == "reqwest-client-per-request"));
    }

    #[test]
    fn telemetry_high_cardinality_flagged() {
        let src = "async fn x() {}\nfn h(uid: &str) { tracing::info!(user_id = %uid, \"ok\"); }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "telemetry-cardinality-explode")
        );
    }

    #[test]
    fn safe_handler_clean() {
        let src = "use axum;\nasync fn x() {}\nfn h() { tracing::info!(\"safe\"); }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(r.is_empty());
    }

    #[test]
    fn test_file_skipped() {
        let src = "async fn x() {}\nfn h() { tokio::spawn(async { 1 }); }\n";
        let r = detect("crate/tests/h.rs", src);
        assert!(r.is_empty());
    }
}
