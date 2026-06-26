use crate::state::SessionState;

// ── User-focus override predicate (operator directive 2026-06-18) ───────────────
// `user_is_steering_this_turn()` is the stop-gate's authority check: a turn the
// user steered must NOT be hijacked onto a different kanban card.

#[test]
fn user_steering_true_when_directive_stamped_this_turn() {
    let mut s = SessionState::default();
    s.increment_turn(); // turn 1
    s.mark_user_directive(); // user spoke on turn 1
    assert!(
        s.user_is_steering_this_turn(),
        "user spoke THIS turn → steering"
    );
}

#[test]
fn user_steering_false_on_a_later_autonomous_turn() {
    let mut s = SessionState::default();
    s.increment_turn();
    s.mark_user_directive(); // user spoke on turn 1
    s.increment_turn(); // turn 2 — autonomous loop tick, user did NOT speak
    assert!(
        !s.user_is_steering_this_turn(),
        "a later turn with no fresh directive → autonomous, dispatch normally"
    );
}

#[test]
fn user_steering_false_at_session_start_no_directive() {
    let s = SessionState::default();
    assert!(
        !s.user_is_steering_this_turn(),
        "turn 0 / no directive → not steering"
    );
}

#[test]
fn mark_research_done() {
    let mut s = SessionState::default();
    assert!(!s.research_done);
    s.mark_research_done();
    assert!(s.research_done);
}

#[test]
fn mark_memory_queried() {
    let mut s = SessionState::default();
    assert!(!s.memory_queried);
    s.mark_memory_queried();
    assert!(s.memory_queried);
}

#[test]
fn increment_turn() {
    let mut s = SessionState::default();
    assert_eq!(s.turn_count, 0);
    s.increment_turn();
    s.increment_turn();
    assert_eq!(s.turn_count, 2);
}

#[test]
fn add_case_fact_caps_at_20() {
    let mut s = SessionState::default();
    for i in 0..25 {
        s.add_case_fact(&format!("fact-{i}"));
    }
    assert_eq!(s.case_facts.len(), 20);
}

#[test]
fn add_case_fact_sanitizes_newlines() {
    let mut s = SessionState::default();
    s.add_case_fact("line1\nline2\rline3");
    assert_eq!(s.case_facts[0], "line1 line2 line3");
}

#[test]
fn has_recent_failure_false_initially() {
    let s = SessionState::default();
    assert!(!s.has_recent_failure());
}

#[test]
fn increment_stop_reblock_is_bounded() {
    // The pending-work breaker must terminate: bounded at max + 1, never ∞,
    // so the stop gate reaches its forced-stop terminal instead of looping.
    let mut s = SessionState::default();
    for _ in 0..50 {
        s.increment_stop_reblock();
    }
    assert!(s.stop_reblock_count <= SessionState::max_stop_reblocks() + 1);
}

#[test]
fn clear_stop_reblock_resets_only_on_clean_stop() {
    let mut s = SessionState::default();
    s.increment_stop_reblock();
    s.increment_stop_reblock();
    s.clear_stop_reblock(); // genuine clean stop / forced terminal
    assert_eq!(s.stop_reblock_count, 0);
}

// --- rca.stop-breaker-no-progress-reset regression suite ---
// The bounded breaker is supposed to trip on LIVE-LOCK (no progress between
// stops) — NOT on a successful multi-wave plan. These tests prove the
// AWS/Fowler circuit-breaker success-reset edge is wired.

#[test]
fn breaker_resets_when_writes_advance_between_stops() {
    // Wave 1 finishes (writes happened), agent stops. Wave 2 finishes
    // (more writes), agent stops. The breaker must reset on each wave's
    // forward progress instead of climbing toward the cap.
    let mut s = SessionState::default();
    s.last_write_turn = 5; // wave 1 wrote code on turn 5
    s.increment_stop_reblock();
    // snapshot captures last_write_turn=5; count = 1 (no progress vs the
    // default snapshot of 0... wait — last_write_turn=5 > snapshot=0, so
    // progress IS detected → reset to 0). Verify:
    assert_eq!(
        s.stop_reblock_count, 0,
        "wave-1 writes (last_write_turn=5 > snapshot=0) are progress → reset"
    );
    assert_eq!(s.last_progress_snapshot_writes, 5, "snapshot updated");

    // Wave 2: agent edits more code, then stops again.
    s.last_write_turn = 9;
    s.increment_stop_reblock();
    assert_eq!(
        s.stop_reblock_count, 0,
        "wave-2 writes (9 > snapshot 5) are progress → reset stays at 0"
    );
    assert_eq!(s.last_progress_snapshot_writes, 9);
}

#[test]
fn breaker_trips_on_true_livelock_no_writes_between_stops() {
    // The live-lock guard: the agent is genuinely stuck — researching/
    // analyzing but writing nothing — the breaker MUST climb to the cap.
    let mut s = SessionState::default();
    s.last_write_turn = 0; // never wrote

    s.increment_stop_reblock();
    assert_eq!(s.stop_reblock_count, 1, "1st stalled stop → 1");

    s.increment_stop_reblock();
    assert_eq!(s.stop_reblock_count, 2, "2nd stalled stop → 2");

    s.increment_stop_reblock();
    assert_eq!(s.stop_reblock_count, 3, "3rd stalled stop → cap reached");

    // One past the cap is fine (saturating, terminal allowed).
    s.increment_stop_reblock();
    assert!(s.stop_reblock_count <= SessionState::max_stop_reblocks() + 1);
}

#[test]
fn breaker_reset_then_relapse_into_livelock_still_trips() {
    // Real-world sequence: wave 1 ships (reset), then agent gets stuck on
    // wave 2 (no writes) for 3 Stops in a row → breaker must trip on the
    // genuine stall, not be permanently disabled by the prior wave's reset.
    let mut s = SessionState::default();

    // Wave 1 ships.
    s.last_write_turn = 5;
    s.increment_stop_reblock(); // resets to 0, snapshot=5
    assert_eq!(s.stop_reblock_count, 0);

    // Wave 2: no writes for 3 consecutive Stops (live-lock).
    s.increment_stop_reblock();
    s.increment_stop_reblock();
    s.increment_stop_reblock();
    assert_eq!(
        s.stop_reblock_count, 3,
        "post-reset live-lock must still climb to cap"
    );
}

#[test]
fn has_progress_since_last_stop_strictly_compares() {
    let mut s = SessionState::default();
    assert!(
        !s.has_progress_since_last_stop(),
        "default state: no writes, no progress"
    );

    s.last_write_turn = 0;
    s.last_progress_snapshot_writes = 0;
    assert!(
        !s.has_progress_since_last_stop(),
        "equal values are NOT progress (strict >)"
    );

    s.last_write_turn = 1;
    assert!(
        s.has_progress_since_last_stop(),
        "advanced write turn IS progress"
    );
}

#[test]
fn db_card_close_counts_as_progress_and_resets_breaker() {
    // REGRESSION rca.stop-breaker-db-progress-blind: the loop died after 3
    // productive card-closes because closing a card is a DB write
    // (last_db_write_turn), not a file Write (last_write_turn), and the
    // progress check only watched file writes. A "close card -> dispatch
    // next -> stop" cycle therefore read as a no-progress spin.
    let mut s = SessionState::default();

    // No file writes this whole sequence — only DB card-closes.
    s.last_db_write_turn = 3; // closed a card on turn 3
    s.increment_stop_reblock();
    assert_eq!(
        s.stop_reblock_count, 0,
        "a DB card-close since last stop is progress → breaker resets, not increments"
    );

    // Next cycle: another card closed (turn advanced) → still progress.
    s.last_db_write_turn = 7;
    s.increment_stop_reblock();
    assert_eq!(
        s.stop_reblock_count, 0,
        "consecutive productive card-closes keep the breaker at 0"
    );

    // Live-lock safety preserved: a stop with NO new write of either kind
    // (file or DB) since the snapshot still counts down toward the cap.
    s.increment_stop_reblock();
    assert_eq!(
        s.stop_reblock_count, 1,
        "no progress (neither file nor DB advanced) → breaker increments"
    );
}

#[test]
fn has_progress_since_last_stop_counts_db_writes() {
    let mut s = SessionState::default();
    s.last_db_write_turn = 5;
    s.last_progress_snapshot_db_writes = 5;
    assert!(
        !s.has_progress_since_last_stop(),
        "equal DB-write values are NOT progress (strict >)"
    );
    s.last_db_write_turn = 6;
    assert!(
        s.has_progress_since_last_stop(),
        "advanced DB-write turn IS progress even with zero file writes"
    );
}
