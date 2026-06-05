//! P0 structural detector tests (catch-all/indexing/dead_code/serde/empty-fn)
//! plus the TF-IDF cosine-similarity false-positive regression lock.
use crate::config::j;
use crate::rust_guard::{RustSeverity, detect};

#[test]
fn p0_catch_all() {
    let v = detect("src/lib.rs", "        _ => {}");
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern.contains("catch-all"))
    );
}

#[test]
fn p0_direct_indexing() {
    let v = detect("src/lib.rs", "let x = v[i];");
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "direct indexing")
    );
}

#[test]
fn p0_allow_dead_code() {
    let code = j(&["#[allow(dead_co", "de)]\nfn f() {}"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "allow(dead_code)")
    );
}

#[test]
fn p0_serde_default_bool() {
    let code = j(&["#[serde(defa", "ult)]\n    pub is_admin: bool"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern.contains("serde"))
    );
}

#[test]
fn p0_empty_fn_body() {
    let v = detect("src/lib.rs", "pub fn handle_request() {}");
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "empty function body")
    );
}

#[test]
fn p0_empty_fn_with_return_type() {
    let v = detect("src/lib.rs", "fn process(input: &str) -> bool {}");
    assert!(v.iter().any(|x| x.pattern == "empty function body"));
}

#[test]
fn nonempty_fn_ok() {
    let v = detect("src/lib.rs", "fn process() { do_work(); }");
    assert!(!v.iter().any(|x| x.pattern == "empty function body"));
}

#[test]
fn p0_allow_unused() {
    let code = j(&["#[allow(unus", "ed)]"]);
    let v = detect("src/lib.rs", &code);
    assert!(
        v.iter()
            .any(|x| x.severity == RustSeverity::P0Block && x.pattern == "allow(unused)")
    );
}

#[test]
fn p0_no_false_positive_on_tfidf_cosine_similarity() {
    // Regression: a stale pattern false-positived on TF-IDF cosine similarity.
    let content = include_str!("tfidf_fixture.txt");
    let v = detect("src/gate_patterns.rs", content);
    let p0: Vec<_> = v
        .iter()
        .filter(|x| x.severity == RustSeverity::P0Block)
        .collect();
    assert!(p0.is_empty(), "P0 violations found: {p0:?}");
}
