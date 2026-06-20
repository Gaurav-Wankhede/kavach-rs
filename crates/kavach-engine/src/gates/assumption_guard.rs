//! Detect assumption/hallucination phrases in written content.
//!
//! Blocks content containing "I think", "I believe", "based on my knowledge"
//! and similar phrases that indicate ungrounded LLM output.

use crate::gates::directive_cache::dyn_directive;

/// Phrases that indicate ungrounded assumptions.
const ASSUMPTION_PHRASES: &[&str] = &[
    "i think",
    "i believe",
    "i assume",
    "based on my knowledge",
    "based on my training",
    "from my understanding",
    "as far as i know",
    "i recall that",
    "if i remember correctly",
    "i'm fairly certain",
    "i'm pretty sure",
    "most likely",
    "presumably",
    "in my experience",
];

/// True when `phrase` occurs in `haystack` as a whole token sequence — i.e. the
/// chars flanking the match are non-alphanumeric (or string edges). A raw
/// `contains` false-positives on partial-token hits (e.g. "i think" inside a
/// hypothetical identifier, or "presumably" as a substring of a longer word);
/// the boundary check restricts the gate to genuine prose assertions.
fn contains_phrase(haystack: &str, phrase: &str) -> bool {
    let plen = phrase.len();
    haystack.match_indices(phrase).any(|(start, _)| {
        // Flanking chars via UTF-8-safe slices (`get` returns None on a non-boundary
        // index, which can't happen here since match_indices returns char boundaries).
        let before_ok = haystack
            .get(..start)
            .and_then(|s| s.chars().next_back())
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = start
            .checked_add(plen)
            .and_then(|end| haystack.get(end..))
            .and_then(|s| s.chars().next())
            .is_none_or(|c| !c.is_alphanumeric());
        before_ok && after_ok
    })
}

/// Check written content for assumption phrases.
/// Returns Some(warning) if assumptions detected.
pub(crate) fn check_for_assumptions(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    let mut found: Vec<&str> = Vec::new();

    for phrase in ASSUMPTION_PHRASES {
        if contains_phrase(&lower, phrase) {
            found.push(phrase);
        }
    }

    if found.is_empty() {
        return None;
    }

    // Tag + matched phrases literal; the remediation imperative is research-refreshed.
    let remedy = dyn_directive(
        "assumption.grounding-remedy",
        "WebSearch to verify the claim, then cite the source explicitly. \
         Replace with: \"According to [source]\", \"Docs show\", \"Verified via\".",
    );
    Some(format!(
        "[ASSUMPTION_DETECTED]\n\
         Remove ungrounded phrases: {}\n\
         {remedy}",
        found.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_content_passes() {
        assert!(check_for_assumptions("fn main() {}").is_none());
    }

    #[test]
    fn assumption_phrase_detected() {
        let r = check_for_assumptions("I think this API uses OAuth2");
        assert!(r.is_some());
        assert!(r.unwrap().contains("ASSUMPTION_DETECTED"));
    }

    #[test]
    fn multiple_phrases_all_reported() {
        let r = check_for_assumptions(
            "I believe the endpoint is /api/v2. Based on my knowledge it uses JWT.",
        );
        let msg = r.unwrap();
        assert!(msg.contains("i believe"));
        assert!(msg.contains("based on my knowledge"));
    }

    #[test]
    fn code_comments_with_assumptions_caught() {
        let r = check_for_assumptions("// I think this might cause a race condition");
        assert!(r.is_some());
    }

    #[test]
    fn partial_token_does_not_false_positive() {
        // "presumably" must not fire when it is a substring of a longer token,
        // and "i think" must not fire glued to surrounding alphanumerics.
        assert!(check_for_assumptions("let presumablyx = compute();").is_none());
        assert!(check_for_assumptions("xpresumably = 1;").is_none());
        assert!(check_for_assumptions("the ithinker module").is_none());
        // But the real standalone phrase still fires.
        assert!(check_for_assumptions("presumably the cache is warm").is_some());
        assert!(check_for_assumptions("i think so").is_some());
    }
}
