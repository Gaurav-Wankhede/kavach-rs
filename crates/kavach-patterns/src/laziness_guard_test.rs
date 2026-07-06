//! Tests for the laziness guard — block lazy-recommendation, allow real direction.

use super::{detect_lazy_recommendation, detect_researchable_question};
use serde_json::json;

/// Build a single-question payload with an explicit STEM (the researchable
/// detector reads the question stem for factual forms, unlike the lazy detector).
fn ask_q(stem: &str, opts: &[(&str, &str)]) -> serde_json::Value {
    json!({ "questions": [ {
        "question": stem, "header": "h", "multiSelect": false,
        "options": opts.iter().map(|(l, d)| json!({"label": l, "description": d})).collect::<Vec<_>>()
    } ] })
}

fn ask(opts: &[(&str, &str)]) -> serde_json::Value {
    json!({ "questions": [ {
        "question": "q", "header": "h", "multiSelect": false,
        "options": opts.iter().map(|(l, d)| json!({"label": l, "description": d})).collect::<Vec<_>>()
    } ] })
}

#[test]
fn blocks_recommended_leave_as_is_over_rebuild() {
    let input = ask(&[
        (
            "Leave as-is (Recommended)",
            "No heavy rebuild; just record the facts.",
        ),
        (
            "Full canonical rebuild",
            "Run the proper rebuild end-to-end.",
        ),
    ]);
    let r = detect_lazy_recommendation(&input).expect("must block lazy-recommended");
    assert!(r.contains("[LAZINESS_POLICY]"));
    assert!(r.contains("division_of_labor") || r.contains("do all the labor"));
}

#[test]
fn blocks_recommended_check_back_later_over_finish() {
    let input = ask(&[
        ("Stop now, check back later (Recommended)", "defer the work"),
        ("Finish the implementation", "complete the fix this turn"),
    ]);
    assert!(detect_lazy_recommendation(&input).is_some());
}

#[test]
fn allows_genuine_direction_question() {
    // No effort asymmetry — two real architectural directions.
    let input = ask(&[
        ("PASETO v4 (Recommended)", "stateless local tokens"),
        ("JWT with rotation", "industry-standard, wide tooling"),
    ]);
    assert!(
        detect_lazy_recommendation(&input).is_none(),
        "real direction != laziness"
    );
}

#[test]
fn allows_when_recommended_is_the_work_option() {
    // The recommended option IS the do-the-work path — correct, not lazy.
    let input = ask(&[
        (
            "Full canonical rebuild (Recommended)",
            "run the proper rebuild end-to-end",
        ),
        ("Leave as-is", "skip the rebuild for now"),
    ]);
    assert!(
        detect_lazy_recommendation(&input).is_none(),
        "recommending the work path is the correct behavior"
    );
}

#[test]
fn silent_on_single_option() {
    let input = ask(&[("Leave as-is (Recommended)", "do nothing")]);
    assert!(
        detect_lazy_recommendation(&input).is_none(),
        "needs >=2 options to be a false choice"
    );
}

#[test]
fn silent_on_lazy_recommended_without_work_sibling() {
    // Both options are low-effort variants — not the labor-as-direction pattern
    // this guard targets (no do-the-work path being dodged).
    let input = ask(&[
        ("Leave as-is (Recommended)", "no change"),
        ("Defer to later", "revisit next week"),
    ]);
    assert!(detect_lazy_recommendation(&input).is_none());
}

/// Build a multi-question payload from N (label, description) option groups.
fn multi_ask(questions: &[&[(&str, &str)]]) -> serde_json::Value {
    json!({
        "questions": questions.iter().map(|opts| json!({
            "question": "q", "header": "h", "multiSelect": false,
            "options": opts.iter().map(|(l, d)| json!({"label": l, "description": d})).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

#[test]
fn cross_question_pairing_does_not_trigger() {
    // Q1 is a pure DIRECTION choice (its recommended option carries no lazy marker);
    // Q2 is a pure EFFORT choice. The OLD flattening bug would pair Q2's lazy-
    // recommended option with Q1's work option and falsely fire. Per-question
    // evaluation must NOT: neither question is a lazy-vs-work split on its own.
    let input = multi_ask(&[
        &[
            ("PASETO v4 (Recommended)", "stateless"),
            ("JWT rotation", "rebuild the auth layer"),
        ],
        &[
            ("Defer the cleanup (Recommended)", "later"),
            ("Proceed", "go ahead"),
        ],
    ]);
    assert!(
        detect_lazy_recommendation(&input).is_none(),
        "options from different questions must not cross-pair"
    );
}

#[test]
fn lazy_split_within_one_question_of_a_multi_payload_fires() {
    // First question is benign direction; the SECOND question is a genuine
    // lazy-vs-work split. Per-question evaluation must still catch it.
    let input = multi_ask(&[
        &[
            ("Option A (Recommended)", "approach a"),
            ("Option B", "approach b"),
        ],
        &[
            ("Leave as-is (Recommended)", "skip it"),
            (
                "Full canonical rebuild",
                "run the proper rebuild end-to-end",
            ),
        ],
    ]);
    assert!(
        detect_lazy_recommendation(&input).is_some(),
        "a lazy-vs-work split within any single question must fire"
    );
}

#[test]
fn new_deferral_synonyms_fire() {
    for lazy in [
        "Postpone this (Recommended)",
        "Shelve it (Recommended)",
        "Table it for now (Recommended)",
    ] {
        let input = ask(&[
            (lazy, "not now"),
            ("Finish the migration", "implement it end-to-end"),
        ]);
        assert!(
            detect_lazy_recommendation(&input).is_some(),
            "deferral synonym must be caught: {lazy}"
        );
    }
}

#[test]
fn silent_on_empty_or_malformed_input() {
    // The DETECTOR reports "no lazy pattern" on empty/malformed input — it never
    // panics. NOTE: the GATE (kavach-engine pre_tool_question.rs) is the layer that
    // FAILS CLOSED on an absent `tool_input` (deny, not allow); the detector only
    // answers "is this lazy?" on whatever options it CAN parse. Two distinct layers.
    assert!(detect_lazy_recommendation(&json!({})).is_none());
    assert!(detect_lazy_recommendation(&json!({"questions": []})).is_none());
    assert!(detect_lazy_recommendation(&json!({"questions": [{"options": []}]})).is_none());
    assert!(detect_lazy_recommendation(&json!("not an object")).is_none());
    // Hostile shapes must not panic: wrong types where strings/arrays are expected.
    assert!(detect_lazy_recommendation(&json!({"questions": "nope"})).is_none());
    assert!(detect_lazy_recommendation(&json!({"questions": [{"options": 42}]})).is_none());
    assert!(
        detect_lazy_recommendation(&json!({"questions": [{"options": [{"label": 1}]}]})).is_none()
    );
}

// ── detect_researchable_question (D4) ───────────────────────────────────────

#[test]
fn fires_on_researchable_library_choice() {
    // "Which library — Reqwest or Hyper?" is internet-answerable, not the user's call.
    let input = ask_q(
        "Which HTTP library should I use?",
        &[("Reqwest", "high-level client"), ("Hyper", "low-level")],
    );
    let r = detect_researchable_question(&input).expect("researchable question must nudge");
    assert!(r.contains("[RESEARCH_FIRST]"));
    assert!(r.contains("WebSearch") || r.contains("authoritative"));
}

#[test]
fn fires_on_api_signature_and_flag_questions() {
    for (stem, opt) in [
        (
            "What is the correct flag for cargo to check the workspace?",
            "--workspace",
        ),
        ("Which version of Dioxus supports this feature?", "0.7.9"),
        ("What is the method name on the reqwest Client?", "get"),
    ] {
        let input = ask_q(stem, &[(opt, "a"), ("other", "b")]);
        assert!(
            detect_researchable_question(&input).is_some(),
            "researchable factual question must fire: {stem}"
        );
    }
}

#[test]
fn silent_on_genuine_direction_question() {
    // A real tradeoff scoped to THIS project / priority is the user's call — never nudge.
    let input = ask_q(
        "Which approach fits our latency budget better?",
        &[
            ("In-process cache", "lower latency, more memory"),
            ("Redis", "shared, network hop"),
        ],
    );
    assert!(
        detect_researchable_question(&input).is_none(),
        "a genuine direction tradeoff must not be nudged toward WebSearch"
    );
}

#[test]
fn silent_on_authorization_question() {
    // Irreversible-action authorization is never researchable.
    let input = ask_q(
        "Should I push these commits and deploy to production?",
        &[("Push + deploy", "go live"), ("Hold", "wait")],
    );
    assert!(
        detect_researchable_question(&input).is_none(),
        "authz question is the user's call"
    );
}

#[test]
fn researchable_silent_on_empty_or_malformed() {
    assert!(detect_researchable_question(&json!({})).is_none());
    assert!(detect_researchable_question(&json!({"questions": []})).is_none());
    assert!(detect_researchable_question(&json!("nope")).is_none());
    assert!(detect_researchable_question(&json!({"questions": "x"})).is_none());
}
