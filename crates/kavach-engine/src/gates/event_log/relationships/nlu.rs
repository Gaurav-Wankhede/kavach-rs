//! NLU/regex dependency extraction from prose — the declarative-edge companion
//! to frontmatter + wikilinks. Closes the [RCA] in `graph/relationships.rs`
//! (4235 nodes, ~0 edges): the Jaccard `infer_relationships` pass misses
//! INTENDED dependencies a human stated in plain language. This scanner lifts
//! "after X", "depends on Y", "blocked by Z", "requires W" → typed edges so the
//! DAG scheduler has real arcs to topologically order, not just similarity.
//!
//! ALGO: precompiled-regex linear scan | `PROBLEM_CLASS`: information-extraction
//! TIME: `O(p·n)` p=patterns n=content len | SPACE: `O(m)` matches | YEAR: 2026
//! SOURCE: <https://docs.rs/regex/latest/regex/#syntax>
use super::ExtractedRelationship;
use regex::Regex;
use std::sync::LazyLock;
/// Each pattern maps a natural-language dependency cue to its edge type. The
/// capture group `key` is the referenced card key/qname. Case-insensitive;
/// the key token is `[\w./-]+` (slug chars + qname separators), so both bare
/// keys (`dag-gap1`) and qnames (`proj/roadmap/dag-gap1`) are captured.
static NLU_RULES: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    let key = r"(?P<key>[\w./-]+)";
    #[expect(
        clippy::expect_used,
        reason = "patterns are compile-time-constant literals; a malformed regex is a build-time bug, not a runtime path"
    )]
    let build = |verb: &str| {
        Regex::new(&format!(r"(?i)\b{verb}\s+(?:on\s+|by\s+)?{key}")).expect("static NLU regex")
    };
    vec![
        // "depends on X", "dependent on X"
        ("depends_on", build(r"depend(?:s|ent)?")),
        // "blocked by X", "blocks X"
        ("blocks", build(r"block(?:ed|s)?")),
        // "requires X", "required X"
        ("depends_on", build(r"require(?:s|d)?")),
        // "after X" (sequencing) → X must finish first → depends_on X
        ("depends_on", build(r"after")),
        // "supersedes X", "replaces X"
        ("supersedes", build(r"super(?:sede|cede)s?")),
        ("supersedes", build(r"replace(?:s|d)?")),
    ]
});
/// Extract NLU dependency edges from prose. Skips frontmatter/wikilink regions
/// (those are handled by the dedicated extractors) by only matching cue verbs.
/// Targets are bare keys or qnames; the caller resolves bare keys to the
/// current project+category, same as frontmatter directives.
pub(super) fn extract_nlu_rels(content: &str, out: &mut Vec<ExtractedRelationship>) {
    for (rel, re) in &*NLU_RULES {
        for caps in re.captures_iter(content) {
            if let Some(m) = caps.name("key") {
                let target = m.as_str().trim_end_matches(['.', ',', ';', ')']);
                if is_plausible_key(target) {
                    let mut edge = ExtractedRelationship::new((*rel).to_owned(), target);
                    edge.speculative = true;
                    out.push(edge);
                }
            }
        }
    }
}
/// A target is plausible only if it looks like a card key, not an English
/// stop-word the verb happened to precede ("after that", "requires it"). A key
/// has a slug separator (`-`, `/`, `.`) OR is a long single token; bare common
/// words are rejected to keep precision high (false edges poison the DAG).
fn is_plausible_key(t: &str) -> bool {
    // Bare common words the verb may precede ("after that", "requires it") —
    // rejected to keep precision high (false edges poison the DAG).
    const STOP: &[&str] = &[
        "the",
        "that",
        "this",
        "it",
        "them",
        "a",
        "an",
        "all",
        "some",
        "any",
        "completion",
        "review",
        "approval",
        "merge",
        "build",
        "tests",
        "you",
        "me",
        "us",
    ];
    if t.is_empty() || t.len() < 3 {
        return false;
    }
    let has_sep = t.contains('-') || t.contains('/') || t.contains('.');
    if has_sep {
        return true;
    }
    !STOP.contains(&t.to_lowercase().as_str())
}
#[cfg(test)]
#[path = "nlu_test.rs"]
mod tests;
