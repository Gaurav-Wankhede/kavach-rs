use super::check_loophole_interrogation;

#[test]
fn fires_on_done_claim_touching_risk_path() {
    let c = "Done — the lease acquire is now atomic and the claim is race-free.";
    let out = check_loophole_interrogation(c).expect("should fire");
    assert!(out.contains("[LOOPHOLE_CHECK]"));
    assert!(out.contains("concurrency"));
    // Imperative, fix-first language — NOT a passive "consider" prompt.
    assert!(out.contains("FIX THIS TURN"), "commands a same-turn fix: {out}");
    assert!(out.contains("Loopholes closed:"), "marker is the action verb: {out}");
    assert!(
        out.contains("do NOT narrate") && out.contains("do NOT defer"),
        "forbids the summary/deferral path: {out}"
    );
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
    // wrote_this_turn = true: a real risk-bearing write happened this turn.
    let out = check_stop_interrogation(msg, true).expect("should nudge at stop");
    assert!(out.contains("mistake ledger"));
    // Imperative: command the fix, do not just record-and-move-on.
    assert!(out.contains("Do NOT stop"), "refuses the stop, drives the fix: {out}");
    assert!(out.contains("fix it now"), "fix-first language: {out}");
}

#[test]
fn stop_silent_on_read_only_turn_even_with_risk_prose() {
    use super::check_stop_interrogation;
    // The false-positive fix: a read-only Q&A turn whose PROSE describes past
    // risk fixes (lease/atomic/done) must NOT refuse the stop — nothing was
    // written, so no loophole can be live. wrote_this_turn = false.
    let msg = "Done — explained the lease claim is now atomic and race-free.";
    assert!(
        check_stop_interrogation(msg, false).is_none(),
        "a turn that wrote no file cannot have a live loophole; risk WORDS != risk WRITE"
    );
}

#[test]
fn stop_silent_when_loopholes_already_closed() {
    use super::check_stop_interrogation;
    // The action marker `Loopholes closed:` satisfies the gate; a passive
    // `considered:` no longer does.
    let msg = "Done — the lease claim is now atomic.\n\
               Loopholes closed: concurrency -> fixed at acquire.rs:38; \
               failure -> TTL reclaim at lease.rs:71; replay -> N/A at claim.rs:12.";
    assert!(check_stop_interrogation(msg, true).is_none());
}

#[test]
fn stop_still_fires_on_passive_considered_marker() {
    use super::check_stop_interrogation;
    // A passive "considered" line is NOT a fix — the gate must still drive action.
    let msg = "Done — the lease claim is now atomic.\n\
               Loopholes considered: concurrency might be an issue.";
    assert!(
        check_stop_interrogation(msg, true).is_some(),
        "passive consideration does not satisfy the fix-first gate"
    );
}

#[test]
fn stop_silent_on_trivial_turn() {
    use super::check_stop_interrogation;
    let msg = "Done — renamed a variable and fixed a typo.";
    assert!(check_stop_interrogation(msg, true).is_none());
}
