//! Tests for architecture guard.

use super::*;

#[test]
fn detects_scale_patterns() {
    let code = "fn setup() { let r = horizontal_scale(config); }";
    let f = detect("src/infra.rs", code);
    assert!(!f.is_empty());
    assert_eq!(f[0].scope, ArchScope::Scale);
}

#[test]
fn detects_cache_patterns() {
    let code = "use moka::sync::Cache; let c = distributed_cache::new();";
    let f = detect("src/cache.rs", code);
    assert!(!f.is_empty());
    assert!(f.iter().any(|x| x.scope == ArchScope::Cache));
}

#[test]
fn detects_messaging_patterns() {
    let code = "let q = message_queue::connect(); pub_sub.publish(msg);";
    let f = detect("src/events.rs", code);
    assert!(!f.is_empty());
    assert!(f.iter().any(|x| x.scope == ArchScope::Messaging));
}

#[test]
fn detects_data_patterns() {
    let code = "impl cqrs::Handler { fn handle(&self) { event_sourcing::apply(e); } }";
    let f = detect("src/domain.rs", code);
    assert!(!f.is_empty());
    assert!(f.iter().any(|x| x.scope == ArchScope::Data));
}

#[test]
fn detects_service_patterns() {
    let code = "let cb = circuit_breaker::new(); rate_limiter.check(req);";
    let f = detect("src/resilience.rs", code);
    assert!(!f.is_empty());
    assert!(f.iter().any(|x| x.scope == ArchScope::Service));
}

#[test]
fn skips_test_files() {
    let code = "let r = horizontal_scale(config);";
    let f = detect("src/tests/infra.rs", code);
    assert!(f.is_empty());
}

#[test]
fn skips_patterns_crate() {
    let code = "let r = horizontal_scale(config);";
    let f = detect("kavach-patterns/src/arch.rs", code);
    assert!(f.is_empty());
}

#[test]
fn has_arch_comment_valid() {
    let code = "// ARCH: distributed_cache\nfn foo() {}";
    assert!(has_arch_comment(code));
}

#[test]
fn has_arch_comment_missing() {
    let code = "fn foo() { distributed_cache(); }";
    assert!(!has_arch_comment(code));
}

#[test]
fn count_fields_all_present() {
    let code = r"
// ARCH: distributed_cache
// SCOPE: cache
// CAP: AP
// QPS: 10000 | PEAK: 3x
// STORAGE: 10GB
// FAILURE_MODE: stale reads
// TRADEOFF: consistency
// SEARCHED: 2026-04
// REFERENCE: https://example.com
fn foo() {}
";
    assert_eq!(count_arch_fields(code), 9);
}

#[test]
fn count_fields_partial() {
    let code = "// ARCH: x\n// SCOPE: cache\nfn foo() {}";
    assert_eq!(count_arch_fields(code), 2);
}

#[test]
fn check_allows_no_patterns() {
    let code = "fn simple() { let x = 1; }";
    let outcome = check("src/lib.rs", code, false);
    assert_eq!(outcome, ArchGuardOutcome::Allow);
}

#[test]
fn check_allows_with_skill_invoked() {
    let code = "let c = distributed_cache::new();";
    let outcome = check("src/cache.rs", code, true);
    assert_eq!(outcome, ArchGuardOutcome::Allow);
}

#[test]
fn check_allows_with_complete_comment() {
    let code = r"
// ARCH: distributed_cache
// SCOPE: cache
// CAP: AP
// QPS: 10000
// STORAGE: 10GB
// FAILURE_MODE: stale reads
// TRADEOFF: consistency
// SEARCHED: 2026-04
// REFERENCE: https://example.com
let c = distributed_cache::new();
";
    let outcome = check("src/cache.rs", code, false);
    assert_eq!(outcome, ArchGuardOutcome::AllowWithComment);
}

#[test]
fn check_blocks_without_comment() {
    let code = "let c = distributed_cache::new();";
    let outcome = check("src/cache.rs", code, false);
    assert!(matches!(outcome, ArchGuardOutcome::Block(_)));
}

#[test]
fn check_blocks_incomplete_comment() {
    let code = "// ARCH: x\nlet c = distributed_cache::new();";
    let outcome = check("src/cache.rs", code, false);
    assert!(matches!(outcome, ArchGuardOutcome::Block(_)));
}

#[test]
fn advise_returns_message() {
    let code = "let c = distributed_cache::new();";
    let msg = advise("src/cache.rs", code);
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("ARCH_ADVISORY"));
}
