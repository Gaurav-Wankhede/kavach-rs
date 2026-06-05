//! Tests for the bandit-log emit seam. Prove the serialized payload shape the
//! `db.bandit_row` RPC will persist, without standing up a daemon.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]
use super::*;

fn ctx() -> BanditContext {
    BanditContext::new("micro_file_guard", "Write", "rs", 2048, "low", 1)
}

#[test]
fn build_row_carries_action_and_pending_reward() {
    let row = build_row("sess_1", 42, ctx(), GateAction::Block, 1.0, None);
    assert_eq!(row.session_id, "sess_1");
    assert_eq!(row.timestamp_ms, 42);
    assert_eq!(row.action, GateAction::Block);
    assert!(row.awaits_reward(), "decision-time row has no reward yet");
}

#[test]
fn build_row_back_fills_reward_when_supplied() {
    let row = build_row(
        "s",
        0,
        ctx(),
        GateAction::Allow,
        1.0,
        Some(Reward::VerifiedClean),
    );
    assert!(!row.awaits_reward());
    assert_eq!(row.reward, Some(Reward::VerifiedClean));
}

#[test]
fn payload_is_the_wire_shape_the_rpc_stores() {
    // The RPC store persists exactly this string; a field drop here would lose
    // training signal silently, so assert the snake_case wire contract.
    let row = build_row("sess_wire", 7, ctx(), GateAction::Ask, 1.0, None);
    let json = payload_of(&row).expect("serialize");
    assert!(json.contains("\"action\":\"ask\""), "got: {json}");
    assert!(json.contains("\"reward\":null"), "got: {json}");
    assert!(
        json.contains("\"gate\":\"micro_file_guard\""),
        "got: {json}"
    );
    assert!(json.contains("\"session_id\":\"sess_wire\""), "got: {json}");
}

#[test]
fn emit_with_empty_session_is_a_silent_noop() {
    // No session id -> nothing to key the row to; must not panic or call the RPC.
    emit_decision("", ctx(), GateAction::Allow, 1.0, None);
}
