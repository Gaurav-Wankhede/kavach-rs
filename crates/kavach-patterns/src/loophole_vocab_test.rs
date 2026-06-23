//! Red-Green proofs: the loophole vocab is tech-stack AGNOSTIC + a fail-closed floor.
//! SOURCE: decision.loophole-mistake-umbrella + decision.w5 (detector floor in-binary).

use super::{LoopholeVocab, dimension_for_marker, fired_dimensions};

#[test]
fn floor_is_never_empty() {
    // w5 fail-closed: the compiled floor must always detect SOMETHING on DB outage.
    let v = LoopholeVocab::default();
    assert!(!v.trigger_markers().is_empty(), "trigger floor must not be empty");
    assert!(!v.dimensions.is_empty(), "dimension floor must not be empty");
}

#[test]
fn injection_is_language_agnostic() {
    // The SAME injection lens fires across Rust, Python, JS, Java, Go — not one stack.
    let v = LoopholeVocab::default();
    for tok in ["sqlx::query", "os.system", "child_process.exec", "Runtime.exec", "exec.Command"] {
        assert_eq!(
            dimension_for_marker(&v, tok).as_deref(),
            Some("injection"),
            "{tok} must map to injection across languages"
        );
    }
}

#[test]
fn ssrf_is_language_agnostic() {
    let v = LoopholeVocab::default();
    for tok in ["reqwest::get", "fetch(", "axios.get", "requests.get", "urllib.request", "http.Get"] {
        assert_eq!(dimension_for_marker(&v, tok).as_deref(), Some("ssrf"), "{tok} -> ssrf");
    }
}

#[test]
fn xss_dimension_exists_and_is_agnostic() {
    // NEW dimension (CWE-79 #1 2025). dangerouslySetInnerHTML/v-html/innerHTML/Markup.
    let v = LoopholeVocab::default();
    for tok in ["innerHTML", "dangerouslySetInnerHTML", "v-html", "dangerous_inner_html"] {
        assert_eq!(dimension_for_marker(&v, tok).as_deref(), Some("xss"), "{tok} -> xss");
    }
}

#[test]
fn memory_safety_dimension_exists() {
    // NEW dimension (CWE-787/125/416/120-122 2025). unsafe/memcpy/raw-pointer across langs.
    let v = LoopholeVocab::default();
    for tok in ["unsafe", "memcpy", "get_unchecked", "strcpy", "unsafe.Pointer"] {
        assert_eq!(dimension_for_marker(&v, tok).as_deref(), Some("memory-safety"), "{tok}");
    }
}

#[test]
fn fired_dimensions_dedup_preserves_order() {
    let v = LoopholeVocab::default();
    // Two injection tokens + one ssrf token -> "injection, ssrf" (deduped, ordered).
    let dims = fired_dimensions(&v, &["os.system", "exec.Command", "requests.get"]);
    assert_eq!(dims, "injection, ssrf");
}

#[test]
fn unknown_marker_is_general() {
    let v = LoopholeVocab::default();
    assert_eq!(dimension_for_marker(&v, "xyzzy_not_a_marker"), None);
    assert_eq!(fired_dimensions(&v, &[]), "general");
}

#[test]
fn graph_overlay_adds_dimension_floor_intact() {
    // ADDITIVE: a project registers a new dimension; the floor injection/ssrf survive.
    let mut v = LoopholeVocab::default();
    v.dimensions.push(super::DimensionRule {
        label: "prompt-injection".to_owned(),
        lens_query: "llm prompt-injection jailbreak loophole lens".to_owned(),
        markers: vec!["system_prompt".to_owned(), "ignore previous".to_owned()],
    });
    assert_eq!(dimension_for_marker(&v, "ignore previous").as_deref(), Some("prompt-injection"));
    assert_eq!(dimension_for_marker(&v, "os.system").as_deref(), Some("injection"), "floor intact");
}

#[test]
fn malformed_overlay_degrades_to_floor() {
    // serde(default): an empty JSON object yields the full compiled floor.
    let v: LoopholeVocab = serde_json::from_str("{}").expect("empty obj valid");
    assert!(!v.dimensions.is_empty() && !v.trigger_markers().is_empty());
    // injection floor still present after a degenerate overlay.
    assert_eq!(dimension_for_marker(&v, "os.system").as_deref(), Some("injection"));
}
