use crate::state::SessionState;

#[test]
fn gate_block_trips_at_threshold_on_successful_persist() {
    let mut s = SessionState::default();
    s.gate_circuit_breaker_threshold = 3;
    assert!(!s.record_gate_block("deferral"), "block 1 must not trip");
    assert!(!s.record_gate_block("deferral"), "block 2 must not trip");
    assert!(s.record_gate_block("deferral"), "block 3 must trip");
    assert!(s.is_gate_tripped("deferral"));
    assert!(
        !s.record_gate_block("permission"),
        "other category independent"
    );
}

#[cfg(unix)]
#[test]
fn record_gate_block_fails_closed_when_state_dir_unwritable() {
    use std::os::unix::fs::PermissionsExt;

    struct Restore {
        tmp: std::path::PathBuf,
    }
    impl Drop for Restore {
        #[expect(
            clippy::print_stderr,
            reason = "test-only cleanup guard: stderr is the only diagnostic channel in a Drop \
                      with no tracing subscriber; a leaked perms/dir failure must stay visible"
        )]
        fn drop(&mut self) {
            if let Ok(m) = std::fs::metadata(&self.tmp) {
                let mut p = m.permissions();
                p.set_mode(0o700);
                if let Err(e) = std::fs::set_permissions(&self.tmp, p) {
                    eprintln!("test cleanup: restore perms failed: {e}");
                }
            }
            if let Err(e) = std::fs::remove_dir_all(&self.tmp) {
                eprintln!("test cleanup: remove tmp dir failed: {e}");
            }
            crate::paths::set_test_state_dir(None);
        }
    }

    let unique = format!(
        "kavach-faultinj-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    );
    let tmp_dir = std::env::temp_dir().join(unique);

    let _restore = Restore {
        tmp: tmp_dir.clone(),
    };

    std::fs::create_dir_all(&tmp_dir).expect("create tmp state dir");
    crate::paths::set_test_state_dir(Some(tmp_dir.clone()));
    let mut ro = std::fs::metadata(&tmp_dir)
        .expect("stat state dir")
        .permissions();
    ro.set_mode(0o500);
    std::fs::set_permissions(&tmp_dir, ro).expect("chmod state dir ro");

    let mut s = SessionState::default();
    s.gate_circuit_breaker_threshold = 1;

    let tripped = s.record_gate_block("deferral");
    assert!(
        !tripped,
        "record_gate_block must fail closed (return false) when persist fails"
    );
    assert!(
        !s.is_gate_tripped("deferral"),
        "no tripped category may be reported when the trip could not be persisted"
    );
}

#[test]
fn test_record_skill_no_duplicate() {
    let mut s = SessionState::default();
    s.invoked_skills.push("rust".into());
    s.record_skill_invoked("rust");
    assert_eq!(s.invoked_skills.len(), 1);
}

#[test]
fn evidence_window_non_implement_always_passes() {
    let mut s = SessionState::default();
    s.intent_type = "general".into();
    s.websearch_count_since_intent = 0;
    assert!(s.evidence_window_satisfied());

    s.intent_type = "explain".into();
    assert!(s.evidence_window_satisfied());
}

#[test]
fn evidence_window_implement_blocked_without_websearch() {
    let mut s = SessionState::default();
    s.intent_type = "implement".into();
    s.websearch_count_since_intent = 0;
    assert!(!s.evidence_window_satisfied());
}

#[test]
fn evidence_window_implement_passes_after_websearch() {
    let mut s = SessionState::default();
    s.intent_type = "implement".into();
    s.websearch_count_since_intent = 0;
    s.record_websearch();
    assert!(s.evidence_window_satisfied());
}

#[test]
fn record_websearch_increments_counter() {
    let mut s = SessionState::default();
    assert_eq!(s.websearch_count_since_intent, 0);
    s.record_websearch();
    assert_eq!(s.websearch_count_since_intent, 1);
    s.record_websearch();
    assert_eq!(s.websearch_count_since_intent, 2);
}

#[test]
fn record_websearch_saturates_at_i32_max() {
    let mut s = SessionState::default();
    s.websearch_count_since_intent = i32::MAX;
    s.record_websearch();
    assert_eq!(s.websearch_count_since_intent, i32::MAX);
}

#[test]
fn reset_evidence_window_clears_counter_and_anchors_turn() {
    let mut s = SessionState::default();
    s.intent_type = "implement".into();
    s.websearch_count_since_intent = 3;
    s.turn_count = 5;
    s.reset_evidence_window();
    assert_eq!(s.websearch_count_since_intent, 0);
    assert_eq!(s.intent_set_turn, 5);
    assert!(!s.evidence_window_satisfied());
}

#[test]
fn reset_then_research_satisfies_window() {
    let mut s = SessionState::default();
    s.intent_type = "implement".into();
    s.turn_count = 7;
    s.reset_evidence_window();
    assert!(!s.evidence_window_satisfied());
    s.record_websearch();
    assert!(s.evidence_window_satisfied());
}

#[test]
fn start_loop_sets_active_and_target() {
    let mut s = SessionState::default();
    s.turn_count = 10;
    s.start_loop("kanban:empty");
    assert!(s.loop_active);
    assert_eq!(s.loop_target, "kanban:empty");
    assert_eq!(s.loop_iteration, 0);
    assert_eq!(s.loop_start_turn, 10);
}

#[test]
fn increment_loop_increments_iteration() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    s.increment_loop();
    s.increment_loop();
    assert_eq!(s.loop_iteration, 2);
}

#[test]
fn increment_loop_saturates_at_max() {
    let mut s = SessionState::default();
    s.loop_iteration = i32::MAX;
    s.increment_loop();
    assert_eq!(s.loop_iteration, i32::MAX);
}

#[test]
fn loop_exceeded_max_when_iteration_reaches_limit() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    s.loop_max_iterations = 3;
    s.loop_iteration = 3;
    assert!(s.loop_exceeded_max());
}

#[test]
fn loop_exceeded_max_false_when_inactive() {
    let mut s = SessionState::default();
    s.loop_active = false;
    s.loop_iteration = 100;
    s.loop_max_iterations = 50;
    assert!(!s.loop_exceeded_max());
}

#[test]
fn loop_target_reached_kanban_empty() {
    let mut s = SessionState::default();
    s.start_loop("kanban:empty");
    s.loop_kanban_runnable = Some(0);
    assert!(s.loop_target_reached());
}

#[test]
fn loop_target_reached_kanban_not_empty() {
    let mut s = SessionState::default();
    s.start_loop("kanban:empty");
    s.loop_kanban_runnable = Some(6);
    assert!(!s.loop_target_reached());
}

#[test]
fn loop_target_kanban_uncensused_fails_closed() {
    let mut s = SessionState::default();
    s.start_loop("kanban:empty");
    assert!(!s.loop_target_reached());
}

#[test]
fn loop_target_reached_phase_match() {
    let mut s = SessionState::default();
    s.start_loop("phase:TEST");
    s.current_phase = "TEST".into();
    assert!(s.loop_target_reached());
}

#[test]
fn loop_target_reached_phase_mismatch() {
    let mut s = SessionState::default();
    s.start_loop("phase:TEST");
    s.current_phase = "IMPLEMENT".into();
    assert!(!s.loop_target_reached());
}

#[test]
fn loop_target_reached_goal() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    s.goal_achieved = true;
    assert!(s.loop_target_reached());
}

#[test]
fn loop_target_reached_goal_via_verified_receipt() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    s.goal_receipt_pass = true;
    assert!(s.loop_target_reached());
}

#[test]
fn loop_target_goal_blocks_without_proof() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    assert!(!s.goal_receipt_pass);
    assert!(!s.goal_achieved);
    assert!(!s.loop_target_reached());
}

#[test]
fn goal_receipt_pass_round_trips_through_serde() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    s.goal_receipt_pass = true;
    let ini = s.to_ini_full();
    let parsed = crate::load::parse_ini_str(&ini);
    assert!(
        parsed.goal_receipt_pass,
        "receipt flag must survive persistence"
    );
}

#[test]
fn loop_target_reached_when_inactive() {
    let s = SessionState::default();
    assert!(s.loop_target_reached());
}

#[test]
fn loop_target_reached_unknown_target_returns_false() {
    let mut s = SessionState::default();
    s.start_loop("unknown:target");
    assert!(!s.loop_target_reached());
}

#[test]
fn stop_loop_deactivates() {
    let mut s = SessionState::default();
    s.start_loop("goal");
    s.stop_loop();
    assert!(!s.loop_active);
}

#[test]
fn budget_not_exceeded_when_total_unset() {
    let mut s = SessionState::default();
    s.token_budget_total = 0;
    s.token_budget_used = 999_999;
    assert!(!s.budget_exceeded(), "zero total = unbounded, never trips");
}

#[test]
fn budget_not_exceeded_below_total() {
    let mut s = SessionState::default();
    s.token_budget_total = 1000;
    s.token_budget_used = 999;
    assert!(!s.budget_exceeded());
}

#[test]
fn budget_exceeded_at_and_above_total() {
    let mut s = SessionState::default();
    s.token_budget_total = 1000;
    s.token_budget_used = 1000;
    assert!(s.budget_exceeded(), "spend == total trips (>=)");
    s.token_budget_used = 1500;
    assert!(s.budget_exceeded());
}

#[test]
fn record_token_spend_accumulates_and_clamps_negative() {
    let mut s = SessionState::default();
    s.record_token_spend(100);
    s.record_token_spend(50);
    assert_eq!(s.token_budget_used, 150);
    s.record_token_spend(-999);
    assert_eq!(s.token_budget_used, 150);
}

#[test]
fn record_token_spend_saturates_at_i32_max() {
    let mut s = SessionState::default();
    s.token_budget_used = i32::MAX;
    s.record_token_spend(1000);
    assert_eq!(s.token_budget_used, i32::MAX, "no wrap re-opening budget");
}
