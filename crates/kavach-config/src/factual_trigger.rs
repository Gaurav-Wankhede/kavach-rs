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

/// True when the prompt carries a factual signal that demands a live source
/// (a version, date, URL, registry, or price word) — internet-first fires even
/// without an implement verb. Bypass patterns are filtered upstream in
/// `requires_research`, so a non-factual refactor prompt never reaches here.
#[must_use]
pub fn contains_factual_trigger(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    FACTUAL_TRIGGERS.iter().any(|t| lower.contains(t)) || has_year(&lower) || has_semver(&lower)
}

/// A recent-year token (2024-2029) — a date prompt the stale weights can't answer.
fn has_year(text: &str) -> bool {
    ["2024", "2025", "2026", "2027", "2028", "2029"].iter().any(|y| text.contains(y))
}

/// A `digit.digit` version-ish token (e.g. `1.2`, `0.9.5`) — a version claim.
fn has_semver(text: &str) -> bool {
    text.as_bytes()
        .windows(3)
        .any(|w| matches!(w, [a, b'.', c] if a.is_ascii_digit() && c.is_ascii_digit()))
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
