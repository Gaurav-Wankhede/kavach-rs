use super::*;
#[test]
fn test_file_types() {
    assert!(is_frontend_file("a.tsx"));
    assert!(is_backend_file("a.rs"));
    assert!(is_test_file("a_test.go"));
    assert!(is_test_file("src/gates/intent_tests.rs"));
    assert!(is_test_file("crates/x/src/foo/tests.rs"));
    assert!(is_test_file("tests/integration.rs"));
}
#[test]
fn test_allowlist_claude_rules_md() {
    // .md files in ~/.claude/ must be exempt — they document anti-patterns by name.
    assert!(is_allowlisted(
        "/Users/gauravwankhede/.claude/rules/04-anti-patterns.md"
    ));
    assert!(is_allowlisted("/Users/gauravwankhede/.claude/CLAUDE.md"));
    assert!(is_allowlisted(
        "/Users/gauravwankhede/.claude/skills/rust/SKILL.md"
    ));
}
#[test]
fn test_allowlist_does_not_exempt_rs_files() {
    assert!(!is_allowlisted("src/gates/intent.rs"));
}
#[test]
fn test_api_client_by_path() {
    assert!(is_api_client_file("src/api/users.ts"));
    assert!(is_api_client_file("src/services/auth.ts"));
    assert!(is_api_client_file("src/client/http.ts"));
    assert!(is_api_client_file("src/requests/payments.tsx"));
    assert!(is_api_client_file("src/endpoints/orders.ts"));
}
#[test]
fn test_api_client_by_name() {
    assert!(is_api_client_file("src/apiClient.ts"));
    assert!(is_api_client_file("src/authApi.ts"));
    assert!(is_api_client_file("src/soundbak.client.ts"));
    assert!(is_api_client_file("src/user.service.ts"));
}
#[test]
fn test_is_marker_inside_string_quoted_todo() {
    assert!(is_marker_inside_string(
        r#"            "todo"        => "[TODO]","#
    ));
}
#[test]
fn test_is_marker_inside_string_bare_todo_in_comment() {
    assert!(!is_marker_inside_string("    // TODO: implement this"));
}
#[test]
fn test_is_marker_inside_string_bare_fixme() {
    assert!(!is_marker_inside_string("    // FIXME: broken"));
}
#[test]
fn test_is_marker_inside_string_no_marker() {
    assert!(is_marker_inside_string("    let x = 42;"));
}
#[test]
fn test_api_client_excludes_ui() {
    assert!(!is_api_client_file("src/components/Button.tsx"));
    assert!(!is_api_client_file("src/hooks/useAuth.ts"));
    assert!(!is_api_client_file("src/utils/format.ts"));
    assert!(!is_api_client_file("src/pages/Home.tsx"));
    assert!(!is_api_client_file("server/handler.rs"));
}
#[test]
fn test_edge_worker_file_detection() {
    assert!(is_edge_worker_file(
        "packages/workers/edge-cache/src/index.ts"
    ));
    assert!(is_edge_worker_file("packages/workers/api/src/handler.ts"));
    assert!(is_edge_worker_file("src/durable-objects/counter.ts"));
    assert!(is_edge_worker_file("src/cf-workers/rate-limiter.ts"));
    assert!(is_edge_worker_file("src/cloudflare-workers/auth.ts"));
    assert!(is_edge_worker_file("src/ratelimit-do.ts"));
    assert!(is_edge_worker_file("src/counter.do.ts"));
    assert!(is_edge_worker_file("src/cache-worker.ts"));
    assert!(is_edge_worker_file("src/auth.worker.ts"));
    assert!(!is_edge_worker_file("src/components/Button.tsx"));
    assert!(!is_edge_worker_file("src/pages/Home.tsx"));
    assert!(!is_edge_worker_file("src/api/client.ts"));
}
#[test]
fn test_frontend_excludes_edge_workers() {
    assert!(!is_frontend_file(
        "packages/workers/edge-cache/src/ratelimit-do.ts"
    ));
    assert!(!is_frontend_file("src/durable-objects/counter.ts"));
    assert!(!is_frontend_file("src/auth-worker.ts"));
    assert!(is_frontend_file("src/components/Button.tsx"));
    assert!(is_frontend_file("src/pages/Home.ts"));
}
