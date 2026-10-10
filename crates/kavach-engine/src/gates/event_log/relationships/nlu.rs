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
/// word the verb happened to precede. A key has a slug separator (`-`, `/`,
/// `.`) OR is a long single token; bare common words are rejected to keep
/// precision high (false edges poison the DAG).
///
/// A hand-rolled 17-word stop list was NOT enough: live kavach-rs data carried
/// speculative edges to `their`, `dedup`, `every`, `model`, `rename`, `binary`,
/// `hooks` — ordinary prose words that followed a cue verb ("after the parent
/// did ...", "requires model ...", "blocks rename ..."). Every one became a
/// permanent `(unresolved)` DAG node. So the gate is now STRUCTURAL, not lexical:
///
/// 1. A qname (`proj/cat/key`, ≥2 slashes) is accepted — it cannot be prose.
/// 2. Otherwise the token MUST carry a slug separator (`-`/`.`) OR be ≥4 chars
///    AND camelCase/snake/digit-bearing — a real card key is kebab/snake/dotted
///    (`dag-gap1`, `v0_3_cli`, `stage1-backend`), whereas English prose is
///    lowercase alphabetic.
/// 3. Single lowercase alphabetic tokens are rejected outright unless they carry
///    a digit (e.g. `g1a`, `p9`), which prose essentially never does in a key
///    position.
fn is_plausible_key(t: &str) -> bool {
    if t.is_empty() || t.len() < 3 {
        return false;
    }
    // A qualified qname is unambiguous by construction.
    if t.matches('/').count() >= 2 {
        return true;
    }
    // Kebab / dotted / snake keys are the house convention.
    if t.contains('-') || t.contains('.') || t.contains('_') {
        return true;
    }
    // No separator: only accept if it looks deliberately key-shaped — a digit
    // (`g1a`, `w2`), or internal case (`dagGap1`). Prose words are flat
    // lowercase alphabetic and get rejected here.
    let has_digit = t.chars().any(|c| c.is_ascii_digit());
    let has_inner_upper = t.chars().skip(1).any(char::is_uppercase);
    has_digit || has_inner_upper
}
#[cfg(test)]
#[path = "nlu_test.rs"]
mod tests;
