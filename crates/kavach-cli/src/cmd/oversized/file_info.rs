// TIME: O(V+E) | SPACE: O(V+E)
// YEAR: 1972 | SEARCHED: 2026-04

const MIXED_CONCERNS_LINE_LIMIT: usize = 200;
/// Hard line-count cap per file.
/// SOURCE: gist.github.com/Illyism/29981ba721a544cbe49044e6f4bb6869 — 200-line rule for AI coding.
/// We use 300 as the BLOCK threshold; 200 stays as the mixed-concerns advisory.
const HARD_LINE_LIMIT: usize = 300;
/// Function-density cap per file.
/// SOURCE: 42-pattern catalog §3 — "Long Method >50 LOC, God Class >15 fn".
const HARD_FN_LIMIT: usize = 10;

/// Metadata about a single Rust source file.
#[derive(Debug)]
pub struct FileInfo {
    pub path: String,
    pub line_count: usize,
    pub fn_count: usize,
    pub test_fn_count: usize,
    pub has_struct: bool,
    pub has_impl: bool,
    pub has_async_fn: bool,
    /// True when the file's first 5 lines contain `// split:` —
    /// the escape hatch that disables oversized-file warnings.
    /// SOURCE: rustfmt convention — file-level directives appear in the header.
    pub has_split_marker: bool,
    pub deps: Vec<String>,
}

fn has_keyword(content: &str, keyword: &str) -> bool {
    content.contains(keyword)
}

/// Count `fn ` declarations excluding tests (#[test] / #[cfg(test)] mod).
/// Tests are exempt because they legitimately accumulate per-detector cases.
fn count_fns(content: &str) -> (usize, usize) {
    let mut total = 0usize;
    let mut test_count = 0usize;
    let mut in_test_mod = false;
    let mut prev_line_was_test_attr = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            in_test_mod = true;
            prev_line_was_test_attr = false;
            continue;
        }
        let is_test_attr = trimmed.starts_with("#[test]");
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("async fn ") || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("pub(crate) fn ") || trimmed.starts_with("pub(crate) async fn ")
        {
            total += 1;
            if in_test_mod || prev_line_was_test_attr {
                test_count += 1;
            }
        }
        prev_line_was_test_attr = is_test_attr;
    }
    (total, test_count)
}

impl FileInfo {
    pub fn from_content(path: String, content: &str) -> Self {
        let line_count = content.lines().count();
        let has_struct = has_keyword(content, "\nstruct ") || has_keyword(content, "\npub struct ");
        let has_impl = has_keyword(content, "\nimpl ");
        let has_async_fn = has_keyword(content, "async fn ");
        let (fn_count, test_fn_count) = count_fns(content);
        let has_split_marker = content
            .lines()
            .take(5)
            .any(|l| l.contains("// split:"));
        let deps = extract_deps(content);
        Self {
            path, line_count, fn_count, test_fn_count,
            has_struct, has_impl, has_async_fn, has_split_marker, deps,
        }
    }

    /// Production function count (excludes test fns).
    pub fn prod_fn_count(&self) -> usize {
        self.fn_count.saturating_sub(self.test_fn_count)
    }

    pub fn is_mixed_concerns_oversized(&self) -> bool {
        if self.has_split_marker {
            return false;
        }
        self.line_count > MIXED_CONCERNS_LINE_LIMIT
            && self.has_struct
            && self.has_impl
            && self.has_async_fn
    }

    /// Hard threshold: file too large or too many production functions.
    /// Test functions are excluded — they legitimately scale with detector count.
    /// SOURCE: eslint max-lines + max-lines-per-function (200/50 thresholds).
    pub fn exceeds_hard_threshold(&self) -> bool {
        if self.has_split_marker {
            return false;
        }
        self.line_count > HARD_LINE_LIMIT || self.prod_fn_count() > HARD_FN_LIMIT
    }
}

/// Returns true when `name` has an opening brace — i.e. it is an inline mod body, not a file mod.
fn mod_is_inline(name: &str) -> bool {
    name.find('{').is_some()
}

pub fn extract_deps(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("use ") {
            if let Some(module) = rest.split("::").next() {
                let name = module.trim_end_matches(';').trim().to_string();
                if !name.is_empty() {
                    deps.push(name);
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("mod ") {
            let name = rest.trim_end_matches(';').trim();
            if !mod_is_inline(name) && !name.is_empty() {
                deps.push(name.to_string());
            }
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_detect_mixed_concerns_when_over_200_lines() {
        let big: String = (0..201).map(|_| "x\n").collect();
        let content = format!("{big}struct Foo {{}}\nimpl Foo {{}}\nasync fn bar() {{}}");
        let fi = FileInfo::from_content("src/foo.rs".into(), &content);
        assert!(fi.is_mixed_concerns_oversized());
    }

    #[test]
    fn should_not_flag_split_escape_hatch() {
        let big: String = (0..201).map(|_| "x\n").collect();
        // Marker must appear in first 5 lines of content, not in the path.
        let content = format!(
            "// split: intentional — see roadmap id=3539\n{big}struct Foo {{}}\nimpl Foo {{}}\nasync fn bar() {{}}"
        );
        let fi = FileInfo::from_content("src/foo.rs".into(), &content);
        assert!(!fi.is_mixed_concerns_oversized());
        assert!(!fi.exceeds_hard_threshold());
    }

    #[test]
    fn should_extract_use_deps() {
        let content = "use std::collections::HashMap;\nuse tokio::sync::Mutex;";
        let deps = extract_deps(content);
        assert!(deps.iter().any(|d| d == "std"));
        assert!(deps.iter().any(|d| d == "tokio"));
    }

    #[test]
    fn should_skip_inline_mod() {
        let content = "mod foo;\nmod bar { fn x() {} }";
        let deps = extract_deps(content);
        assert!(deps.iter().any(|d| d == "foo"));
        assert!(!deps.iter().any(|d| d == "bar"));
    }
}
