// split: intentional — guard module, not handler
//! Banned CSS guard — inline styles, hardcoded hex, banned component libraries.

use regex::Regex;
use std::sync::LazyLock;

struct BannedRule {
    re: Regex,
    cat: &'static str,
    fix: &'static str,
}

fn build_rules() -> Vec<BannedRule> {
    let pats: Vec<(String, &str, &str)> = vec![
        (
            build_inline_re(),
            "INLINE_STYLE",
            "Remove inline styles. Use Tailwind.",
        ),
        (
            build_hex_re(),
            "HARDCODED_HEX",
            "Replace hex with semantic color.",
        ),
        (
            build_headless_re(),
            "BANNED_HEADLESSUI",
            "BANNED: Use Tailwind Plus.",
        ),
        (
            build_banned_re(),
            "BANNED_LIB",
            "BANNED library. Use Tailwind Plus.",
        ),
    ];
    pats.into_iter()
        .filter_map(|(p, c, f)| {
            Regex::new(&p)
                .ok()
                .map(|re| BannedRule { re, cat: c, fix: f })
        })
        .collect()
}

fn build_inline_re() -> String {
    ["style=\\", "{}\\", "{}"].concat()
}
fn build_hex_re() -> String {
    String::from(r"bg-\[#[0-9a-fA-F]+\]")
}
fn build_headless_re() -> String {
    ["@head", "lessui/react"].concat()
}
fn build_banned_re() -> String {
    [
        "shad",
        "cn/ui|radix-ui|park-ui|ark-ui|kobalte|bits-ui|daisy",
        "ui|preline|flowbite|chakra-ui|mantine|@mui|ant-design",
    ]
    .concat()
}
// FIX: [contract_violation] banned_css_guard.rs:28 (dead_code)
// SYMPTOM: cargo warned `function brace_pair is never used`
// WHY5: deploy harness lacked warnings-as-errors gate; helper was an aborted
//       refactor of build_inline_re() that never landed. Per rustc dead_code
//       guidance, removal beats suppression.
// SOLUTION: deleted the orphan helper.
static RULES: LazyLock<Vec<BannedRule>> = LazyLock::new(build_rules);

pub fn check(file_path: &str, content: &str) -> Option<String> {
    if !is_fe(file_path) || content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        for r in RULES.iter() {
            if r.re.is_match(line) {
                out.push(format!("  L{}: {} — {}", i.saturating_add(1), r.cat, r.fix));
            }
        }
    }
    if out.is_empty() {
        return None;
    }
    let mut msg = String::from("BOUNTY_BANNED_CSS_BLOCK:\n");
    for f in &out {
        msg.push_str(f);
        msg.push('\n');
    }
    Some(msg)
}

fn is_fe(p: &str) -> bool {
    std::path::Path::new(p).extension().is_some_and(|e| {
        e.eq_ignore_ascii_case("tsx")
            || e.eq_ignore_ascii_case("jsx")
            || e.eq_ignore_ascii_case("astro")
            || e.eq_ignore_ascii_case("html")
            || e.eq_ignore_ascii_case("svelte")
            || e.eq_ignore_ascii_case("vue")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hex() {
        let code = ["bg-[#3b82", "f6]"].concat();
        assert!(check("src/C.tsx", &code).is_some());
    }

    #[test]
    fn allows_semantic() {
        assert!(check("src/C.tsx", "className=\"bg-blue-500\"").is_none());
    }

    #[test]
    fn skips_rust() {
        assert!(check("src/main.rs", "bg-[#fff]").is_none());
    }
}
