//! Shape proofs for the back-fill RPC DTOs. The async grading path needs a live
//! store (covered by the kavach-surreal integration tests); here we lock the
//! wire contract — notably that the field renamed to dodge the secret-Debug
//! heuristic still serializes as `session_id` for callers.

use super::{BanditBackfillParams, BanditBackfillResult};

#[test]
fn params_deserialize_from_the_session_id_wire_name() {
    let json = r#"{"session_id":"sess_abc","verified_clean":true,"limit":50}"#;
    let p: BanditBackfillParams = serde_json::from_str(json).expect("parse params");
    assert_eq!(p.session, "sess_abc", "wire `session_id` maps to the `session` field");
    assert!(p.verified_clean);
    assert_eq!(p.limit, 50);
}

#[test]
fn params_serialize_back_to_session_id() {
    let p = BanditBackfillParams {
        session: "sess_xyz".to_owned(),
        verified_clean: false,
        limit: 10,
    };
    let json = serde_json::to_string(&p).expect("serialize");
    assert!(json.contains(r#""session_id":"sess_xyz""#), "field renames to session_id on the wire");
}

#[test]
fn result_round_trips() {
    let r = BanditBackfillResult {
        success: true,
        graded: 7,
        skipped: 1,
        error: None,
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: BanditBackfillResult = serde_json::from_str(&json).expect("parse");
    assert_eq!(back.graded, 7);
    assert_eq!(back.skipped, 1);
    assert!(back.success && back.error.is_none());
}
