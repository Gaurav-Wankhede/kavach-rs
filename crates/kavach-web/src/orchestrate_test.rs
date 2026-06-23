//! TDD: OpenAI front-door handler core. Localhost-only (server binds
//! Ipv4Addr::LOCALHOST in lib.rs::serve) — the authz model is the loopback bind.
//! SOURCE: decision.fugu-u4-held-pending-authorization (authz = localhost-only).
use super::*;
use kavach_engine::ChatMessage;

#[test]
fn run_maps_request_through_pool_to_response() {
    // `true` echo backend stands in for a vendor: exit 0, empty stdout.
    let pool = test_pool();
    let req = ChatCompletionRequest {
        model: "kavach-fugu".into(),
        messages: vec![ChatMessage { role: "user".into(), content: "true".into() }],
    };
    let resp = run_orchestration(&pool, &req).expect("dispatch ok");
    assert_eq!(resp.object, "chat.completion");
    assert_eq!(resp.model, "kavach-fugu");
    assert_eq!(resp.choices[0].message.role, "assistant");
}

#[test]
fn failed_vendor_is_an_error_not_a_silent_ok() {
    let pool = fail_pool();
    let req = ChatCompletionRequest {
        model: "m".into(),
        messages: vec![ChatMessage { role: "user".into(), content: "x".into() }],
    };
    assert!(run_orchestration(&pool, &req).is_err());
}
