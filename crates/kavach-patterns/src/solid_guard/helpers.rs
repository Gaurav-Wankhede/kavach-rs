//! Helper functions for SOLID gate detection: text analysis and validation.

/// Count the number of struct fields in a captured block (field list).
pub(super) fn count_struct_fields(captured: &str) -> usize {
    captured
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with("///")
                && !t.starts_with("#[")
                && !t.starts_with("/*")
                && !t.starts_with('*')
                && t.contains(':')
        })
        .count()
}

/// Count the number of trait methods in a captured block (method list).
pub(super) fn count_trait_methods(captured: &str) -> usize {
    captured
        .lines()
        .filter(|l| l.trim_start().starts_with("fn ") || l.trim_start().starts_with("async fn "))
        .count()
}

/// Count conflated derives in a derive attribute list.
pub(super) fn count_conflated_derives(captured: &str, derives_to_check: &[&str]) -> usize {
    derives_to_check
        .iter()
        .filter(|d| captured.contains(*d))
        .count()
}

/// Count the number of lines in an async fn body, starting from the opening brace position.
pub(super) fn count_lines_in_async_fn(content: &str, fn_start: usize) -> usize {
    let bytes = content.as_bytes();
    let mut depth: i32 = 1;
    let mut lines = 0usize;
    for &b in bytes.iter().skip(fn_start) {
        if b == b'{' {
            depth = depth.saturating_add(1);
        } else if b == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return lines;
            }
        } else if b == b'\n' {
            lines = lines.saturating_add(1);
        }
    }
    lines
}

/// Check if a file is a Rust backend file eligible for SOLID checks.
pub(super) fn is_rust_backend_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    if p.ends_with("/build.rs") {
        return false;
    }
    content.contains("async fn")
        || content.contains("axum::")
        || content.contains("tonic::")
        || content.contains("sqlx::")
        || content.contains("reqwest::")
        || content.contains("Service")
        || p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/repository/")
        || p.contains("/repo/")
        || p.contains("/domain/")
}
