//! AST pattern scanning wrapper over `ast-grep-core`.

pub use ast_grep_language::SupportLang;
use ast_grep_core::AstGrep;

/// An AST search match location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstMatch {
    pub file: String,
    pub line: usize,
    pub matched_text: String,
    pub rule_kind: &'static str,
}

/// Convert byte offset in `content` to 1-based line number.
#[must_use]
pub fn offset_to_line(content: &str, byte_offset: usize) -> usize {
    let mut line = 1usize;
    for (idx, b) in content.bytes().enumerate() {
        if idx >= byte_offset {
            break;
        }
        if b == b'\n' {
            line = line.saturating_add(1);
        }
    }
    line
}

/// Determine language from file path extension.
#[must_use]
pub fn lang_for_path(path: &str) -> Option<SupportLang> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rs" => Some(SupportLang::Rust),
        "ts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "js" | "mjs" | "cjs" => Some(SupportLang::JavaScript),
        "jsx" => Some(SupportLang::JavaScript),
        "py" => Some(SupportLang::Python),
        "go" => Some(SupportLang::Go),
        "java" => Some(SupportLang::Java),
        "sh" | "bash" => Some(SupportLang::Bash),
        _ => None,
    }
}

/// Scan a file content using `ast-grep-core` for a specific declared identifier `name`.
#[must_use]
pub fn scan_ast(
    name: &str,
    file: &str,
    content: &str,
    lang: SupportLang,
    patterns: &[(&'static str, &'static str)],
) -> Vec<AstMatch> {
    // Fast-path byte guard: if content doesn't even contain the literal substring,
    // skip tree-sitter allocation entirely.
    if !content.contains(name) {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let grep = AstGrep::new(content, lang);
    let root = grep.root();

    for (rule_kind, pattern) in patterns {
        for m in root.find_all(*pattern) {
            if let Some(env_node) = m.get_env().get_match("NAME") {
                let raw_text = env_node.text().to_string();
                let clean_text = raw_text.trim_matches(|c: char| c == '"' || c == '\'');
                if clean_text == name {
                    let start_offset = env_node.range().start;
                    let line = offset_to_line(content, start_offset);
                    matches.push(AstMatch {
                        file: file.to_owned(),
                        line,
                        matched_text: clean_text.to_owned(),
                        rule_kind,
                    });
                }
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_ast_rust() {
        let code = r#"
            pub fn run_worker() {
                let status = 1;
            }
        "#;
        let patterns = &[("fn", "pub fn $NAME($$$) { $$$ }")];
        let res = scan_ast("run_worker", "worker.rs", code, SupportLang::Rust, patterns);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].line, 2);
        assert_eq!(res[0].matched_text, "run_worker");
    }

    #[test]
    fn test_scan_ast_unmatched_fast_path() {
        let code = "fn other() {}";
        let patterns = &[("fn", "fn $NAME() {}")];
        let res = scan_ast("missing", "test.rs", code, SupportLang::Rust, patterns);
        assert!(res.is_empty());
    }
}
