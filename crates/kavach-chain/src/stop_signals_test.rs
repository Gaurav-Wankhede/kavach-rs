use super::*;

#[test]
fn all_signal_patterns_compile() {
    assert!(detect_strategic_deferral("add post-launch").unwrap());
    assert!(detect_value_gating("adds zero value until later").unwrap());
    assert!(detect_self_imposed_limit("running low on context").unwrap());
    assert!(detect_unsolicited_reprioritization("the priority should be X").unwrap());
    assert!(detect_continuation_menu("say continue and I'll proceed").unwrap());
    assert!(detect_strong_scope_ask("what would you like?").unwrap());
    assert!(detect_sycophancy("great work!").unwrap());
    assert!(detect_false_inability("I can't access that").unwrap());
    assert!(detect_incomplete_work("I'll leave that to you").unwrap());
    assert!(detect_remaining_phases("remaining phase is X").unwrap());
    assert!(detect_parallel_system("in parallel with other systems").unwrap());
    assert!(detect_passive_info_request("let me know when done").unwrap());
    assert!(detect_research_only_stop("here's what is happening with the issue", false).unwrap());
    assert!(detect_deferred_dismissal("could check later").unwrap());
    assert!(detect_user_report_dismissal("you reported the issue").unwrap());
    assert!(detect_summary_exit("to summarize").unwrap());
    assert!(detect_permission_seek("should I proceed?").unwrap());
    assert!(detect_unverified_code_claim("the code is working now").unwrap());
}

// Continuation-menu detector tests live in a sibling file to keep each test
// file under the micro-file ceiling; included here as a nested submodule.
#[path = "stop_signals_test_menu.rs"]
mod menu;

#[test]
fn strategic_deferral_paraphrases() {
    for m in [
        "let's revisit this after we've shipped",
        "this belongs in phase three honestly",
        "make it a v3 milestone",
        "premature to optimize this right now",
        "noted for a future iteration",
    ] {
        assert!(detect_strategic_deferral(m).unwrap(), "missed: {m}");
    }
}

#[test]
fn strategic_deferral_suppressed_when_doing_it() {
    assert!(!detect_strategic_deferral("was post-launch but building now").unwrap());
    assert!(!detect_strategic_deferral("let me implement the post-launch charts").unwrap());
    assert!(!detect_strategic_deferral("post-launch item, implementing it anyway").unwrap());
}

#[test]
fn value_gating_paraphrases_and_negation() {
    assert!(detect_value_gating("the chart would just be empty boxes").unwrap());
    assert!(detect_value_gating("good enough for now, skip it").unwrap());
    assert!(detect_value_gating("once there are multiple campaigns").unwrap());
    assert!(!detect_value_gating("adds zero value but building it anyway").unwrap());
}

#[test]
fn self_limit_paraphrases_and_escapes() {
    assert!(detect_self_imposed_limit("this is 4 hours of focused work").unwrap());
    assert!(detect_self_imposed_limit("splitting this across sessions").unwrap());
    assert!(!detect_self_imposed_limit("the stop_detect gate catches this").unwrap());
    assert!(!detect_self_imposed_limit("stopping as you requested due to context limit").unwrap());
}

#[test]
fn reprioritize_paraphrases_and_alignment() {
    assert!(detect_unsolicited_reprioritization("better to focus on the core API first").unwrap());
    assert!(
        !detect_unsolicited_reprioritization("as you requested, the priority right now is charts")
            .unwrap()
    );
}

#[test]
fn permission_seek_exempts_user_directed_asks() {
    // Bare permission-seek still fires.
    assert!(detect_permission_seek("should I proceed with the migration?").unwrap());
    // The broadened NEG arm exempts genuine user-delegated asks — these are the
    // counterexamples surfaced when the detector was wired into the Stop gate.
    for legit in [
        "you asked me to choose, so should I proceed with option A?",
        "this is your decision — should I continue?",
        "you directed me to confirm before each step; may I proceed?",
        "per your request, should I proceed to the next handler?",
    ] {
        assert!(
            !detect_permission_seek(legit).unwrap(),
            "user-directed ask must be exempt: {legit}"
        );
    }
}

#[test]
fn unverified_code_with_code_block_is_exempted() {
    assert!(
        !detect_unverified_code_claim(
            "Not yet built. The current code:\n```\nfn handle() { unfinished() }\n```"
        )
        .unwrap()
    );
}

#[test]
fn empty_is_inert() {
    assert!(!detect_strategic_deferral("").unwrap());
    assert!(!detect_value_gating("").unwrap());
    assert!(!detect_self_imposed_limit("").unwrap());
    assert!(!detect_unsolicited_reprioritization("").unwrap());
    assert!(!detect_continuation_menu("").unwrap());
    assert!(!detect_strong_scope_ask("").unwrap());
    assert!(!detect_sycophancy("").unwrap());
    assert!(!detect_false_inability("").unwrap());
    assert!(!detect_incomplete_work("").unwrap());
    assert!(!detect_remaining_phases("").unwrap());
    assert!(!detect_parallel_system("").unwrap());
    assert!(!detect_passive_info_request("").unwrap());
    assert!(!detect_research_only_stop("", false).unwrap());
    assert!(!detect_deferred_dismissal("").unwrap());
    assert!(!detect_user_report_dismissal("").unwrap());
    assert!(!detect_summary_exit("").unwrap());
    assert!(!detect_permission_seek("").unwrap());
    assert!(!detect_unverified_code_claim("").unwrap());
}
