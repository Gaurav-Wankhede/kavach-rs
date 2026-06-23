//! OpenAI-compatible front door — `POST /v1/chat/completions`.
//!
//! Authz model = the server's loopback bind (`Ipv4Addr::LOCALHOST` in
//! [`crate::serve`]); no token. Maps an OpenAI chat request through the kavach
//! vendor pool ([`request_to_vendor`] → role backend dispatch →
//! [`vendor_to_response`]) so any local OpenAI client drives the orchestrator.
//!
//! SOURCE: decision.fugu-u4-held-pending-authorization (localhost-only), wired
//! per decision.wired-not-just-defined-is-done.
use axum::Json;
use axum::http::StatusCode;

use kavach_engine::{
    request_to_vendor, vendor_to_response, ChatCompletionRequest, ChatCompletionResponse, RolePool,
};

/// Pure core: map a chat request through `pool` to a chat response.
///
/// # Errors
/// Returns the vendor's error string when dispatch fails (fail-closed — a failed
/// vendor is never a silent success).
pub fn run_orchestration(
    pool: &RolePool,
    req: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, String> {
    let vreq = request_to_vendor(req);
    let out = pool
        .backend_for(vreq.role)
        .dispatch(&vreq)
        .map_err(|e| e.to_string())?;
    Ok(vendor_to_response(&out, &req.model))
}

/// `POST /v1/chat/completions` handler. Localhost-only via the server bind.
pub async fn chat_completions(
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, String)> {
    let pool = RolePool::default();
    run_orchestration(&pool, &req)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_GATEWAY, e))
}

#[cfg(test)]
#[path = "orchestrate_test.rs"]
mod orchestrate_test;

#[cfg(test)]
fn test_pool() -> RolePool {
    RolePool::with_argv("true-vendor", |_| vec!["true".into()])
}

#[cfg(test)]
fn fail_pool() -> RolePool {
    RolePool::with_argv("false-vendor", |_| vec!["false".into()])
}
