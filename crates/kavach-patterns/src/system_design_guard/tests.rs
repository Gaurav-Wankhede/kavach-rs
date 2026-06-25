use crate::system_design_guard::{detect, SysViolation};

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
