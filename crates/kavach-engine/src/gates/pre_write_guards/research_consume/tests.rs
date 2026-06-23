//! FP-bound + fail-closed-block proofs for the internet-first research gate.

use super::{content_has_evidence, LOCAL_ANALYSIS_INTENTS};
use crate::gates::pre_write_context::WriteContext;
use kavach_session::SessionState;

fn ctx<'a>(path: &'a str, content: &'a str) -> WriteContext<'a> {
    WriteContext {
        file_path: path,
        tool_name: "Write",
        content,
        effective_content: content.to_owned(),
        is_code: kavach_patterns::is_rust_file(path),
        is_test: crate::gates::pre_write_checks::is_test_or_exempt(path),
        is_rust: kavach_patterns::is_rust_file(path),
        is_frontend: false,
    }
}

fn session_needing_research() -> SessionState {
    let mut s = SessionState::default();
    s.research_topic = "axum 0.8 middleware".to_owned();
    s.intent_type = "implement".to_owned();
    s.research_done = false;
    s
}

#[test]
fn resolves_with_advisory_when_research_required_and_no_evidence() {
    // The write is NOT suppressed: the gate RESOLVES on the spot — it attaches a
    // research advisory (kicked / inflight / resolved) and the write proceeds.
    let s = session_needing_research();
    let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
    let advisory = super::check(&c, &s).expect("must attach a resolve advisory");
    assert!(
        advisory.contains("[RESEARCH_KICKED]")
            || advisory.contains("[RESEARCH_INFLIGHT]")
            || advisory.contains("[RESEARCH_RESOLVED]"),
        "advisory must drive the Internet, never block: {advisory}"
    );
    assert!(!advisory.contains("BLOCKED"), "the gate must never suppress the write");
}

#[test]
fn allows_when_content_cites_source_url() {
    let s = session_needing_research();
    let c = ctx(
        "crates/foo/src/lib.rs",
        "// SOURCE: https://docs.rs/axum\nfn handler() {}",
    );
    assert!(super::check(&c, &s).is_none(), "URL evidence must clear the gate");
}

#[test]
fn allows_test_files() {
    let s = session_needing_research();
    let c = ctx("crates/foo/src/lib_test.rs", "fn t() {}");
    assert!(super::check(&c, &s).is_none(), "test files exempt");
}

#[test]
fn allows_non_code_files() {
    let s = session_needing_research();
    let c = ctx("README.md", "# docs");
    assert!(super::check(&c, &s).is_none(), "non-code exempt");
}

#[test]
fn allows_when_no_research_required() {
    let mut s = SessionState::default();
    s.research_topic.clear();
    s.intent_type = "implement".to_owned();
    let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
    assert!(super::check(&c, &s).is_none(), "no topic ⇒ nothing to enforce");
}

#[test]
fn allows_local_analysis_intents() {
    for intent in LOCAL_ANALYSIS_INTENTS {
        let mut s = session_needing_research();
        s.intent_type = intent.to_owned();
        let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
        assert!(
            super::check(&c, &s).is_none(),
            "intent {intent} inspects local code, no external research"
        );
    }
}

#[test]
fn bypass_env_disables_the_gate() {
    // SAFETY note: this test reads the env var; it does not mutate it. The
    // unset path is covered by the resolve-advisory test above (no var set).
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        let s = session_needing_research();
        let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
        assert!(super::check(&c, &s).is_none());
    }
}

#[test]
fn content_evidence_recognizes_all_markers() {
    assert!(content_has_evidence("see https://x.com"));
    assert!(content_has_evidence("see http://x.com"));
    assert!(content_has_evidence("[RESEARCH] findings"));
    assert!(content_has_evidence("research(https://x)"));
    assert!(content_has_evidence("// SOURCE: rfc"));
    assert!(!content_has_evidence("plain code, no citation"));
}
