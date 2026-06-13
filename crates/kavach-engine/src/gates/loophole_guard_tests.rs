use super::check_loophole_interrogation;

#[test]
fn fires_on_done_claim_touching_risk_path() {
    let c = "Done — the lease acquire is now atomic and the claim is race-free.";
    let out = check_loophole_interrogation(c).expect("should fire");
    assert!(out.contains("[LOOPHOLE_CHECK]"));
    assert!(out.contains("concurrency"));
}

#[test]
fn silent_on_done_claim_without_risk_path() {
    // Completion language but a trivial, non-risk change -> no nag.
    let c = "Done — renamed the variable and updated the doc comment.";
    assert!(check_loophole_interrogation(c).is_none());
}

#[test]
fn silent_on_risk_path_without_done_claim() {
    // Touches auth but makes no completion claim -> not the trigger moment.
    let c = "Adding an auth check to the session token handler.";
    assert!(check_loophole_interrogation(c).is_none());
}

#[test]
fn silent_on_empty() {
    assert!(check_loophole_interrogation("").is_none());
}

#[test]
fn fires_on_payment_completion() {
    let c = "Fixed the balance transfer — transaction is committed atomically.";
    assert!(check_loophole_interrogation(c).is_some());
}

#[test]
fn stop_fires_when_risk_completion_lacks_answer() {
    use super::check_stop_interrogation;
    let msg = "Done — the lease claim is now atomic and race-free.";
    let out = check_stop_interrogation(msg).expect("should nudge at stop");
    assert!(out.contains("mistake ledger"));
}

#[test]
fn stop_silent_when_loopholes_already_considered() {
    use super::check_stop_interrogation;
    let msg = "Done — the lease claim is now atomic.\n\
               Loopholes considered: concurrency -> closed at acquire.rs:38; \
               failure -> TTL reclaim; replay -> N/A.";
    assert!(check_stop_interrogation(msg).is_none());
}

#[test]
fn stop_silent_on_trivial_turn() {
    use super::check_stop_interrogation;
    let msg = "Done — renamed a variable and fixed a typo.";
    assert!(check_stop_interrogation(msg).is_none());
}
