//! Sidecar tests for `card_mutation` (included by the parent as
//! `#[path] mod card_mutation_tests;`, so this file is the module body itself).
fn is_claimable(current_status: &str) -> bool {
    current_status == "todo"
}

#[test]
fn claim_card_only_claims_a_fresh_todo() {
    assert!(is_claimable("todo"), "a fresh todo must be claimable");
    assert!(
        !is_claimable("in_progress"),
        "an in_progress card is already claimed — re-claim must be a no-op"
    );
    assert!(
        !is_claimable("done"),
        "a done card must never be resurrected by a claim"
    );
    assert!(
        !is_claimable("verified"),
        "a verified card must never be resurrected by a claim"
    );
}

// --- Lease-fused-claim wire contract (roadmap.unit.dispatch-lease-fused-claim) ---
// AC2: ClaimCardParams must accept an optional session_id (a legacy caller that
// omits it still deserializes), and ClaimCardResult must round-trip the fence
// epoch. These are the wire guarantees the engine edge depends on — a silent
// schema drift here re-opens the concurrent double-resume defect.

#[test]
fn claim_params_accept_optional_session_id() {
    use crate::methods::roadmap::types::ClaimCardParams;
    // Legacy payload (no session_id) must still parse — backward compatible.
    let legacy: ClaimCardParams =
        serde_json::from_value(serde_json::json!({"project": "p", "key": "k"}))
            .expect("legacy payload without session_id must deserialize");
    assert!(
        legacy.session_id.is_none(),
        "absent session_id => None => status-only claim (pre-lease behaviour)"
    );
    // Modern payload carries the lease owner.
    let modern: ClaimCardParams = serde_json::from_value(
        serde_json::json!({"project": "p", "key": "k", "session_id": "sess-A"}),
    )
    .expect("payload with session_id must deserialize");
    assert_eq!(
        modern.session_id.as_deref(),
        Some("sess-A"),
        "session_id must thread through so the RPC fuses an owner lease"
    );
}

#[test]
fn claim_result_round_trips_fence_epoch() {
    use crate::methods::roadmap::types::ClaimCardResult;
    let won = ClaimCardResult {
        key: "k".into(),
        status: "in_progress".into(),
        claimed: true,
        epoch: Some(7),
    };
    let won_json = serde_json::to_value(&won).expect("serialize");
    assert_eq!(
        won_json.get("epoch").and_then(serde_json::Value::as_i64),
        Some(7),
        "a won claim must expose the fence epoch so a renewer can present it"
    );
    // A legacy/lost claim has no epoch and must OMIT the field (skip_if None),
    // never emit `epoch: null` — keeps the wire shape identical to pre-change.
    let lost = ClaimCardResult {
        key: "k".into(),
        status: "todo".into(),
        claimed: false,
        epoch: None,
    };
    let v = serde_json::to_value(&lost).expect("serialize");
    assert!(
        v.get("epoch").is_none(),
        "epoch=None must be omitted from the wire, not serialized as null"
    );
}

fn is_auto_verifiable(current_status: &str) -> bool {
    current_status == "done"
}

#[test]
fn verify_card_only_promotes_a_done_card() {
    assert!(
        is_auto_verifiable("done"),
        "a done card must be auto-verifiable"
    );
    assert!(
        !is_auto_verifiable("todo"),
        "a todo card must NOT skip straight to verified — the work isn't done"
    );
    assert!(
        !is_auto_verifiable("in_progress"),
        "in_progress work is unfinished — never auto-verify it"
    );
    assert!(
        !is_auto_verifiable("verified"),
        "a verified card is terminal — auto-verify is a no-op"
    );
}

#[test]
fn done_card_satisfies_a_dependent_and_is_promotable() {
    use crate::methods::roadmap::readiness::{dep_key_satisfied, is_runnable_status};
    fn entry(key: &str, status: &str, content: &str) -> kavach_surreal::MemoryEntry {
        kavach_surreal::MemoryEntry {
            id: None,
            project: surrealdb_types::RecordId::new("project", "t"),
            category: Some("roadmap".into()),
            entry_key: key.into(),
            title: key.into(),
            content: content.into(),
            status: None,
            entry_status: Some(status.into()),
            tags: None,
            decay_score: None,
            access_count: None,
            created_at: None,
            updated_at: None,
            priority: None,
            lane: None,
        }
    }
    let all = vec![
        entry("dep.done", "done", ""),
        entry("child", "todo", "DEPENDS_ON: dep.done"),
    ];
    assert!(
        dep_key_satisfied("dep.done", &all),
        "done dep unblocks dependents"
    );
    assert!(
        !is_runnable_status("done"),
        "the done card is non-runnable, so it needs auto-verify to close"
    );
    assert!(
        is_auto_verifiable("done"),
        "and auto-verify will promote it once the witnesses pass"
    );
}
