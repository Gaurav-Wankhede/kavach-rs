//! Detect assumption/hallucination phrases in written content.
//!
//! Blocks content containing "I think", "I believe", "based on my knowledge"
//! and similar phrases that indicate ungrounded LLM output.

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

/// Check written content for assumption phrases.
/// Returns Some(warning) if assumptions detected.
pub(crate) fn check_for_assumptions(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    let mut found: Vec<&str> = Vec::new();

    for phrase in ASSUMPTION_PHRASES {
        if lower.contains(phrase) {
            found.push(phrase);
        }
    }

    if found.is_empty() {
        return None;
    }

    Some(format!(
        "[ASSUMPTION_DETECTED]\n\
         Remove ungrounded phrases: {}\n\
         WebSearch to verify the claim, then cite the source explicitly.\n\
         Replace with: \"According to [source]\", \"Docs show\", \"Verified via\"",
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
}
