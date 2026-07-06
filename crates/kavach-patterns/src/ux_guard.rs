// split: intentional — guard module, not handler
//! UX guard — design system consistency: tokens, spacing, components, motion, responsive.

use regex::Regex;
use std::sync::LazyLock;

struct UxRule {
    re: Regex,
    sev: &'static str,
    cat: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, sev: &'static str, cat: &'static str, fix: &'static str) -> Option<UxRule> {
    Regex::new(pat).ok().map(|re| UxRule { re, sev, cat, fix })
}

fn build_rules() -> Vec<UxRule> {
    let arb_spacing = r"(?i)(p|m|gap|space)-\[\d+px\]";
    let arb_rem = r"(?i)(p|m|gap|space)-\[\d+(\.\d+)?rem\]";
    let raw_anim = r"@keyframes\s+\w+";
    let css_anim = r"animation\s*:";
    let layout_tag = r"<(html|body|head)\b";
    vec![
        mk(
            arb_spacing,
            "P0",
            "ARBITRARY_SPACING",
            "Use Tailwind scale (p-4, m-6) not arbitrary px values.",
        ),
        mk(
            arb_rem,
            "P0",
            "ARBITRARY_SPACING_REM",
            "Use Tailwind scale not arbitrary rem values.",
        ),
        mk(
            raw_anim,
            "P1",
            "RAW_KEYFRAMES",
            "Use motion/react for animations. Not raw @keyframes in TSX.",
        ),
        mk(
            css_anim,
            "P1",
            "RAW_CSS_ANIMATION",
            "Use motion/react animate prop. Not CSS animation property.",
        ),
        mk(
            layout_tag,
            "P0",
            "LAYOUT_IN_COMPONENT",
            "html/body/head belong in layouts/ only. Not in components.",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

static RULES: LazyLock<Vec<UxRule>> = LazyLock::new(build_rules);

fn count_islands(content: &str) -> usize {
    content
        .lines()
        .filter(|l| {
            l.contains("client:load") || l.contains("client:visible") || l.contains("client:only")
        })
        .count()
}

pub fn check(file_path: &str, content: &str) -> Option<String> {
    if !is_fe(file_path) || content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    let in_layout = file_path.contains("/layouts/") || file_path.contains("/layout");
    let mut blocks = Vec::new();
    let mut advs = Vec::new();

    for (i, line) in content.lines().enumerate() {
        for r in RULES.iter() {
            if r.cat == "LAYOUT_IN_COMPONENT" && in_layout {
                continue;
            }
            if r.re.is_match(line) {
                let entry = format!("  L{}: {} — {}", i.saturating_add(1), r.cat, r.fix);
                match r.sev {
                    "P0" => blocks.push(entry),
                    _ => advs.push(entry),
                }
            }
        }
    }

    let islands = count_islands(content);
    if islands > 1 {
        blocks.push(format!(
            "  MULTI_ISLAND: {islands} islands. Wrap compound components in ONE island."
        ));
    }

    if blocks.is_empty() {
        return None;
    }
    let mut msg = String::from("BOUNTY_UX_BLOCK:\n");
    for f in &blocks {
        msg.push_str(f);
        msg.push('\n');
    }
    if !advs.is_empty() {
        msg.push_str("[UX_ADVISORY]\n");
        for f in &advs {
            msg.push_str(f);
            msg.push('\n');
        }
    }
    Some(msg)
}

fn is_fe(p: &str) -> bool {
    let ext = std::path::Path::new(p)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);
    matches!(ext.as_deref(), Some(e) if matches!(e, "tsx" | "jsx" | "astro" | "svelte" | "vue"))
}

#[cfg(test)]
#[path = "ux_guard_test.rs"]
mod tests;
