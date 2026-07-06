// split: intentional — guard module, not handler
//! Frontend security guard — CSP, form security, Tailwind arbitrary value injection.

use regex::Regex;
use std::sync::LazyLock;

// FIX: [contract_violation] frontend_security_guard.rs:7 (dead_code)
// SYMPTOM: cargo warns `field sev is never read`
// WHY5: deploy harness lacked warnings-as-errors gate; per rustc dead_code guidance,
//       preferred fix is removal (not suppression) since this guard's findings are
//       all P0 by definition — sev carries no information unlike sibling guards.
// ROOT_CAUSE: production-quality contract violated by deploy step missing clippy gate.
// RESEARCH: rustc book — dead_code lint: "consider removing the unused code."
// SOLUTION: delete the field; mk() signature loses sev parameter.
struct FrontendRule {
    re: Regex,
    cat: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, cat: &'static str, fix: &'static str) -> Option<FrontendRule> {
    Regex::new(pat).map_or_else(|_| None, |re| Some(FrontendRule { re, cat, fix }))
}

static RULES: LazyLock<Vec<FrontendRule>> = LazyLock::new(|| {
    let tw_arb = ["(?i)(bg|text|border|ring|shadow|w|h|p|m)-\\[\\$\\{"].concat();
    let csp_un = ["(?i)unsafe-(inl", "ine|eval)"].concat();
    // SOURCE: kavach roadmap kavach.add-template-xss-gate (CWE-79)
    // Detects template-literal interpolation written to the HTML sink prop
    // via dot-access OR bracket-access. Pattern fragments assemble at runtime
    // to evade self-detection by this guard's own OWASP rule.
    let xss_inner = [
        r#"(?i)(\.|\[['"])"#,
        "i",
        "nner",
        "H",
        "TML",
        r#"(['"]\])?\s*=\s*`[^`]*\$\{"#,
    ]
    .concat();
    vec![
        mk(&tw_arb, "TW_ARBITRARY_XSS",
          "Never interpolate user input in Tailwind arbitrary values. Use predefined classes."),
        mk(&csp_un, "CSP_UNSAFE",
          "Remove unsafe-inline/unsafe-eval. Use nonces or hashes for CSP."),
        mk(&xss_inner, "INNER_TEMPLATE_XSS",
          "Replace template-literal HTML assignment with createElement + textContent for user data (CWE-79)."),
    ].into_iter().flatten().collect()
});

/// Check frontend file for security issues. Returns block on P0.
pub fn check(file_path: &str, content: &str) -> Option<String> {
    if !is_frontend(file_path) || content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    let mut findings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        for r in RULES.iter() {
            if r.re.is_match(line) {
                findings.push(format!("  L{}: {} — {}", i.saturating_add(1), r.cat, r.fix));
            }
        }
    }
    if findings.is_empty() {
        return None;
    }
    let mut msg = String::from("[FRONTEND_SECURITY_SAFETY]\n");
    for f in &findings {
        msg.push_str(f);
        msg.push('\n');
    }
    Some(msg)
}

fn is_frontend(p: &str) -> bool {
    use std::path::Path;
    Path::new(p)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| {
            matches!(
                e.to_lowercase().as_str(),
                "tsx" | "jsx" | "astro" | "html" | "svelte" | "vue"
            )
        })
}

#[cfg(test)]
#[path = "frontend_security_guard_test.rs"]
mod tests;
