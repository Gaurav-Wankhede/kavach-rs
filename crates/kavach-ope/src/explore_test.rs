//! Proof suite for the epsilon-greedy core: the propensity arithmetic sums to
//! one, the greedy action dominates, the C2 P0-bar holds (an action absent from
//! the candidate set is never selected), degenerate inputs fall back to the
//! deterministic policy, and the dep-free RNG stays in range.

use super::{epsilon_greedy_select, next_draw};

/// A minimal advisory action set to exercise the generic selector + the C2 bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Allow,
    Ask,
    Block,
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn propensities_sum_to_one() {
    // |A| = 3, epsilon = 0.3 → greedy 0.8, each non-greedy 0.1. Drive each arm
    // with a draw that lands on it and confirm the three masses sum to 1.0.
    let set = [Act::Allow, Act::Ask, Act::Block];
    let (g_act, g_p) = epsilon_greedy_select(Act::Allow, &set, 0.3, 0.5); // exploit
    let (ask_act, ask_p) = epsilon_greedy_select(Act::Allow, &set, 0.3, 0.15); // idx 1
    let (blk_act, blk_p) = epsilon_greedy_select(Act::Allow, &set, 0.3, 0.25); // idx 2
    assert_eq!((g_act, ask_act, blk_act), (Act::Allow, Act::Ask, Act::Block));
    assert!(close(g_p, 0.8), "greedy propensity {g_p}");
    assert!(close(ask_p, 0.1) && close(blk_p, 0.1), "explore {ask_p} {blk_p}");
    assert!(close(g_p + ask_p + blk_p, 1.0), "sum {}", g_p + ask_p + blk_p);
}

#[test]
fn greedy_propensity_dominates_non_greedy() {
    let set = [Act::Allow, Act::Ask, Act::Block];
    let (_, greedy_p) = epsilon_greedy_select(Act::Allow, &set, 0.2, 0.9);
    // draw 0.1 with eps 0.2, |A|=3 maps to index 1 (Ask) — a genuine non-greedy.
    let (act, explore_p) = epsilon_greedy_select(Act::Allow, &set, 0.2, 0.1);
    assert_ne!(act, Act::Allow, "draw should explore off greedy");
    assert!(greedy_p > explore_p, "greedy {greedy_p} !> explore {explore_p}");
}

#[test]
fn single_candidate_is_deterministic() {
    // Nothing to explore → propensity 1.0 regardless of epsilon/draw.
    let (act, p) = epsilon_greedy_select(Act::Allow, &[Act::Allow], 0.9, 0.01);
    assert_eq!(act, Act::Allow);
    assert!(close(p, 1.0));
}

#[test]
fn epsilon_zero_is_exact_pre_p7_behavior() {
    let set = [Act::Allow, Act::Ask, Act::Block];
    let (act, p) = epsilon_greedy_select(Act::Allow, &set, 0.0, 0.0);
    assert_eq!(act, Act::Allow);
    assert!(close(p, 1.0));
}

#[test]
fn a_draw_below_epsilon_can_explore_off_greedy() {
    let set = [Act::Allow, Act::Ask, Act::Block];
    // draw 0.15 with eps 0.3 maps to index 1 (Ask) — a genuine non-greedy emit.
    let (act, p) = epsilon_greedy_select(Act::Allow, &set, 0.3, 0.15);
    assert_ne!(act, Act::Allow);
    assert!(close(p, 0.1));
}

#[test]
fn c2_never_selects_an_action_outside_the_candidate_set() {
    // Block is NOT a candidate → it must never be returned, at any draw. This is
    // the structural P0-bar: the selector cannot invent a forbidden action.
    let set = [Act::Allow, Act::Ask];
    for i in 0..=100u32 {
        let draw = f32::from(u16::try_from(i).unwrap()) / 100.0;
        let (act, _) = epsilon_greedy_select(Act::Allow, &set, 0.5, draw.min(0.999));
        assert_ne!(act, Act::Block, "synthesized a non-candidate at draw {draw}");
    }
}

#[test]
fn greedy_absent_from_candidates_falls_back_safe() {
    // Misuse guard: a greedy not in the set yields the deterministic fallback
    // rather than skewed propensities.
    let (act, p) = epsilon_greedy_select(Act::Block, &[Act::Allow, Act::Ask], 0.5, 0.1);
    assert_eq!(act, Act::Block);
    assert!(close(p, 1.0));
}

#[test]
fn next_draw_stays_in_unit_interval_and_advances() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15; // non-zero seed
    let first = state;
    for _ in 0..1000 {
        let d = next_draw(&mut state);
        assert!((0.0..1.0).contains(&d), "draw {d} out of [0,1)");
    }
    assert_ne!(state, first, "state must advance");
}
