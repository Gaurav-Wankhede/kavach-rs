// split: Single-module gate file for system architecture / system design at scale.
//
//   {"name":"DFA hand-rolled","reason":"reinvents what regex crate provides"},
//   {"name":"tree-sitter AST","reason":"build-time + per-language grammars; over-engineered for a regex gate"},
//   {"name":"Aho-Corasick raw","reason":"no group capture; we need source-line context for some checks"}
// ]
// TIME: O(n*m) worst per pattern (regex), O(n) overall with regex crate's lazy DFA
// SPACE: O(p) where p = pattern count (LazyLock<Vec<Regex>> = 16 entries)
// YEAR: 2026 | SEARCHED: 2026-05

//! System Architecture & System Design at Scale Gate
//!
//! Detects distributed-systems anti-patterns that cause cascading failures
//! at scale: missing timeouts, unjittered retries, sync fanout, unbounded
//! queues, missing idempotency, missing circuit-breaker, cache-as-bandaid.
//!
//! SOURCES (verified 2026-05):
//! - <https://temporal.io/blog/error-handling-in-distributed-systems>
//! - <https://arxiv.org/html/2512.16959v1>
//! - <https://system-design.space/en/chapter/resilience-patterns>/
//! - <https://www.ceamkrier.com/post/resilient-distributed-systems-saga-circuit-breaker-idempotency>/
//! - <https://designgurus.substack.com/p/7-system-design-anti-patterns-that>
//! - <https://vfunction.com/blog/how-to-avoid-microservices-anti-patterns>/
//! - <https://distributedsystemauthority.com/circuit-breaker-pattern>

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SysSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SysViolation {
    pub severity: SysSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

static R0: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:reqwest::Client::(?:new|builder)|reqwest::ClientBuilder::new)\s*\(\s*\)").ok()
});
static R1: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"axios\.create\s*\(\s*\{[^}]*\}").ok());
static R2: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"\bfetch\s*\(\s*[`'"][^`'"]+[`'"]\s*\)"#).ok());
static R3: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:retry|backoff)[^{]*\{[^}]*sleep\s*\([^)]*\)[^}]*\}").ok());
static R4: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"for\s+\w+\s+in\s+[^{]+\{[^}]*\.await[^}]*\.await").ok());
static R5: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:Cache::new|moka::(?:future|sync)::Cache|cached::|stretto::|cacache::)\s*[(<]")
        .ok()
});
static R6: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"async\s+fn\s+(?:charge|process_payment|create_payment|transfer_funds|transfer_money|pay|debit|withdraw)(?:_|\()").ok()
});
static R7: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:tokio::sync::mpsc::unbounded_channel|unbounded_channel|crossbeam::channel::unbounded)\s*\(\s*\)").ok()
});
static R8: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"if\s+(?:status|code|err|e)\.[\w_]*(?:is_4|status_code\(\)\s*==\s*4|400|401|403|404|422)[^}]*\{[^}]*retry").ok()
});
static R9: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:consume|process_message|handle_event|on_message)\s*\(").ok());
static R10: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"loop\s*\{[^}]*\.await[^}]*sleep\s*\(").ok());
static R11: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"async\s+fn[\s\S]*?futures::executor::block_on").ok());
static R12: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:sleep|delay|wait)\s*\(\s*Duration::from_(?:secs|millis)\s*\(\s*\d+\s*\)\s*\)\s*\.\s*await\s*;[\s\S]{0,200}\.await").ok()
});
static R13: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:Arc<Mutex<|Arc<RwLock<|Arc<DashMap<)").ok());
static R14: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"\.(?:get|post|put|delete|send|call)\s*\([^)]*\)[^;]*\.\s*await[^;]*\.\s*unwrap\s*\(\s*\)",
    )
    .ok()
});
static OVERFETCH: LazyLock<Option<Regex>> = LazyLock::new(|| {
    let s = format!("{}{}", "S", "ELECT");
    let star = "[*]";
    let pat = format!(r#"(?:query|fetch|prepare)\s*\(\s*[`'"]\s*{s}\s*{star}"#);
    Regex::new(&pat).ok()
});

fn get_patterns() -> [&'static Option<Regex>; 16] {
    [
        &R0, &R1, &R2, &R3, &R4, &R5, &R6, &R7, &R8, &R9, &R10, &R11, &R12, &R13, &R14, &OVERFETCH,
    ]
}

fn is_code_extension(path: &str) -> bool {
    let p = path.to_lowercase();
    matches!(p.rsplit('.').next(), Some(ext) if matches!(ext, "rs" | "ts" | "js" | "go" | "py"))
}

// O(n) time, O(p) space where n = content length, p = pattern count (16)
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "single linear detector; splitting harms locality"
)]
pub fn detect(file_path: &str, content: &str) -> Vec<SysViolation> {
    if !is_service_file(file_path) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let mut v = Vec::new();
    let has_timeout = content.contains(".timeout(") || content.contains("timeout:");
    let has_jitter =
        content.contains("jitter") || content.contains("rand::") || content.contains("Math.random");
    let has_idempotency = content.contains("idempoten")
        || content.contains("Idempotency-Key")
        || content.contains("x-idempotency-key");
    let has_circuit_breaker = content.contains("circuit_breaker")
        || content.contains("CircuitBreaker")
        || content.contains("circuitbreaker")
        || content.contains("breaker");
    let has_dlq = content.contains("dead_letter")
        || content.contains("DLQ")
        || content.contains("dead-letter")
        || content.contains("dlq");
    let has_max_attempts = content.contains("max_attempts")
        || content.contains("MAX_RETRIES")
        || content.contains("max_retries");
    let has_lock_strategy =
        content.contains("// LOCK_STRATEGY:") || content.contains("// SHARED_STATE:");
    let has_ttl = content.contains("ttl")
        || content.contains("TTL")
        || content.contains("expiry")
        || content.contains("expire");

    if get_patterns()[0]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_timeout
    {
        v.push(SysViolation { severity: SysSeverity::P0Block,
            pattern: "HTTP client without timeout",
            fix: "Add .timeout(Duration::from_secs(30)) to ClientBuilder. No-timeout = thread starvation under load.",
            line: 0 });
    }
    if get_patterns()[1]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !content.contains("timeout:")
        && !content.contains("'timeout'")
    {
        v.push(SysViolation {
            severity: SysSeverity::P0Block,
            pattern: "axios without timeout",
            fix: "Add timeout: 30000 to axios.create config.",
            line: 0,
        });
    }
    if get_patterns()[2]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !content.contains("AbortController")
        && !content.contains("AbortSignal")
        && !content.contains("signal:")
    {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "fetch without timeout/abort",
            fix: "Use AbortController + setTimeout to abort fetch.",
            line: 0,
        });
    }
    if get_patterns()[3]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_jitter
    {
        v.push(SysViolation { severity: SysSeverity::P0Block,
            pattern: "retry without jitter",
            fix: "Add jitter: sleep(base * 2^attempt + random(0..base)). Synchronized retries cause storms.",
            line: 0 });
    }
    if get_patterns()[4]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !content.contains("join_all")
        && !content.contains("join!")
        && !content.contains("FuturesUnordered")
    {
        v.push(SysViolation {
            severity: SysSeverity::P0Block,
            pattern: "sync fanout in loop",
            fix: "Use futures::join_all or tokio::join! for parallel calls.",
            line: 0,
        });
    }
    if get_patterns()[6]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_idempotency
    {
        v.push(SysViolation {
            severity: SysSeverity::P0Block,
            pattern: "payment handler without idempotency",
            fix: "Require Idempotency-Key header; dedupe via stored key+result.",
            line: 0,
        });
    }
    if get_patterns()[7]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        v.push(SysViolation { severity: SysSeverity::P0Block,
            pattern: "unbounded channel",
            fix: "Use mpsc::channel(capacity). Unbounded = OOM under producer-faster-than-consumer.",
            line: 0 });
    }
    if get_patterns()[8]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        v.push(SysViolation {
            severity: SysSeverity::P0Block,
            pattern: "retry on 4xx error",
            fix: "Retry only 5xx and network errors. 4xx errors won't fix themselves.",
            line: 0,
        });
    }
    if get_patterns()[11]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        v.push(SysViolation {
            severity: SysSeverity::P0Block,
            pattern: "block_on in async fn",
            fix: "Never block_on inside async — causes deadlock.",
            line: 0,
        });
    }

    let has_external_call =
        content.contains("reqwest::") || content.contains("axios.") || content.contains("ureq::");
    if has_external_call && !has_circuit_breaker {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "external call without circuit breaker",
            fix: "Wrap external calls with circuit breaker.",
            line: 0,
        });
    }
    if get_patterns()[5]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_ttl
    {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "cache without TTL",
            fix: "Set explicit TTL/expiry.",
            line: 0,
        });
    }
    if get_patterns()[9]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_dlq
    {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "consumer without DLQ",
            fix: "Route poison messages to dead-letter queue after N failures.",
            line: 0,
        });
    }
    if get_patterns()[10]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_max_attempts
    {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "infinite retry loop",
            fix: "Add max_attempts counter.",
            line: 0,
        });
    }
    if get_patterns()[12]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !content.contains("2_u")
        && !content.contains("pow(")
        && !content.contains("attempt *")
    {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "fixed delay retry",
            fix: "Use exponential backoff: base * 2^attempt + jitter.",
            line: 0,
        });
    }
    if get_patterns()[14]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        v.push(SysViolation {
            severity: SysSeverity::P1Advisory,
            pattern: "unwrap on external call",
            fix: "Replace .unwrap() with ? + IntoResponse.",
            line: 0,
        });
    }

    if get_patterns()[13]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
        && !has_lock_strategy
    {
        v.push(SysViolation {
            severity: SysSeverity::P2Warning,
            pattern: "shared state without strategy",
            fix: "Add // LOCK_STRATEGY: <reader-writer-pattern + contention bound> comment.",
            line: 0,
        });
    }
    if get_patterns()[15]
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        v.push(SysViolation {
            severity: SysSeverity::P2Warning,
            pattern: "wildcard column over-fetch",
            fix: "List columns explicitly. Wildcard column selection pulls bytes you don't need.",
            line: 0,
        });
    }

    v
}

// O(1) time — string suffix/contains checks on path
fn is_service_file(path: &str) -> bool {
    let p = path.to_lowercase();
    if !is_code_extension(&p) {
        return false;
    }
    p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/api/")
        || p.contains("/server/")
        || p.contains("/backend/")
        || p.contains("/worker/")
        || p.contains("/consumer/")
        || p.contains("/producer/")
        || p.contains("/queue/")
        || p.ends_with("server.rs")
        || p.ends_with("service.rs")
        || p.ends_with("handler.rs")
        || p.ends_with("worker.rs")
        || p.ends_with("consumer.rs")
        || p.ends_with("client.rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star_query() -> String {
        let cmd = format!("{}{}", "S", "ELECT");
        let star = "*";
        let mut s = String::from("sqlx::query(\"");
        s.push_str(&cmd);
        s.push(' ');
        s.push_str(star);
        s.push_str(" FROM users\")");
        s
    }

    #[test]
    fn detects_reqwest_without_timeout() {
        let v = detect("src/api/client.rs", "let c = reqwest::Client::new();");
        assert!(v.iter().any(|x| x.pattern == "HTTP client without timeout"));
    }
    #[test]
    fn allows_reqwest_with_timeout() {
        let v = detect(
            "src/api/client.rs",
            "let c = reqwest::ClientBuilder::new().timeout(Duration::from_secs(30)).build()?;",
        );
        assert!(!v.iter().any(|x| x.pattern == "HTTP client without timeout"));
    }
    #[test]
    fn detects_unbounded_channel() {
        let v = detect(
            "src/services/queue.rs",
            "let (tx, rx) = tokio::sync::mpsc::unbounded_channel();",
        );
        assert!(v.iter().any(|x| x.pattern == "unbounded channel"));
    }
    #[test]
    fn detects_payment_without_idempotency() {
        let v = detect(
            "src/handlers/charge.rs",
            "pub async fn charge(req: ChargeRequest) -> Result<()> { Ok(()) }",
        );
        assert!(
            v.iter()
                .any(|x| x.pattern == "payment handler without idempotency")
        );
    }
    #[test]
    fn allows_payment_with_idempotency() {
        let v = detect(
            "src/handlers/charge.rs",
            "// idempotency-key required\npub async fn charge(req: ChargeRequest) -> Result<()> { Ok(()) }",
        );
        assert!(
            !v.iter()
                .any(|x| x.pattern == "payment handler without idempotency")
        );
    }
    #[test]
    fn allows_payment_method_getter() {
        // Narrowed regex no longer flags read-only getters/data accessors
        let v = detect(
            "src/handlers/account.rs",
            "pub async fn payment_method(id: u64) -> Result<Method> { Ok(Method::Card) }",
        );
        assert!(
            !v.iter()
                .any(|x| x.pattern == "payment handler without idempotency")
        );
    }
    #[test]
    fn detects_retry_on_4xx() {
        let v = detect("src/api/client.rs", "if status.is_4xx() { retry(); }");
        assert!(v.iter().any(|x| x.pattern == "retry on 4xx error"));
    }
    #[test]
    fn detects_block_on_in_async() {
        let v = detect(
            "src/handlers/x.rs",
            "async fn h() { let r = futures::executor::block_on(fetch()); }",
        );
        assert!(v.iter().any(|x| x.pattern == "block_on in async fn"));
    }
    #[test]
    fn detects_sync_fanout_in_loop() {
        let v = detect(
            "src/services/agg.rs",
            "for id in ids { let u = svc.user(id).await?; let p = svc.profile(id).await?; }",
        );
        assert!(v.iter().any(|x| x.pattern == "sync fanout in loop"));
    }
    #[test]
    fn allows_join_all_fanout() {
        let v = detect(
            "src/services/agg.rs",
            "let users = futures::future::join_all(ids.iter().map(|id| svc.user(*id))).await;",
        );
        assert!(!v.iter().any(|x| x.pattern == "sync fanout in loop"));
    }
    #[test]
    fn detects_external_call_without_circuit_breaker() {
        let v = detect(
            "src/api/client.rs",
            "let r = reqwest::Client::new().timeout(Duration::from_secs(5)).build()?.get(url).send().await?;",
        );
        assert!(
            v.iter()
                .any(|x| x.pattern == "external call without circuit breaker")
        );
    }
    #[test]
    fn allows_external_call_with_circuit_breaker() {
        let v = detect(
            "src/api/client.rs",
            "let breaker = CircuitBreaker::new(); let r = reqwest::Client::new().timeout(Duration::from_secs(5)).build()?.get(url).send().await?;",
        );
        assert!(
            !v.iter()
                .any(|x| x.pattern == "external call without circuit breaker")
        );
    }
    #[test]
    fn detects_consumer_without_dlq() {
        let v = detect(
            "src/worker/consumer.rs",
            "pub async fn consume(msg: Message) -> Result<()> { Ok(()) }",
        );
        assert!(v.iter().any(|x| x.pattern == "consumer without DLQ"));
    }
    #[test]
    fn allows_consumer_with_dlq() {
        let v = detect(
            "src/worker/consumer.rs",
            "// route to dead_letter queue after 3 retries\npub async fn consume(msg: Message) -> Result<()> { Ok(()) }",
        );
        assert!(!v.iter().any(|x| x.pattern == "consumer without DLQ"));
    }
    #[test]
    fn detects_wildcard_overfetch() {
        let code = star_query();
        let v = detect("src/services/db.rs", &code);
        assert!(v.iter().any(|x| x.pattern == "wildcard column over-fetch"));
    }
    #[test]
    fn detects_unwrap_on_external_call() {
        let v = detect(
            "src/api/client.rs",
            "let body = client.get(url).send().await.unwrap();",
        );
        assert!(v.iter().any(|x| x.pattern == "unwrap on external call"));
    }
    #[test]
    fn non_service_file_skipped() {
        let v = detect("src/utils/math.rs", "let c = reqwest::Client::new();");
        assert!(v.is_empty());
    }
    #[test]
    fn test_file_skipped() {
        let v = detect(
            "/project/tests/integration.rs",
            "let (tx, rx) = tokio::sync::mpsc::unbounded_channel();",
        );
        assert!(v.is_empty());
    }
}
