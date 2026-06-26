//! Proof suite for the P7 live-exploration wiring. The pure core (the propensity
//! arithmetic, the C2 candidate-set bar) is proven in `kavach_ope::explore`; here
//! we prove the WIRING: the disarmed default is the exact pre-P7 deterministic
//! emit, the seed is never the xorshift fixed point and varies per session, and
//! the truthy parser only arms on an explicit truthy.
//!
//! Env-reading paths are process-global; nextest's per-test process isolation
//! (the workspace test contract) means this process never has `KAVACH_RL_EXPLORE`
//! set, so `explore_action` exercises the disarmed branch deterministically
//! without us mutating env (`set_var` is `unsafe` in edition 2024 — forbidden here).

use super::{explore_action, fnv1a, held_out_roll, is_truthy, seed};
use kavach_patterns::bandit_log::GateAction;

#[test]
fn disarmed_default_is_deterministic_greedy_one() {
    // KAVACH_RL_EXPLORE is unset in this isolated test process ⇒ the exact pre-P7
    // behavior: the greedy action with propensity 1.0, regardless of session/clock.
    let (action, p) = explore_action(GateAction::Allow, "sess_x", 1_234_567);
    assert_eq!(
        action,
        GateAction::Allow,
        "disarmed must not explore off greedy"
    );
    assert!(
        (p - 1.0).abs() < f32::EPSILON,
        "disarmed propensity must be 1.0, got {p}"
    );
}

#[test]
fn disarmed_preserves_a_block_greedy_unchanged() {
    // A hard `Block` greedy (absent from the advisory set) must round-trip as
    // (Block, 1.0) — the C2 bar plus the disarmed fallback both agree.
    let (action, p) = explore_action(GateAction::Block, "sess_y", 42);
    assert_eq!(action, GateAction::Block);
    assert!((p - 1.0).abs() < f32::EPSILON);
}

#[test]
fn seed_is_never_zero_the_xorshift_fixed_point() {
    // The xorshift fixed point is 0; the low-bit force-set guarantees a live seed
    // even when timestamp and hash cancel. Probe the adversarial all-zero inputs.
    assert_ne!(
        seed("", 0),
        0,
        "empty session + epoch-zero must still seed non-zero"
    );
    // Construct a ts that XORs the hash to zero, proving the | 1 is load-bearing.
    let h = fnv1a("collide");
    let ts_that_cancels = h; // ts ^ h == 0 before the | 1
    let s = seed("collide", ts_that_cancels.cast_signed());
    assert_eq!(
        s, 1,
        "a fully-cancelling seed must be rescued to 1 by the low-bit set"
    );
    assert_ne!(s, 0);
}

#[test]
fn seed_varies_by_session_at_the_same_instant() {
    // Two sessions deciding in the same millisecond must explore independently —
    // the session-id hash is what decorrelates their draws.
    let ts = 9_999_999_i64;
    assert_ne!(
        seed("session-a", ts),
        seed("session-b", ts),
        "distinct sessions must produce distinct seeds at the same timestamp"
    );
}

#[test]
fn fnv1a_is_deterministic_and_distinguishes() {
    assert_eq!(fnv1a("abc"), fnv1a("abc"), "hash must be a pure function");
    assert_ne!(
        fnv1a("abc"),
        fnv1a("abd"),
        "a one-byte change must change the hash"
    );
    // The FNV-1a offset basis is the empty-string hash — a known fixed constant.
    assert_eq!(
        fnv1a(""),
        0xcbf2_9ce4_8422_2325,
        "empty string is the offset basis"
    );
}

#[test]
fn held_out_roll_is_in_range_and_decorrelated_from_exploration() {
    // P8: the held-out sampler draw must be a valid probability and must NOT equal
    // the exploration draw seeded from the same (session, ts) — else a turn that
    // explores would deterministically also be (or never be) held out, coupling
    // two decisions the design requires to be independent.
    let ts = 5_000_i64;
    let roll = held_out_roll("sess_h", ts);
    assert!(
        (0.0..1.0).contains(&roll),
        "held-out roll {roll} out of [0,1)"
    );
    // The exploration draw uses seed(); the held-out draw salts it. Same inputs,
    // different draws.
    let mut explore_state = seed("sess_h", ts);
    let explore_draw = kavach_ope::explore::next_draw(&mut explore_state);
    assert!(
        (roll - explore_draw).abs() > f32::EPSILON,
        "held-out draw {roll} must differ from exploration draw {explore_draw} (decorrelated salt)"
    );
}

#[test]
fn held_out_roll_varies_by_session() {
    let ts = 7_777_i64;
    assert!(
        (held_out_roll("a", ts) - held_out_roll("b", ts)).abs() > f32::EPSILON,
        "distinct sessions must roll independently at the same instant"
    );
}

#[test]
fn is_truthy_only_on_explicit_truthy() {
    for t in ["1", "true", "TRUE", "Yes", " on "] {
        assert!(is_truthy(t), "{t:?} should arm");
    }
    for f in ["", "0", "false", "no", "off", "maybe", "2"] {
        assert!(
            !is_truthy(f),
            "{f:?} must NOT arm (disarmed is the safe default)"
        );
    }
}
