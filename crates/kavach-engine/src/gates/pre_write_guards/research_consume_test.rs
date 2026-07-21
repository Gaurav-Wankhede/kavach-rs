//! FP-bound + fail-closed-block proofs for the internet-first research gate.

use super::{LOCAL_ANALYSIS_INTENTS, content_has_evidence};
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
fn blocks_when_research_required_and_no_evidence() {
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return;
    }
    let mut s = session_needing_research();
    let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
    let block = super::check(&c, &mut s).expect("must BLOCK the unsourced write");
    assert!(
        block.contains("[RESEARCH_EVIDENCE]"),
        "action-imperative block tag: {block}"
    );
    assert!(
        block.contains("// SOURCE: <url-you-read>"),
        "names the satisfaction mechanic: {block}"
    );
    assert!(
        block.contains("RETRY this write"),
        "closes with the retry imperative: {block}"
    );
}

#[test]
fn allows_comment_only_edit_without_source() {
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return;
    }
    let mut s = session_needing_research();
    let c = ctx(
        "crates/foo/src/lib.rs",
        "/// Resolve the `subject_ids` into recipients.",
    );
    assert!(
        super::check(&c, &mut s).is_none(),
        "comment-only edit must be exempt from research enforcement"
    );
}

#[test]
fn allows_when_content_cites_source_url() {
    let mut s = session_needing_research();
    let c = ctx(
        "crates/foo/src/lib.rs",
        "// SOURCE: https://docs.rs/axum\nfn handler() {}",
    );
    assert!(
        super::check(&c, &mut s).is_none(),
        "URL evidence must clear the gate"
    );
}

#[test]
fn allows_test_files() {
    let mut s = session_needing_research();
    let c = ctx("crates/foo/src/lib_test.rs", "fn t() {}");
    assert!(super::check(&c, &mut s).is_none(), "test files exempt");
}

#[test]
fn allows_non_code_files() {
    let mut s = session_needing_research();
    let c = ctx("README.md", "# docs");
    assert!(super::check(&c, &mut s).is_none(), "non-code exempt");
}

#[test]
fn allows_when_no_research_required() {
    let mut s = SessionState::default();
    s.research_topic.clear();
    s.intent_type = "implement".to_owned();
    let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
    assert!(
        super::check(&c, &mut s).is_none(),
        "no topic ⇒ nothing to enforce"
    );
}

#[test]
fn allows_local_analysis_intents() {
    for intent in LOCAL_ANALYSIS_INTENTS {
        let mut s = session_needing_research();
        s.intent_type = intent.to_owned();
        let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
        assert!(
            super::check(&c, &mut s).is_none(),
            "intent {intent} inspects local code, no external research"
        );
    }
}

#[test]
fn bypass_env_disables_the_gate() {
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        let mut s = session_needing_research();
        let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
        assert!(super::check(&c, &mut s).is_none());
    }
}

#[test]
fn circuit_breaker_trips_after_repeated_blocks() {
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return;
    }
    let mut s = session_needing_research();
    let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
    // First 3 blocks should return Some(block_reason)
    for _ in 0..3 {
        assert!(
            super::check(&c, &mut s).is_some(),
            "block expected before circuit breaker trips"
        );
    }
    // 4th call should force-allow (circuit breaker tripped)
    assert!(
        super::check(&c, &mut s).is_none(),
        "circuit breaker must force-allow after threshold"
    );
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
