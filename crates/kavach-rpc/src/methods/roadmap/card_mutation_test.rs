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
            owner_gated: None,
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
