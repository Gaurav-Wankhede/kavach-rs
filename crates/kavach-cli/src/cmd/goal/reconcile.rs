// `kavach goal reconcile` — connect an oracle receipt to the stop-gate flag.
//
// Phase 4 of roadmap.unit.goal-oracle-workflow. The Workflow loop runs the
// oracle each attempt and records a receipt (`kavach db event --type
// goal_loop_attempt`). This verb takes that receipt's verdict and flips the
// session's proof-gated `goal_receipt_pass` flag — the ONLY way that flag
// becomes true. The stop gate (enforcement.rs `loop_target_reached`) then lets
// the goal close. A `fail` verdict explicitly clears the flag so a stale pass
// can never linger.
//
// The oracle for THIS phase: reconcile("pass") -> flag set -> goal reached;
// reconcile("fail") -> flag clear -> goal blocked (see tests).
//
// SOURCE: decision.goal-oracle-workflow.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};

/// Apply an oracle verdict to a session's proof-gated completion flag.
/// Returns the new value of `goal_receipt_pass`. Pure — no I/O — so the state
/// transition is unit-testable in isolation.
fn apply_verdict(state: &mut kavach_session::SessionState, oracle_result: &str) -> bool {
    let pass = oracle_result == "pass";
    state.goal_receipt_pass = pass;
    pass
}

/// `kavach goal reconcile --goal-id <id> --oracle-result <pass|fail>`.
pub(crate) fn run(goal_id: &str, oracle_result: &str) -> i32 {
    let mut session = kavach_session::get_or_create_session();
    // Locked read-modify-write — bare load+save() is a racy RMW (lost update).
    // SOURCE: decision.goal-reconcile-lost-update-fix.
    let mut pass = false;
    if let Err(e) = session.atomic_update(|s| pass = apply_verdict(s, oracle_result)) {
        eprintln!("kavach goal reconcile: persist session: {e}");
        return 1;
    }
    let verdict = if pass {
        "PASS — goal may close"
    } else {
        "FAIL — goal stays blocked"
    };
    let banner = format!("[ORACLE_RECONCILED] goal={goal_id} result={oracle_result} -> {verdict}");
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    i32::from(!pass) // 0 when the oracle passed, 1 when it did not
}

#[cfg(test)]
mod tests {
    use super::apply_verdict;
    use kavach_session::SessionState;

    #[test]
    fn pass_sets_the_proof_flag() {
        // THE PHASE-4 ORACLE: a 'pass' verdict flips goal_receipt_pass true.
        let mut s = SessionState::default();
        assert!(apply_verdict(&mut s, "pass"));
        assert!(s.goal_receipt_pass);
    }

    #[test]
    fn fail_clears_the_proof_flag() {
        let mut s = SessionState::default();
        s.goal_receipt_pass = true; // a prior pass
        assert!(!apply_verdict(&mut s, "fail"));
        assert!(!s.goal_receipt_pass, "a fail must clear a stale pass");
    }

    #[test]
    fn unknown_verdict_is_treated_as_not_passing() {
        let mut s = SessionState::default();
        assert!(!apply_verdict(&mut s, "timeout"));
        assert!(!s.goal_receipt_pass);
    }

    #[test]
    fn reconcile_then_gate_reaches_only_on_pass() {
        // End-to-end of the invariant: reconcile drives loop_target_reached.
        let mut s = SessionState::default();
        s.start_loop("goal");
        apply_verdict(&mut s, "fail");
        assert!(!s.loop_target_reached(), "fail must keep the gate closed");
        apply_verdict(&mut s, "pass");
        assert!(s.loop_target_reached(), "pass must let the gate open");
    }
}
