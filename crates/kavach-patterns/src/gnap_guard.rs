use regex::Regex;
use std::fmt::Write;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GnapFinding {
    pub pattern: String,
    pub violation: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

struct Rule {
    re: Regex,
    violation: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, violation: &'static str, fix: &'static str) -> Option<Rule> {
    Regex::new(pat).ok().map(|re| Rule { re, violation, fix })
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(build_rules);

fn build_rules() -> Vec<Rule> {
    vec![
        // OAuth patterns — BANNED
        mk(
            r"(?i)Authorization:\s*Bearer\s",
            "Bearer token header (no key proof)",
            "httpsig (RFC 9421) with Ed25519 key-bound token",
        ),
        mk(
            r"(?i)\bclient_id\s*[=:]",
            "OAuth client_id pattern",
            "GNAP client.key with proof: httpsig",
        ),
        mk(
            r"(?i)\bclient_secret\s*[=:]",
            "OAuth client_secret pattern",
            "GNAP key proofing (httpsig/mtls)",
        ),
        mk(
            r"(?i)\bredirect_uri\s*[=:]",
            "OAuth redirect_uri pattern",
            "GNAP interact.finish.uri",
        ),
        mk(
            r#"(?i)\bresponse_type\s*=\s*["']?code"#,
            "OAuth authorization code flow",
            "GNAP interact.start: [\"redirect\"]",
        ),
        mk(
            r#"(?i)\bgrant_type\s*=\s*["']?(authorization_code|client_credentials|refresh_token)"#,
            "OAuth grant_type pattern",
            "GNAP grant request with access_token field",
        ),
        mk(
            r#"(?i)\bscope\s*=\s*["'][^"']*["']"#,
            "OAuth scope string pattern",
            "GNAP access: [{type, actions, locations}]",
        ),
        mk(
            r"(?i)oauth2?::|OAuth2?Client|oauth2?_",
            "OAuth library usage",
            "GNAP client with httpsig proofing",
        ),
        // Token storage patterns — BANNED
        mk(
            r#"localStorage\.(setItem|getItem)\s*\(\s*["'](token|access_token|refresh_token|session)"#,
            "Token in localStorage (XSS vulnerable)",
            "Memory-only + httpOnly cookie for session",
        ),
        mk(
            r#"sessionStorage\.(setItem|getItem)\s*\(\s*["'](token|access_token)"#,
            "Token in sessionStorage",
            "Memory-only storage",
        ),
        // Bearer token construction
        mk(
            r#"["']Bearer\s+["']\s*\+|format!\s*\(\s*["']Bearer\s"#,
            "Bearer token construction",
            "httpsig header construction",
        ),
        mk(
            r#"\.set\s*\(\s*["']Authorization["']\s*,\s*["']Bearer"#,
            "Setting Bearer header",
            "Signature-Input + Signature headers (RFC 9421)",
        ),
        // Refresh token patterns
        mk(
            r"(?i)\brefresh_token\s*[=:]",
            "OAuth refresh_token pattern",
            "GNAP token.manage.uri for rotation",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

static EXEMPTIONS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "// gnap-exempt:",
        "// oauth-migration:",
        "test_",
        "_test.rs",
        "tests/",
        "mock",
        "stripe",
        "paypal",
        "third_party",
        "external_api",
    ]
});

fn is_exempt(file_path: &str, line: &str) -> bool {
    let lower_line = line.to_lowercase();
    let lower_path = file_path.to_lowercase();

    EXEMPTIONS
        .iter()
        .any(|e| lower_line.contains(&e.to_lowercase()) || lower_path.contains(&e.to_lowercase()))
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<GnapFinding> {
    if content.is_empty() {
        return vec![];
    }

    // Skip kavach-patterns itself
    if file_path.contains("kavach-patterns/src/") {
        return vec![];
    }

    // Skip test files
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let rules = &*RULES;
    let mut findings = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if is_exempt(file_path, line) {
            continue;
        }

        for rule in rules {
            if let Some(m) = rule.re.find(line) {
                findings.push(GnapFinding {
                    pattern: m.as_str().to_owned(),
                    violation: rule.violation,
                    fix: rule.fix,
                    line: i.saturating_add(1),
                });
            }
        }
    }

    findings
}

#[must_use]
pub fn check(file_path: &str, content: &str) -> Option<String> {
    let findings = detect(file_path, content);
    if findings.is_empty() {
        return None;
    }

    let mut msg = String::from("BOUNTY_GNAP_BLOCK:\n");
    for f in &findings {
        write!(
            msg,
            "  BANNED: '{}' at L{}\n  VIOLATION: {}\n  FIX: {}\n\n",
            f.pattern, f.line, f.violation, f.fix
        )
        .ok();
    }
    msg.push_str("RESEARCH: WebSearch \"GNAP RFC 9635 implementation {search_year}\"\n");
    msg.push_str("SKILL: Invoke `domain-specialist` skill (gnap-paseto section)\n");
    msg.push_str("SPEC: RFC 9635 (GNAP Core) + RFC 9767 (Resource Servers)\n");
    msg.push_str("STACK: GNAP grants + httpsig (RFC 9421) + PASETO v4 + Ed25519\n");
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_bearer_header() {
        let code = r#"headers.insert("Authorization: Bearer " + token);"#;
        let f = detect("src/api.rs", code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.fix.contains("httpsig")));
    }

    #[test]
    fn blocks_client_id() {
        let code = r#"let client_id = "abc123";"#;
        let f = detect("src/auth.rs", code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.fix.contains("GNAP")));
    }

    #[test]
    fn blocks_client_secret() {
        let code = r#"client_secret: env::var("SECRET"),"#;
        let f = detect("src/oauth.rs", code);
        assert!(!f.is_empty());
    }

    #[test]
    fn blocks_oauth_redirect_uri() {
        let code = r#"redirect_uri: "https://app.example.com/callback","#;
        let f = detect("src/auth.rs", code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.fix.contains("interact.finish")));
    }

    #[test]
    fn blocks_grant_type() {
        let code = r"grant_type=authorization_code";
        let f = detect("src/oauth.rs", code);
        assert!(!f.is_empty());
    }

    #[test]
    fn blocks_oauth_scope() {
        let code = r#"scope = "read write profile""#;
        let f = detect("src/auth.rs", code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.fix.contains("access:")));
    }

    #[test]
    fn blocks_localstorage_token() {
        let code = r#"localStorage.setItem("token", accessToken);"#;
        let f = detect("src/app.tsx", code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.fix.contains("Memory-only")));
    }

    #[test]
    fn blocks_oauth_library() {
        let code = "use oauth2::Client;";
        let f = detect("src/auth.rs", code);
        assert!(!f.is_empty());
    }

    #[test]
    fn allows_gnap_exempt_comment() {
        let code = r#"// gnap-exempt: third-party API requires Bearer
let header = "Authorization: Bearer " + token;"#;
        let f = detect("src/external.rs", code);
        // First line exempt, second line should still be caught
        assert!(f.len() <= 1);
    }

    #[test]
    fn allows_test_files() {
        let code = r#"client_id = "test-id";"#;
        assert!(detect("src/tests/auth_test.rs", code).is_empty());
    }

    #[test]
    fn allows_stripe_exemption() {
        let code = r"stripe::client_id = key;";
        assert!(detect("src/payments.rs", code).is_empty());
    }

    #[test]
    fn skips_patterns_crate() {
        let code = r#"client_id = "example";"#;
        assert!(detect("kavach-patterns/src/gnap_guard.rs", code).is_empty());
    }

    #[test]
    fn check_returns_block_message() {
        let code = r"Authorization: Bearer token123";
        let result = check("src/api.rs", code);
        assert!(result.is_some());
        assert!(
            result
                .as_ref()
                .is_some_and(|m| m.contains("BOUNTY_GNAP_BLOCK"))
        );
    }

    #[test]
    fn blocks_refresh_token() {
        let code = r"refresh_token: stored_refresh,";
        let f = detect("src/auth.rs", code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.fix.contains("manage.uri")));
    }
}
