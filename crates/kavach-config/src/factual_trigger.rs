const FACTUAL_TRIGGERS: &[&str] = &[
    "version",
    "latest",
    "newest",
    "current",
    "release",
    "changelog",
    "deprecat",
    "http://",
    "https://",
    "crate",
    "npm",
    "pypi",
    "cargo add",
    "price",
    "pricing",
    "cost per",
    "rate limit",
    "api docs",
];

pub fn contains_factual_trigger(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    for trigger in FACTUAL_TRIGGERS {
        if lower.contains(trigger) {
            return true;
        }
    }
    // Check for year patterns: 2024/2025/2026/2027
    if lower.contains("202") && (lower.contains("4") || lower.contains("5") || lower.contains("6") || lower.contains("7")) {
        if rg_semver_pattern(&lower) {
            return true;
        }
    }
    // Check for semver-ish pattern: digit.digit
    semver_pattern(&lower)
}

fn semver_pattern(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            if let Some(&next) = chars.peek() {
                if next == '.' {
                    chars.next();
                    if let Some(&after_dot) = chars.peek() {
                        if after_dot.is_ascii_digit() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn rg_semver_pattern(text: &str) -> bool {
    text.contains("2024") || text.contains("2025") || text.contains("2026") || text.contains("2027")
}

#[cfg(test)]
mod factual_trigger_test {
    use super::*;

    #[test]
    fn test_factual_trigger_version() {
        assert!(contains_factual_trigger("what is the latest tokio version"));
    }

    #[test]
    fn test_factual_trigger_https() {
        assert!(contains_factual_trigger("https://x.com"));
    }

    #[test]
    fn test_factual_trigger_semver() {
        assert!(contains_factual_trigger("1.2.3"));
    }

    #[test]
    fn test_factual_trigger_year() {
        assert!(contains_factual_trigger("2025 release notes"));
    }

    #[test]
    fn test_factual_trigger_npm() {
        assert!(contains_factual_trigger("npm latest react"));
    }

    #[test]
    fn test_factual_trigger_pricing() {
        assert!(contains_factual_trigger("what is the pricing"));
    }

    #[test]
    fn test_not_factual_trigger() {
        assert!(!contains_factual_trigger("rename this variable"));
    }

    #[test]
    fn test_factual_trigger_deprecat() {
        assert!(contains_factual_trigger("is this deprecated"));
    }
}
