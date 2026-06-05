use crate::regex_patterns::fbase;
use std::path::Path;

fn check_ext_ignore_case(p: &str, exts: &[&str]) -> bool {
    let path = Path::new(p);
    path.extension()
        .is_some_and(|ext| exts.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

#[must_use]
pub fn is_frontend_file(p: &str) -> bool {
    let is_ts_js = check_ext_ignore_case(p, &["tsx", "ts", "jsx", "js", "astro"]);
    // Cloudflare Workers/Durable Objects are server-side code, not frontend
    if is_ts_js && is_edge_worker_file(p) {
        return false;
    }
    is_ts_js
}
/// True for Cloudflare Workers, Durable Objects, and edge runtime files.
/// These are server-side TypeScript — NOT frontend code.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn is_edge_worker_file(p: &str) -> bool {
    let l = p.to_lowercase();
    l.contains("/workers/")
        || l.contains("/edge-cache/")
        || l.contains("/durable-objects/")
        || l.contains("-do.ts")
        || l.contains(".do.ts")
        || l.ends_with("-worker.ts")
        || l.ends_with(".worker.ts")
        || l.contains("/cf-workers/")
        || l.contains("/cloudflare-workers/")
}
#[must_use]
pub fn is_backend_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["rs", "go", "py", "java", "kt"])
}
#[must_use]
pub fn is_rust_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["rs"])
}
#[must_use]
pub fn is_go_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["go"])
}
#[must_use]
pub fn is_python_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["py"])
}
#[must_use]
pub fn is_java_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["java", "kt"])
}
#[must_use]
pub fn is_astro_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["astro"])
}
#[must_use]
pub fn is_dockerfile(p: &str) -> bool {
    let b = fbase(p);
    b == "dockerfile" || b.ends_with(".dockerfile")
}
#[must_use]
pub fn is_shell_file(p: &str) -> bool {
    check_ext_ignore_case(p, &["sh", "bash", "zsh"])
}
#[must_use]
pub fn is_test_file(p: &str) -> bool {
    let l = p.to_lowercase();
    // Compound suffixes (`.test.ts`) are NOT a single `Path::extension()` — suffix-match them directly.
    l.contains("_test.go")
        || l.contains("test_")
        || l.ends_with(".test.ts")
        || l.ends_with(".test.tsx")
        || l.ends_with(".spec.ts")
        || l.ends_with(".spec.tsx")
        || l.contains("/tests/")
        || l.contains("/test/")
        || fbase(p) == "tests.rs"
}
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn is_allowlisted(p: &str) -> bool {
    let l = p.to_lowercase();
    // Rules/docs files: .md files in ~/.claude/ are config prose, not production code.
    // Scanning them for TODO/FIXME produces false positives on anti-pattern documentation.
    if Path::new(&l)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        && (l.contains("/.claude/")
            || l.contains("/rules/")
            || l.contains("/skills/")
            || l.contains("/docs/"))
    {
        return true;
    }
    is_test_file(p)
        || l.contains("/migrations/")
        || l.contains("/seeds/")
        || l.ends_with(".stories.tsx")
        || l.ends_with(".stories.ts")
        || l.contains("/examples/")
        || l.contains("/docs/")
        || l.contains("kavach-patterns/src/")
        || l.contains("kavach-engine/src/gates/")
}
#[must_use]
pub fn is_handler_file(p: &str) -> bool {
    let l = p.to_lowercase();
    l.contains("handler") || l.contains("routes") || l.contains("lib.rs") || l.contains("main.rs")
}
/// True for frontend TS/JS files that act as API clients (not UI components).
///
/// Scopes `API_DRIFT` and contract checks to the API layer only — avoids firing
/// on UI components, hooks, utilities, and config that happen to be .ts.
#[must_use]
pub fn is_api_client_file(p: &str) -> bool {
    if !is_frontend_file(p) {
        return false;
    }
    let l = p.to_lowercase();
    let b = fbase(p);
    l.contains("/api/")
        || l.contains("/services/")
        || l.contains("/client/")
        || l.contains("/requests/")
        || l.contains("/endpoints/")
        || b.starts_with("api")
        || b.ends_with("api.ts")
        || b.ends_with("api.tsx")
        || b.ends_with("client.ts")
        || b.ends_with("service.ts")
}
#[must_use]
pub fn is_non_config_file(p: &str) -> bool {
    let l = p.to_lowercase();
    ![
        "config",
        ".env",
        "astro.config",
        "vite.config",
        "next.config",
        "wrangler.toml",
        "docker-compose",
        "caddyfile",
        ".toml",
        "dev_ports",
        "constants",
    ]
    .iter()
    .any(|x| l.contains(x))
}
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn has_env_fallback(l: &str) -> bool {
    l.contains("??") || l.contains("||") || l.contains("import.meta.env")
}

/// Returns true when every occurrence of a task-marker word on `line` is inside
/// a quoted string literal (`"..."` or `'...'`). Used to suppress false positives
/// where status values like `"todo"` or `"done"` appear as match-arm patterns.
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn is_marker_inside_string(line: &str) -> bool {
    // Walk the line character-by-character tracking quote depth.
    // If we encounter a non-whitespace word outside any quoted region, return false.
    let mut in_quote: Option<char> = None;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes.get(i).map(|&b| b as char);
        match (in_quote, ch) {
            (Some(q), Some(c))
                if c == q && (i == 0 || bytes.get(i.wrapping_sub(1)) != Some(&b'\\')) =>
            {
                in_quote = None;
            }
            (None, Some('"')) => {
                in_quote = Some('"');
            }
            (None, Some('\'')) => {
                in_quote = Some('\'');
            }
            (None, Some(_)) => {
                // Check if a task-marker word starts here outside a string.
                if let Some(rest) = line.get(i..) {
                    let upper_bytes: Vec<u8> = rest
                        .as_bytes()
                        .iter()
                        .map(|&b| b.to_ascii_uppercase())
                        .collect();
                    for marker in &["TODO", "FIXME", "HACK", "XXX"] {
                        if upper_bytes.starts_with(marker.as_bytes()) {
                            let after = i.wrapping_add(marker.len());
                            let end_is_word_boundary = after >= line.len()
                                || !bytes.get(after).is_some_and(u8::is_ascii_alphanumeric);
                            let start_is_word_boundary = i == 0
                                || !bytes
                                    .get(i.wrapping_sub(1))
                                    .is_some_and(u8::is_ascii_alphanumeric);
                            if start_is_word_boundary && end_is_word_boundary {
                                return false; // bare marker found outside string
                            }
                        }
                    }
                }
            }
            _ => {} // (Some(_), _) and other cases do nothing
        }
        i = i.wrapping_add(1);
    }
    true // all marker occurrences were inside string literals (or none found)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_file_types() {
        assert!(is_frontend_file("a.tsx"));
        assert!(is_backend_file("a.rs"));
        assert!(is_test_file("a_test.go"));
    }
    #[test]
    fn test_allowlist_claude_rules_md() {
        // .md files in ~/.claude/ must be exempt — they document anti-patterns by name
        // and contain words like TODO/FIXME/HACK as prose, not as code markers.
        assert!(is_allowlisted(
            "/Users/gauravwankhede/.claude/rules/04-anti-patterns.md"
        ));
        assert!(is_allowlisted("/Users/gauravwankhede/.claude/CLAUDE.md"));
        assert!(is_allowlisted(
            "/Users/gauravwankhede/.claude/skills/rust/SKILL.md"
        ));
    }
    #[test]
    fn test_allowlist_does_not_exempt_rs_files() {
        assert!(!is_allowlisted("src/gates/intent.rs"));
    }
    #[test]
    fn test_api_client_by_path() {
        assert!(is_api_client_file("src/api/users.ts"));
        assert!(is_api_client_file("src/services/auth.ts"));
        assert!(is_api_client_file("src/client/http.ts"));
        assert!(is_api_client_file("src/requests/payments.tsx"));
        assert!(is_api_client_file("src/endpoints/orders.ts"));
    }
    #[test]
    fn test_api_client_by_name() {
        assert!(is_api_client_file("src/apiClient.ts"));
        assert!(is_api_client_file("src/authApi.ts"));
        assert!(is_api_client_file("src/soundbak.client.ts"));
        assert!(is_api_client_file("src/user.service.ts"));
    }
    #[test]
    fn test_is_marker_inside_string_quoted_todo() {
        // "todo" as a string literal must NOT be flagged as a bare marker
        assert!(is_marker_inside_string(
            r#"            "todo"        => "[TODO]","#
        ));
    }
    #[test]
    fn test_is_marker_inside_string_bare_todo_in_comment() {
        // TODO in a comment is a bare marker — must return false
        assert!(!is_marker_inside_string("    // TODO: implement this"));
    }
    #[test]
    fn test_is_marker_inside_string_bare_fixme() {
        assert!(!is_marker_inside_string("    // FIXME: broken"));
    }
    #[test]
    fn test_is_marker_inside_string_no_marker() {
        // Line with no marker at all — returns true (nothing to flag)
        assert!(is_marker_inside_string("    let x = 42;"));
    }
    #[test]
    fn test_api_client_excludes_ui() {
        assert!(!is_api_client_file("src/components/Button.tsx"));
        assert!(!is_api_client_file("src/hooks/useAuth.ts"));
        assert!(!is_api_client_file("src/utils/format.ts"));
        assert!(!is_api_client_file("src/pages/Home.tsx"));
        assert!(!is_api_client_file("server/handler.rs"));
    }
    #[test]
    fn test_edge_worker_file_detection() {
        // Cloudflare Workers paths
        assert!(is_edge_worker_file(
            "packages/workers/edge-cache/src/index.ts"
        ));
        assert!(is_edge_worker_file("packages/workers/api/src/handler.ts"));
        assert!(is_edge_worker_file("src/durable-objects/counter.ts"));
        assert!(is_edge_worker_file("src/cf-workers/rate-limiter.ts"));
        assert!(is_edge_worker_file("src/cloudflare-workers/auth.ts"));
        // Durable Object naming conventions
        assert!(is_edge_worker_file("src/ratelimit-do.ts"));
        assert!(is_edge_worker_file("src/counter.do.ts"));
        assert!(is_edge_worker_file("src/cache-worker.ts"));
        assert!(is_edge_worker_file("src/auth.worker.ts"));
        // NOT edge workers — regular frontend
        assert!(!is_edge_worker_file("src/components/Button.tsx"));
        assert!(!is_edge_worker_file("src/pages/Home.tsx"));
        assert!(!is_edge_worker_file("src/api/client.ts"));
    }
    #[test]
    fn test_frontend_excludes_edge_workers() {
        // Edge worker .ts files must NOT be classified as frontend
        assert!(!is_frontend_file(
            "packages/workers/edge-cache/src/ratelimit-do.ts"
        ));
        assert!(!is_frontend_file("src/durable-objects/counter.ts"));
        assert!(!is_frontend_file("src/auth-worker.ts"));
        // Regular frontend .ts files still work
        assert!(is_frontend_file("src/components/Button.tsx"));
        assert!(is_frontend_file("src/pages/Home.ts"));
    }
}
