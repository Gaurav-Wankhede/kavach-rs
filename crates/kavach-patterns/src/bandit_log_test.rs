//! Tests for the Layer-A bandit-log tuple (harness-rl Wave P2).
//!
//! Proves: the RLVR tuple round-trips through serde (the RPC store persists it),
//! the fail-closed reward scalars, propensity clamping, and the reward-pending flag.
use super::*;

fn ctx() -> BanditContext {
    BanditContext {
        gate: "nano_file_guard".into(),
        tool: "Write".into(),
        file_ext: "rs".into(),
        diff_bytes: 1280,
        intent_risk: "low".into(),
        prior_fire_count: 2,
    }
}

#[test]
fn reward_scalars_encode_fail_closed_bias() {
    // A false decision is the costly error: it scores below a needed ask, which
    // scores below a clean verify. This ordering is what the OPE estimators rank on.
    assert_eq!(Reward::VerifiedClean.value(), 1);
    assert_eq!(Reward::NeededAsk.value(), 0);
    assert_eq!(Reward::FalseDecision.value(), -1);
    assert!(Reward::FalseDecision.value() < Reward::NeededAsk.value());
    assert!(Reward::NeededAsk.value() < Reward::VerifiedClean.value());
}

#[test]
fn new_row_awaits_reward() {
    let row = BanditRow::new("sess_x", 100, ctx(), GateAction::Block, 1.0);
    assert!(
        row.awaits_reward(),
        "a freshly logged row has no reward yet"
    );
    assert_eq!(row.reward, None);
}

#[test]
fn propensity_is_clamped_to_unit_interval() {
    let hi = BanditRow::new("s", 0, ctx(), GateAction::Allow, 4.2);
    let lo = BanditRow::new("s", 0, ctx(), GateAction::Allow, -3.0);
    assert!((hi.propensity - 1.0).abs() < f32::EPSILON);
    assert!(lo.propensity.abs() < f32::EPSILON);
}

#[test]
fn row_round_trips_through_serde() {
    // The RPC bandit_log store serializes these to persist them — a field drop
    // here would silently lose training signal, so prove the full round-trip.
    let mut row = BanditRow::new("sess_round", 1717, ctx(), GateAction::Ask, 1.0);
    row.reward = Some(Reward::NeededAsk);

    let json = serde_json::to_string(&row).unwrap();
    let back: BanditRow = serde_json::from_str(&json).unwrap();
    assert_eq!(back, row);
    assert!(!back.awaits_reward());
}

#[test]
fn action_and_reward_use_snake_case_on_the_wire() {
    // Stable wire format the offline OPE crate (Layer B) parses.
    let row = BanditRow::new("s", 0, ctx(), GateAction::Block, 1.0);
    let json = serde_json::to_string(&row).unwrap();
    assert!(json.contains("\"action\":\"block\""), "got: {json}");
    assert!(json.contains("\"reward\":null"));
}

#[test]
fn back_filled_reward_serializes_as_snake_case() {
    let mut row = BanditRow::new("s", 0, ctx(), GateAction::Allow, 1.0);
    row.reward = Some(Reward::VerifiedClean);
    let json = serde_json::to_string(&row).unwrap();
    assert!(
        json.contains("\"reward\":\"verified_clean\""),
        "got: {json}"
    );
}

#[test]
fn new_row_defaults_to_the_hard_channel() {
    // P8: every on-policy / pre-P8 row is the HARD (witness) channel. The audit's
    // soft split (`channel_reward(.., true)`) must NOT pick these up.
    let row = BanditRow::new("s", 0, ctx(), GateAction::Allow, 1.0);
    assert!(!row.held_out, "a fresh row belongs to the hard channel by default");
}

#[test]
fn into_held_out_tags_the_soft_channel_and_serializes() {
    // The held-out flag is what `db.ope_audit` splits on; if it failed to serialize
    // the soft channel would always read empty ⇒ the audit would never leave
    // Inconclusive. Prove the flag round-trips on the wire.
    let row = BanditRow::new("s", 1, ctx(), GateAction::Allow, 0.9).into_held_out();
    assert!(row.held_out);
    let json = serde_json::to_string(&row).unwrap();
    assert!(json.contains("\"held_out\":true"), "got: {json}");
    let back: BanditRow = serde_json::from_str(&json).unwrap();
    assert_eq!(back, row);
    assert!(back.held_out);
}

#[test]
fn legacy_row_without_held_out_field_deserializes_as_hard_channel() {
    // A row persisted before P8 has no `held_out` key. `#[serde(default)]` must
    // read it as false (hard channel) — never panic, never mis-route to soft.
    let legacy = r#"{"session_id":"s","timestamp_ms":0,"context":{"gate":"g","tool":"Write","file_ext":"rs","diff_bytes":0,"intent_risk":"low","prior_fire_count":0},"action":"allow","propensity":1.0,"reward":null}"#;
    let back: BanditRow = serde_json::from_str(legacy).unwrap();
    assert!(!back.held_out, "a pre-P8 row must default to the hard channel");
}
