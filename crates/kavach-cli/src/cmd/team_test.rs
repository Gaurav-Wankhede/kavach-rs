//! TDD: team claim-batch wires lease.acquire_set (card #1706 — batch-claim the
//! ready wavefront atomically). The pure param builder is the unit-tested core;
//! the RPC round-trip is covered by the daemon integration path.
use super::*;

#[test]
fn claim_params_carry_table_keys_and_session() {
    let keys = vec!["roadmap.unit.a".to_owned(), "roadmap.unit.b".to_owned()];
    let v = build_claim_params(&keys, "sess-123");
    assert_eq!(v["table"], "roadmap");
    assert_eq!(v["session_id"], "sess-123");
    assert_eq!(v["keys"][0], "roadmap.unit.a");
    assert_eq!(v["keys"][1], "roadmap.unit.b");
}

#[test]
fn claim_params_empty_batch_is_empty_keys_not_null() {
    let v = build_claim_params(&[], "s");
    assert!(v["keys"].as_array().expect("keys is array").is_empty());
}
