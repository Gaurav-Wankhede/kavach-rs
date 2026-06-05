//! `FILE_MIXED_CONCERNS` + `HANDLER_MONOLITH` P0 block tests + escape hatches.
use super::super::check;

#[test]
fn should_block_mixed_concerns_violation_over_200_lines() {
    let body = "pub struct Svc {}\nimpl Svc {}\npub async fn handler() {}\n".repeat(70);
    let msg = check("src/services/registration.rs", &body);
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("FILE_MIXED_CONCERNS"));
}

#[test]
fn should_allow_mixed_concerns_under_200_lines() {
    let body = "pub struct Svc {}\nimpl Svc {}\npub async fn handler() {}\n".repeat(5);
    assert!(check("src/services/registration.rs", &body).is_none());
}

#[test]
fn should_allow_mixed_concerns_with_split_escape_hatch() {
    let mut body = "// split: intentional oversized-file, legacy\n".to_owned();
    body.push_str(&"pub struct Svc {}\nimpl Svc {}\npub async fn handler() {}\n".repeat(70));
    assert!(check("src/services/registration.rs", &body).is_none());
}

#[test]
fn should_not_block_orchestrator_with_mixed_concerns() {
    let body = "pub struct Svc {}\nimpl Svc {}\npub async fn handler() {}\n".repeat(70);
    if let Some(m) = check("src/app.rs", &body) {
        assert!(!m.contains("FILE_MIXED_CONCERNS"));
    }
}

#[test]
fn should_block_handler_oversized_in_middleware_dir() {
    let mut body =
        "pub async fn auth_middleware() {}\npub async fn context_middleware() {}\n".to_owned();
    body.push_str(&"fn helper() {}\n".repeat(100));
    let msg = check("src/middleware/request_context_middleware.rs", &body);
    assert!(msg.is_some());
    assert!(msg.unwrap_or_default().contains("HANDLER_MONOLITH"));
}

#[test]
fn should_block_handler_oversized_in_any_directory() {
    let body = "pub async fn foo() {}\npub async fn bar() {}\n".repeat(52);
    assert!(check("src/services/registration.rs", &body).is_some());
    assert!(check("src/routes/users.rs", &body).is_some());
    assert!(check("src/auth_middleware.rs", &body).is_some());
    assert!(check("src/api/v1/orders.rs", &body).is_some());
}

#[test]
fn should_not_block_single_async_fn_over_100_lines() {
    let mut body = "pub async fn request_context_middleware() {}\n".to_owned();
    body.push_str(&"fn helper() {}\n".repeat(110));
    assert!(check("src/middleware/request_context.rs", &body).is_none());
}

#[test]
fn should_not_block_handler_oversized_with_split_escape_hatch() {
    let mut body = "// split: two handlers intentionally colocated\n".to_owned();
    body.push_str(&"pub async fn foo() {}\npub async fn bar() {}\n".repeat(52));
    assert!(check("src/services/combined.rs", &body).is_none());
}

#[test]
fn should_not_block_handler_file_under_line_limit() {
    let body = "pub async fn foo() {}\npub async fn bar() {}\n".repeat(3);
    assert!(check("src/services/small.rs", &body).is_none());
}
