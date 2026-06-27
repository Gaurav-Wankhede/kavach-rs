//! Research-topic extraction from a prompt.

/// Stop-word/instruction openers — a prompt starting with one is steering, not a topic.
const FILLER_OPENERS: &[&str] = &[
    "as", "the", "you", "here", "now", "so", "this", "that", "it", "we", "i", "let", "please",
    "also", "then", "but", "and", "if", "when", "because", "while", "they", "he", "she",
];

/// Research topic from `prompt`, or `""` when it opens with instruction filler.
pub(crate) fn extract_research_topic(prompt: &str, _intent_type: &str) -> String {
    let words: Vec<&str> = prompt.split_whitespace().take(6).collect();
    let Some(first) = words.first() else {
        return String::new();
    };
    let lead = first
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if FILLER_OPENERS.contains(&lead.as_str()) {
        return String::new();
    }
    words.join(" ")
}

#[cfg(test)]
#[path = "research_test.rs"]
mod tests;
