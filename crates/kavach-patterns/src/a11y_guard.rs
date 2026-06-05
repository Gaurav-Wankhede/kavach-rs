// split: intentional — guard module, not handler
//! Accessibility guard — detect missing alt, aria-label, label for inputs.

use regex::Regex;
use std::sync::LazyLock;

struct A11yRule {
    re: Regex,
    cat: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, cat: &'static str, fix: &'static str) -> Option<A11yRule> {
    Regex::new(pat).map_or_else(|_| None, |re| Some(A11yRule { re, cat, fix }))
}

static RULES: LazyLock<Vec<A11yRule>> = LazyLock::new(build_rules);

fn build_rules() -> Vec<A11yRule> {
    vec![mk(
        r"(?i)onclick\s*=",
        "ONCLICK_NO_KEYBOARD",
        "Add onKeyDown/onKeyPress alongside onClick for keyboard accessibility.",
    )]
    .into_iter()
    .flatten()
    .collect()
}

/// Advisory for accessibility issues. Returns None if clean.
pub fn advise(file_path: &str, content: &str) -> Option<String> {
    if !is_frontend(file_path) || content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    let mut findings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        // String-based checks (no regex lookahead needed)
        if line.contains("<img") && !line.contains("alt=") {
            findings.push(format!(
                "  L{}: IMG_NO_ALT — Add alt attribute to <img>.",
                i.saturating_add(1)
            ));
        }
        if line.contains("<input") && !line.contains("aria-label") && !line.contains("id=") {
            findings.push(format!(
                "  L{}: INPUT_NO_LABEL — Add label or aria-label.",
                i.saturating_add(1)
            ));
        }
        // Regex-based checks
        for r in RULES.iter() {
            if r.re.is_match(line) {
                findings.push(format!("  L{}: {} — {}", i.saturating_add(1), r.cat, r.fix));
                break;
            }
        }
    }
    if findings.is_empty() {
        return None;
    }
    let mut msg = format!("[A11Y_ADVISORY] {} issue(s):\n", findings.len());
    for f in findings.iter().take(10) {
        msg.push_str(f);
        msg.push('\n');
    }
    Some(msg)
}

fn is_frontend(p: &str) -> bool {
    use std::path::Path;
    Path::new(p).extension().is_some_and(|ext| {
        matches!(
            ext.to_ascii_lowercase().to_string_lossy().as_ref(),
            "tsx" | "jsx" | "astro" | "html" | "svelte" | "vue"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_img_no_alt() {
        assert!(advise("src/C.tsx", "<img src=\"pic.jpg\" />").is_some());
    }

    #[test]
    fn allows_img_with_alt() {
        assert!(advise("src/C.tsx", "<img src=\"pic.jpg\" alt=\"photo\" />").is_none());
    }

    #[test]
    fn skips_non_frontend() {
        assert!(advise("src/main.rs", "<img src=\"x\" />").is_none());
    }
}
