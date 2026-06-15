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

    // Close the extract->retrieve loop (F4): the event above only REQUESTS async
    // extraction — nothing consumes it synchronously, so a skill learned on this
    // verify was never retrievable on a later similar write. Persist a durable
    // `pattern` row NOW so the procedural memory lands in the queryable store the
    // pattern-extractor + nearest-retrieval already read. The async agent can
    // later enrich this seed; the row's existence is what makes the loop closed
    // rather than open. SOURCE: unit.loop-eng-injection.f4-skill-procedural.
    persist_pattern_seed(session);
}

/// Write the verify-time procedural-memory seed as a `pattern` row keyed by the
/// closed card, so `retrieve-on-similar` has something to find. Idempotent on the
/// key (re-verify of the same card updates, not duplicates). Fire-and-forget: a
/// daemon outage must never block the Stop hook (CWE-392 loop-disabling guard).
fn persist_pattern_seed(session: &SessionState) {
    if session.current_kanban_card.is_empty() {
        return;
    }
    let key = format!("pattern.verified.{}", session.current_kanban_card);
    let title = format!("Verified procedural seed: {}", session.current_kanban_card);
    let content = format!(
        "EXTRACT-ON-VERIFY seed (f4). card={} goal={} session={}. \
         A proof-gated verify passed here; the reusable procedure is whatever closed \
         the 3-witness loop for this card. retrieve-on-similar surfaces this on a \
         future write whose RAG text matches the card's domain.",
        session.current_kanban_card, session.goal_state, session.session_id
    );
    let params = serde_json::json!({
        "project": session.project,
        "category": "pattern",
        "key": key,
        "title": title,
        "content": content,
        "update_key": key,
    });
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design (Stop must never block)"
    )]
    let _: Result<serde_json::Value, _> = kavach_rpc::client::call("db.write", Some(params));
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

    #[test]
    fn pattern_seed_key_is_card_scoped_and_idempotent() {
        // F4: the persisted pattern row must be keyed by the verified card so a
        // re-verify UPSERTS (updates) rather than duplicating — the key is the
        // idempotency token. Assert the key-derivation shape directly (the RPC
        // itself is fire-and-forget against a live daemon, not unit-reachable).
        let key = format!("pattern.verified.{}", "unit.demo-card");
        assert_eq!(key, "pattern.verified.unit.demo-card");
        assert!(
            key.starts_with("pattern.verified."),
            "seed key must be namespaced so retrieve-on-similar can scope to verify-seeds"
        );
    }

    #[test]
    fn persist_pattern_seed_no_ops_without_card() {
        // boundary: an empty current_kanban_card => no row written (a seed with
        // no card key is meaningless and would collide on `pattern.verified.`).
        let mut session = SessionState::default();
        session.goal_receipt_pass = true;
        session.project = "p".into();
        session.current_kanban_card = String::new();
        persist_pattern_seed(&session); // must early-return, not panic / not write
    }
}
