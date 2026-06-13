//! Extract-on-verify: fire-and-forget pattern-extractor dispatch when the oracle passes.
use kavach_session::SessionState;

/// Request procedural-memory extraction after a proof-gated verify (`goal_receipt_pass`).
///
/// Fire-and-forget: appends an `event` row the pattern-extractor agent (or a
/// downstream worker) can consume. Never blocks the Stop hook.
pub(super) fn trigger_on_verify(session: &SessionState) {
    if !session.goal_receipt_pass || session.project.is_empty() {
        return;
    }
    let payload = serde_json::json!({
        "session_id": session.session_id,
        "project": session.project,
        "card": session.current_kanban_card,
        "goal": session.goal_state,
        "agent": "pattern-extractor",
    })
    .to_string();
    let params = serde_json::json!({
        "event_type": "pattern_extract_requested",
        "source": "stop_verify",
        "project": session.project,
        "payload": payload,
    });
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> =
        kavach_rpc::client::call("event.append", Some(params));
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_session::SessionState;

    #[test]
    fn no_op_without_receipt_pass() {
        let session = SessionState::default();
        trigger_on_verify(&session);
    }
}
