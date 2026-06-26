//! Context-derived research topic: the system decides WHAT to research from the
//! actual work (intent kind + prompt salient tokens), never a hardcoded list.
//!
//! No fixed phrasing per intent — the angle is built from the prompt's own
//! salient tokens plus a short, intent-shaped lens word. The temporal scope is
//! supplied by the caller's live `now_full()`, so nothing here is date-baked.

/// Stopwords stripped before picking salient tokens — generic filler that never
/// sharpens a search. NOT a topic allow-list (that would be the hardcoded tone
/// we are removing); purely noise removal so the salient terms surface.
const STOPWORDS: &[&str] = &[
    "this", "that", "with", "from", "have", "been", "the", "and", "for", "into", "your", "you",
    "are", "but", "not", "make", "sure", "also", "then", "when", "what", "will", "can", "should",
    "would", "could", "please", "need", "want",
];

/// A short lens word per intent KIND — shapes HOW to search, not WHAT (the what
/// comes from the prompt). Unknown intents fall through to a neutral lens.
fn lens(intent_type: &str) -> &'static str {
    match intent_type {
        "implement" => "implementation contract + edge cases",
        "debug" | "bugfix" => "root cause + known issues",
        "security" => "current advisories + secure pattern",
        "deploy" => "production deployment pitfalls",
        "refactor" => "current idiomatic pattern",
        "memory" => "persistence/retrieval architecture",
        _ => "current authoritative contract",
    }
}

/// Derive a research angle from the live work. Picks up to 4 salient prompt
/// tokens (len > 3, not a stopword) and appends an intent-shaped lens. Returns a
/// neutral lens-only string when the prompt yields no salient tokens, so the
/// advisory is never empty and never invents a fake topic.
#[must_use]
pub(crate) fn derive(intent_type: &str, prompt: &str) -> String {
    let salient: Vec<&str> = prompt
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| w.len() > 3 && !STOPWORDS.contains(&w.to_lowercase().as_str()))
        .take(4)
        .collect();
    let lens = lens(intent_type);
    if salient.is_empty() {
        lens.to_owned()
    } else {
        format!("{} — {lens}", salient.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::derive;

    #[test]
    fn derives_from_prompt_tokens_not_a_fixed_list() {
        let out = derive(
            "implement",
            "add scylla request coalescing to the chat handler",
        );
        assert!(
            out.contains("scylla"),
            "salient prompt token must drive the topic: {out}"
        );
        assert!(out.contains("coalescing"));
        assert!(
            out.contains("implementation"),
            "intent lens appended: {out}"
        );
    }

    #[test]
    fn intent_shapes_the_lens() {
        assert!(derive("security", "validate jwt").contains("advisories"));
        assert!(derive("debug", "fix panic").contains("root cause"));
        // Unknown intent → neutral lens, never a panic.
        assert!(!derive("totally-unknown", "foo bar baz").is_empty());
    }

    #[test]
    fn empty_or_filler_prompt_yields_lens_only_never_empty() {
        // All-stopword prompt → no salient tokens → lens-only, not empty, no fake topic.
        let out = derive("refactor", "make sure you can");
        assert_eq!(out, "current idiomatic pattern");
        assert!(!derive("implement", "").is_empty());
    }
}
