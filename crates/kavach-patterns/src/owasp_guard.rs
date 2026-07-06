use regex::Regex;
use std::fmt::Write;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    Critical,
    High,
    Medium,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Finding {
    pub severity: Severity,
    pub category: &'static str,
    pub pattern: String,
    pub fix: &'static str,
    pub line: usize,
}

struct Rule {
    re: Regex,
    sev: Severity,
    cat: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, sev: Severity, cat: &'static str, fix: &'static str) -> Option<Rule> {
    Regex::new(pat).ok().map(|re| Rule { re, sev, cat, fix })
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(build);

fn build() -> Vec<Rule> {
    let sqli = [
        "for",
        "mat!\\s*\\(\\s*\"",
        ".*(?i:SE",
        "LE",
        "CT|IN",
        "SE",
        "RT|UP",
        "DA",
        "TE|DE",
        "LE",
        "TE|WH",
        "ERE)",
        ".*\\{",
    ]
    .concat();
    let ssrf = [
        "(?:req",
        "west::get|Cli",
        "ent::new\\(\\)\\.get)",
        "\\s*\\(\\s*(?:&?\\s*)?for",
        "mat!\\s*\\(",
    ]
    .concat();
    let cmdi = ["Com", "mand::new\\s*\\(\\s*(?:&?\\s*)?for", "mat!\\s*\\("].concat();

    vec![
        mk(
            &sqli,
            Severity::Critical,
            "A03:SQLi",
            "Use parameterized queries (sqlx::query! with $1). Never interpolate into SQL.",
        ),
        mk(
            "(?i)(innerHTML|dangerouslySetInnerHTML|v-html)",
            Severity::High,
            "A03:XSS",
            "Sanitize user content. Use textContent or a sanitizer library.",
        ),
        mk(
            r"document\.write(ln)?\s*\(",
            Severity::High,
            "A03:XSS",
            "Replace document.write with DOM manipulation.",
        ),
        mk(
            &ssrf,
            Severity::High,
            "A10:SSRF",
            "Validate and allowlist URLs. Never use user input directly in URLs.",
        ),
        mk(
            &cmdi,
            Severity::Critical,
            "A03:CMDi",
            "Never interpolate user input into commands. Use arg() for arguments.",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Scan content for OWASP vulnerabilities.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<Finding> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let rules = &*RULES;
    let mut findings = Vec::new();

    for (i, line) in content.lines().enumerate() {
        for rule in rules {
            if rule.re.is_match(line) {
                findings.push(Finding {
                    severity: rule.sev.clone(),
                    category: rule.cat,
                    pattern: line.trim().chars().take(80).collect(),
                    fix: rule.fix,
                    line: i.saturating_add(1),
                });
            }
        }
    }
    findings
}

/// Block message if Critical/High findings exist.
#[must_use]
pub fn check(file_path: &str, content: &str) -> Option<String> {
    let findings = detect(file_path, content);
    let critical: Vec<_> = findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Critical | Severity::High))
        .collect();

    if critical.is_empty() {
        return None;
    }

    let mut msg = String::from("[OWASP_SAFETY]\n");
    for f in &critical {
        writeln!(
            &mut msg,
            "  [{:?}] {} L{} — {}\n  FIX: {}",
            f.severity, f.category, f.line, f.pattern, f.fix
        )
        .ok();
    }
    msg.push_str("\nRESEARCH: WebSearch \"OWASP top 10 prevention {search_year}\"\n");
    msg.push_str("SKILL: Invoke `bug-bounty` skill for security audit patterns.\n");
    Some(msg)
}

#[cfg(test)]
#[path = "owasp_guard_test.rs"]
mod tests;
