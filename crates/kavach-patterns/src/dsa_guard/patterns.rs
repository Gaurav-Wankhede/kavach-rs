use regex::Regex;
use std::sync::OnceLock;

pub(super) fn compile_regex(pat: &str) -> Regex {
    loop {
        if let Ok(re) = Regex::new(pat) {
            break re;
        }
    }
}

pub(super) fn get_patterns() -> &'static Vec<Regex> {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            compile_regex(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,800}?\.contains\s*\(\s*&"),
            compile_regex(r"(?s)\.contains_key\s*\([^)]+\)[^;]{0,200};?[^}]{0,400}?\.insert\s*\("),
            compile_regex(r"\.insert\s*\(\s*0\s*,"),
            compile_regex(r"\.remove\s*\(\s*0\s*\)"),
            compile_regex(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,800}?\.sort(?:_by|_unstable|_unstable_by)?\s*\("),
            compile_regex(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,400}?\b\w+\s*\+=\s*&?[^;]*\.to_string\(\)"),
            compile_regex(r"(?s)\b(?:for|while)\b.{0,600}?format!\s*\("),
            compile_regex(r"(?s)Vec::new\s*\(\s*\).{0,800}?\b(?:for|while)\b.{0,400}?\.push\s*\("),
            compile_regex(r"(?s)HashMap::new\(\).{0,800}?(?:for|while).{0,400}?\.insert\("),
            compile_regex(r"\bLinkedList\s*<"),
            compile_regex(r"\bBTreeMap\s*<"),
            compile_regex(r"\bHashMap\s*<\s*(?:u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b"),
            compile_regex(r"(?s)fn\s+(\w+)\s*\([^)]*\)[^{]*\{([^}]{0,2000})"),
            compile_regex(r"\.collect\s*::\s*<\s*Vec<[^>]+>\s*>\s*\(\s*\)\s*\.iter\s*\("),
            compile_regex(r"\.iter\s*\(\s*\)[^;]{0,200}\.map\s*\([^)]*\.clone\s*\(\s*\)"),
            compile_regex(r"(?s)\.sort(?:_by|_unstable)?\s*\([^)]*\)[^;]{0,200};[^}]{0,200}\.(?:iter|into_iter)\s*\(\s*\)\s*\.take\s*\("),
        ]
    })
}

pub(super) fn is_backend_rust_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    if p == "build.rs" || p.ends_with("/crates/build.rs") || p == "./build.rs" {
        return false;
    }
    content.contains("async fn")
        || content.contains("axum::")
        || content.contains("tonic::")
        || content.contains("sqlx::")
        || content.contains("tokio::")
        || content.contains("Service")
        || p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/repository/")
        || p.contains("/repo/")
        || p.contains("/domain/")
        || p.contains("/core/")
}
