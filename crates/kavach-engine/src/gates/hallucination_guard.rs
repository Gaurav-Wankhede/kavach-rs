//! Detect hallucinated URLs, fake packages, and invented API patterns.
//!
//! Scans written content for fabricated URLs (non-resolvable domains),
//! invented crate names, and suspicious API patterns that LLMs commonly
//! hallucinate during code generation.

/// URL patterns that LLMs commonly hallucinate.
const FAKE_URL_PATTERNS: &[&str] = &[
    "example.com/api",
    "api.example.com",
    "your-api.com",
    "your-domain.com",
    "your-server.com",
    "my-api.com",
    "myapp.com",
    "your-app.com",
    "localhost:3000/api",
    "placeholder.com",
    "fake-api.com",
    "test-api.com",
    "sample-api.com",
];

/// Suspicious placeholder values in code.
const PLACEHOLDER_VALUES: &[&str] = &[
    "sk-your-api-key",
    "your-api-key-here",
    "YOUR_API_KEY",
    "INSERT_KEY_HERE",
    "REPLACE_WITH_",
    "your_token_here",
    "your-secret-key",
    "changeme",
    "password123",
    "XXX_REPLACE",
    "TODO_REPLACE",
];

/// Check written content for hallucination indicators.
/// Returns Some(warning) if suspicious patterns found.
pub(crate) fn check_for_hallucinations(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    let mut issues: Vec<String> = Vec::new();

    check_fake_urls(&lower, &mut issues);
    check_placeholder_values(content, &mut issues);
    check_invented_imports(&lower, &mut issues);

    if issues.is_empty() {
        return None;
    }

    Some(format!(
        "[HALLUCINATION_WARNING]\n\
         Suspicious patterns detected:\n{}\n\
         WebSearch each URL, package name, and API endpoint to confirm it exists.\n\
         Replace placeholders with verified docs.rs/crates.io links.",
        issues.join("\n")
    ))
}

fn check_fake_urls(lower: &str, issues: &mut Vec<String>) {
    for pattern in FAKE_URL_PATTERNS {
        if lower.contains(pattern) {
            issues.push(format!("  - Fake URL pattern: {pattern}"));
        }
    }
}

fn check_placeholder_values(content: &str, issues: &mut Vec<String>) {
    for pattern in PLACEHOLDER_VALUES {
        if content.contains(pattern) {
            issues.push(format!("  - Placeholder value: {pattern}"));
        }
    }
}

fn check_invented_imports(lower: &str, issues: &mut Vec<String>) {
    if lower.contains("use ") && lower.contains("_imaginary") {
        issues.push("  - Suspicious import: contains '_imaginary'".into());
    }
    if lower.contains("from ") && lower.contains("nonexistent") {
        issues.push("  - Suspicious import: contains 'nonexistent'".into());
    }
}

#[cfg(test)]
#[path = "hallucination_guard_tests.rs"]
mod tests;
