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
    // Fail-closed: a research-required production write with no cited source is
    // DENIED at write time — no source, no claim. The block still drives the lookup
    // so the agent can cite + retry immediately. The message is action-imperative:
    // tells the agent the exact action to unblock (cite a source, then retry).
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return; // bypass set in this env ⇒ enforcement disabled (covered elsewhere)
    }
    let s = session_needing_research();
    let c = ctx("crates/foo/src/lib.rs", "fn handler() {}");
    let block = super::check(&c, &s).expect("must BLOCK the unsourced write");
    assert!(
        block.contains("[RESEARCH_FIRST:P0]"),
        "P0 block tag: {block}"
    );
    assert!(
        block.contains("CITE A SOURCE THEN RETRY"),
        "action-imperative leading phrase: {block}"
    );
    assert!(
        block.contains("No source -> no claim"),
        "states the law: {block}"
    );
}

#[test]
fn allows_comment_only_edit_without_source() {
    // A doc-comment/lint fix (e.g. backticking `subject_ids` for clippy::doc_markdown)
    // carries no factual claim, so the research gate must NOT block it — else it
    // deadlocks against the comment-noise gate on a trivial doc edit.
    if std::env::var_os("KAVACH_RESEARCH_BYPASS").is_some() {
        return;
    }
    let s = session_needing_research();
    let c = ctx(
        "crates/foo/src/lib.rs",
        "/// Resolve the `subject_ids` into recipients.",
    );
    assert!(
        super::check(&c, &s).is_none(),
        "comment-only edit must be exempt from research enforcement"
    );
}

#[test]
fn allows_when_content_cites_source_url() {
    let s = session_needing_research();
    let c = ctx(
        "crates/foo/src/lib.rs",
        "// SOURCE: https://docs.rs/axum\nfn handler() {}",
    );
    assert!(
        super::check(&c, &s).is_none(),
        "URL evidence must clear the gate"
    );
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
    assert!(
        super::check(&c, &s).is_none(),
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
