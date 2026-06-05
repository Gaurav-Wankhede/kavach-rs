//! Research-topic extraction from a prompt + intent type.

/// Extract research topic from prompt and intent type.
pub(crate) fn extract_research_topic(prompt: &str, intent_type: &str) -> String {
    let words: Vec<&str> = prompt.split_whitespace().take(6).collect();
    if words.is_empty() {
        return intent_type.to_owned();
    }
    words.join(" ")
}
