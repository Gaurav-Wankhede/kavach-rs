use super::check_loophole_interrogation;

#[test]
fn fires_on_done_claim_touching_risk_path() {
    let c = "Done — the lease acquire is now atomic and the claim is race-free.";
    let out = check_loophole_interrogation(c).expect("should fire");
    // RESOLVE-not-handback: surfaces the risk surface + lenses for awareness; no
    // CTA to manually walk lenses or narrate a `Loopholes closed:` line.
    assert!(out.contains("[LOOPHOLE_SURFACE]"), "awareness tag, not a CTA: {out}");
    assert!(out.contains("concurrency"), "names the relevant lens: {out}");
    assert!(!out.contains("RUN each lens"), "no handback CTA: {out}");
    assert!(!out.contains("Loopholes closed:"), "no narration demand: {out}");
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

// ---- M4: bounded changed-file lens detector ----

#[test]
fn site_scan_names_concrete_lens_file_line() {
    use super::scan_changed_for_loopholes;
    let files = [("crates/x/src/y.rs", "fn ok() {}\nlet v = parse(i).unwrap();")];
    let out = scan_changed_for_loopholes(&files).expect("should flag the unwrap");
    assert!(out.contains("[LOOPHOLE_SITES]"));
    assert!(out.contains("malformed crates/x/src/y.rs:2"), "concrete site: {out}");
}

#[test]
fn site_scan_silent_on_clean_files() {
    use super::scan_changed_for_loopholes;
    let files = [("a.rs", "let s = a.checked_add(b)?;\nfn pure() {}")];
    assert!(scan_changed_for_loopholes(&files).is_none(), "no hints -> no advisory");
}

#[test]
fn site_scan_silent_on_empty_input() {
    use super::scan_changed_for_loopholes;
    assert!(scan_changed_for_loopholes(&[]).is_none());
}

#[test]
fn site_scan_caps_files_and_names_the_drop() {
    use super::scan_changed_for_loopholes;
    // 30 files each with a finding > MAX_FILES_SCANNED (24): the advisory must
    // name the truncation, never silently drop (boundary lens on the gate itself).
    let owned: Vec<(String, String)> = (0..30)
        .map(|i| (format!("f{i}.rs"), "let v = x.unwrap();".to_owned()))
        .collect();
    let refs: Vec<(&str, &str)> = owned.iter().map(|(p, c)| (p.as_str(), c.as_str())).collect();
    let out = scan_changed_for_loopholes(&refs).expect("should flag");
    assert!(out.contains("scanned 24/30"), "names the file cap: {out}");
}

#[test]
fn site_scan_caps_listed_sites_and_names_the_remainder() {
    // One file with 20 findings > MAX_SITES_LISTED (12): list is bounded, the
    // remainder count is surfaced (no silent cap).
    let body: String = (0..20).map(|_| "let v = x.unwrap();\n").collect();
    let out = super::scan_changed_for_loopholes(&[("big.rs", &body)]).expect("flags");
    assert!(out.contains("more suspected site(s)"), "names dropped sites: {out}");
}

#[test]
fn changed_filter_excludes_test_files() {
    use super::is_scannable_rust;
    assert!(is_scannable_rust("crates/x/src/y.rs"));
    assert!(!is_scannable_rust("crates/x/src/y_tests.rs"));
    assert!(!is_scannable_rust("crates/x/src/y_test.rs"));
    assert!(!is_scannable_rust("crates/x/src/y_test_menu.rs"));
    assert!(!is_scannable_rust("crates/x/tests/it.rs"));
    assert!(!is_scannable_rust("crates/x/src/y.toml"));
    assert!(is_scannable_rust("crates/x/src/latest.rs"), "non-test stem stays in scope");
}
