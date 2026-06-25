use super::types::{SysSeverity, SysViolation};
use super::util::{get_patterns, is_service_file};

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
