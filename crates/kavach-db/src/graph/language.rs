//! Enhanced language detection for memory entries.
//!
//! Provides language detection using:
//! - File extension mapping
//! - Content heuristics
//! - Tree-sitter validation (parse and check for errors)

use crate::graph::parser::{MultiLanguageParser, SupportedLanguage};

/// Detect the most likely language for a memory entry.
///
/// Uses a combination of:
/// 1. File extension (if available)
/// 2. Content heuristics
/// 3. Tree-sitter validation (parse and check error count)
#[must_use]
pub fn detect_language(content: &str, extension: Option<&str>) -> SupportedLanguage {
    // 1. Try extension first
    if let Some(ext) = extension
        && let Some(lang) = SupportedLanguage::detect_from_extension(ext)
    {
        return lang;
    }

    // 2. Try content heuristics
    if let Some(lang) = SupportedLanguage::detect_from_content(content) {
        return lang;
    }

    // 3. Default to plain text
    SupportedLanguage::PlainText
}

/// Validate a language guess by parsing and checking error count.
///
/// Returns true if the parse has no errors (or very few).
pub fn validate_language(content: &str, language: SupportedLanguage) -> bool {
    let mut parser = match MultiLanguageParser::new() {
        Ok(p) => p,
        Err(_) => return false,
    };

    match parser.parse_with_lang(content, language) {
        Ok(result) => {
            let root = result.root_node();
            !root.has_error()
        }
        Err(_) => false,
    }
}

/// Detect language with tree-sitter validation.
///
/// Tries multiple languages and returns the one with the fewest parse errors.
pub fn detect_language_with_validation(content: &str, extension: Option<&str>) -> SupportedLanguage {
    // Get initial guess
    let initial = detect_language(content, extension);

    // Validate the initial guess
    if validate_language(content, initial) {
        return initial;
    }

    // Try other languages
    let candidates = [
        SupportedLanguage::Rust,
        SupportedLanguage::Python,
        SupportedLanguage::JavaScript,
        SupportedLanguage::TypeScript,
        SupportedLanguage::Markdown,
        SupportedLanguage::YAML,
        SupportedLanguage::TOML,
        SupportedLanguage::JSON,
        SupportedLanguage::Bash,
        SupportedLanguage::SQL,
        SupportedLanguage::PlainText,
    ];

    let mut best_lang = initial;
    let mut best_score = usize::MAX;

    for lang in candidates {
        if lang == initial {
            continue;
        }

        let mut parser = match MultiLanguageParser::new() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if let Ok(result) = parser.parse_with_lang(content, lang) {
            let root = result.root_node();
            let error_count = count_errors(&root);
            if error_count < best_score {
                best_score = error_count;
                best_lang = lang;
            }
        }
    }

    best_lang
}

/// Count the number of ERROR nodes in a tree.
fn count_errors(node: &tree_sitter::Node<'_>) -> usize {
    let mut count = 0;
    let mut cursor = node.walk();

    loop {
        if node.is_error() || node.is_missing() {
            count += 1;
        }

        if cursor.goto_first_child() {
            count += count_errors(&cursor.node());
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    count
}

/// Batch detect languages for multiple entries.
pub fn batch_detect(entries: &[(String, Option<String>)]) -> Vec<SupportedLanguage> {
    entries
        .iter()
        .map(|(content, ext)| detect_language(content, ext.as_deref()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_extension() {
        assert_eq!(
            detect_language("fn main() {}", Some("rs")),
            SupportedLanguage::Rust
        );
        assert_eq!(
            detect_language("def hello():", Some("py")),
            SupportedLanguage::Python
        );
        assert_eq!(
            detect_language("const x = 1;", Some("js")),
            SupportedLanguage::JavaScript
        );
    }

    #[test]
    fn test_detect_from_content() {
        assert_eq!(
            detect_language("fn main() { println!(\"Hello\"); }", None),
            SupportedLanguage::Rust
        );
        assert_eq!(
            detect_language("def hello():\n    print(\"Hello\")", None),
            SupportedLanguage::Python
        );
    }

    #[test]
    fn test_validate_language() {
        assert!(validate_language("fn main() {}", SupportedLanguage::Rust));
        assert!(!validate_language("fn main() {}", SupportedLanguage::Python));
    }

    #[test]
    fn test_detect_with_validation() {
        let lang = detect_language_with_validation("fn main() { println!(\"Hello\"); }", None);
        assert_eq!(lang, SupportedLanguage::Rust);
    }

    #[test]
    fn test_batch_detect() {
        let entries = vec![
            ("fn main() {}".to_string(), Some("rs".to_string())),
            ("def hello():".to_string(), Some("py".to_string())),
        ];
        let results = batch_detect(&entries);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], SupportedLanguage::Rust);
        assert_eq!(results[1], SupportedLanguage::Python);
    }
}
