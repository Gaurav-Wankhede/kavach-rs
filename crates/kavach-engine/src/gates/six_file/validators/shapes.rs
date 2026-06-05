//! Compound-predicate shape validators (multi-clause checks that don't fit the
//! `has_any` / `has_min_signals` helpers).

/// User story: needs an actor, a desire, and a context clause.
pub(super) fn user_story(lower: &str) -> Result<(), String> {
    let as_a = lower.contains("as a") || lower.contains("as an");
    let want = lower.contains("i want") || lower.contains("i can");
    let then = lower.contains("so that") || lower.contains("given") || lower.contains("when");
    (as_a && want && then)
        .then_some(())
        .ok_or_else(|| "story needs 'as a/an', 'i want/can', 'so that/given/when'".into())
}

/// API contract: needs an HTTP method and a status/code/response signal.
pub(super) fn api_contract(lower: &str) -> Result<(), String> {
    let method = ["get", "post", "put", "patch", "delete"]
        .iter()
        .any(|m| lower.contains(m));
    let status = lower.contains("status") || lower.contains("code") || lower.contains("response");
    (method && status)
        .then_some(())
        .ok_or_else(|| "API needs method + status/code".into())
}

/// State flow: needs the word "state" plus a transition signal.
pub(super) fn state_flow(lower: &str) -> Result<(), String> {
    (lower.contains("state")
        && ["transition", "flow", "next", "status"]
            .iter()
            .any(|s| lower.contains(s)))
    .then_some(())
    .ok_or_else(|| "state flow needs state + transition/flow/next/status".into())
}

/// Roadmap: needs both GOAL and VERIFY.
pub(super) fn roadmap(lower: &str) -> Result<(), String> {
    (lower.contains("goal") && lower.contains("verify"))
        .then_some(())
        .ok_or_else(|| "roadmap needs GOAL and VERIFY".into())
}
