//! Multi-language parser wrapper using tree-sitter.
//!
//! Provides a unified interface for parsing source code in multiple languages
//! using tree-sitter grammars. Returns lossless Concrete Syntax Trees (CST)
//! that preserve all tokens including whitespace and comments.

use std::collections::HashMap;
use tree_sitter::{Language, Parser, Tree};
use tree_sitter_markdown_fork;
use tree_sitter_sequel_tsql;
use tree_sitter_toml_ng;

/// Supported languages for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SupportedLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    TSX,
    Markdown,
    YAML,
    TOML,
    JSON,
    Bash,
    SQL,
    PlainText,
}

impl SupportedLanguage {
    /// Get the tree-sitter Language for this language.
    #[must_use]
    pub fn language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::TSX => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Markdown | Self::PlainText => tree_sitter_markdown_fork::language(),
            Self::YAML => tree_sitter_yaml::LANGUAGE.into(),
            Self::TOML => tree_sitter_toml_ng::LANGUAGE.into(),
            Self::JSON => tree_sitter_json::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::SQL => tree_sitter_sequel_tsql::LANGUAGE.into(),
        }
    }

    /// Detect language from file extension or content.
    #[must_use]
    pub fn detect_from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "jsx" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::TSX),
            "md" | "markdown" => Some(Self::Markdown),
            "yaml" | "yml" => Some(Self::YAML),
            "toml" => Some(Self::TOML),
            "json" => Some(Self::JSON),
            "sh" | "bash" => Some(Self::Bash),
            "sql" => Some(Self::SQL),
            _ => None,
        }
    }

    /// Detect language from content heuristics.
    #[must_use]
    pub fn detect_from_content(content: &str) -> Option<Self> {
        // Check for shebang
        if let Some(first_line) = content.lines().next()
            && first_line.starts_with("#!")
        {
            if first_line.contains("python") {
                return Some(Self::Python);
            }
            if first_line.contains("bash") || first_line.contains("sh") {
                return Some(Self::Bash);
            }
            if first_line.contains("node") {
                return Some(Self::JavaScript);
            }
        }

        // Heuristics for language detection
        if content.contains("fn main()") || content.contains("use std::") || content.contains("mod ") {
            return Some(Self::Rust);
        }
        if content.contains("def ") {
            return Some(Self::Python);
        }
        if content.contains("function ") || content.contains("const ") || content.contains("=>") {
            return Some(Self::JavaScript);
        }
        if content.contains("interface ") || content.contains("type ") || content.contains(": ") {
            return Some(Self::TypeScript);
        }
        if content.starts_with("# ") || content.contains("```") || content.contains("# ") {
            return Some(Self::Markdown);
        }
        if content.trim_start().starts_with("---") || content.contains(": ") && !content.contains("fn ") {
            return Some(Self::YAML);
        }
        if content.contains('[') && content.contains(']') && content.contains('=') && content.contains("version") {
            return Some(Self::TOML);
        }
        if content.trim_start().starts_with('{') || content.trim_start().starts_with('[') {
            return Some(Self::JSON);
        }
        if content.contains("SELECT ") || content.contains("INSERT ") || content.contains("CREATE TABLE") {
            return Some(Self::SQL);
        }
        if content.contains("#!/bin/bash") || content.contains("#!/bin/sh") {
            return Some(Self::Bash);
        }

        Some(Self::PlainText)
    }
}

/// Multi-language parser that manages parsers for different languages.
pub struct MultiLanguageParser {
    parsers: HashMap<SupportedLanguage, Parser>,
}

impl std::fmt::Debug for MultiLanguageParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiLanguageParser")
            .field("parsers", &self.parsers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl MultiLanguageParser {
    /// Create a new multi-language parser with all supported languages.
    ///
    /// # Errors
    /// Returns an error if any language fails to initialize.
    pub fn new() -> Result<Self, String> {
        let mut parsers = HashMap::new();

        for lang in [
            SupportedLanguage::Rust,
            SupportedLanguage::Python,
            SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript,
            SupportedLanguage::TSX,
            SupportedLanguage::Markdown,
            SupportedLanguage::YAML,
            SupportedLanguage::TOML,
            SupportedLanguage::JSON,
            SupportedLanguage::Bash,
            SupportedLanguage::SQL,
            SupportedLanguage::PlainText,
        ] {
            let mut parser = Parser::new();
            parser
                .set_language(&lang.language())
                .map_err(|e| format!("Failed to set language {lang:?}: {e:?}"))?;
            parsers.insert(lang, parser);
        }

        Ok(Self { parsers })
    }

    /// Parse source code with automatic language detection.
    ///
    /// # Errors
    /// Returns an error if parsing fails.
    pub fn parse_auto(&mut self, source: &str) -> Result<ParseResult, String> {
        let lang = SupportedLanguage::detect_from_content(source);
        self.parse_with_lang(source, lang.unwrap_or(SupportedLanguage::PlainText))
    }

    /// Parse source code with a specific language.
    ///
    /// # Errors
    /// Returns an error if the parser is not found or parsing fails.
    pub fn parse_with_lang(&mut self, source: &str, lang: SupportedLanguage) -> Result<ParseResult, String> {
        let parser = self
            .parsers
            .get_mut(&lang)
            .ok_or_else(|| format!("Parser not found for language: {lang:?}"))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| format!("Failed to parse source with {lang:?}"))?;

        Ok(ParseResult {
            tree,
            language: lang,
            source: source.to_owned(),
        })
    }

    /// Parse with explicit language override.
    ///
    /// # Errors
    /// Returns an error if parsing fails.
    pub fn parse(&mut self, source: &str, lang: SupportedLanguage) -> Result<ParseResult, String> {
        self.parse_with_lang(source, lang)
    }
}

/// Result of parsing containing the CST and metadata.
#[non_exhaustive]
pub struct ParseResult {
    pub tree: Tree,
    pub language: SupportedLanguage,
    pub source: String,
}

impl std::fmt::Debug for ParseResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParseResult")
            .field("language", &self.language)
            .field("source_len", &self.source.len())
            .field("tree", &"Tree (omitted)")
            .finish()
    }
}

impl ParseResult {
    /// Get the root node of the CST.
    #[must_use]
    pub fn root_node(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }

    /// Get the source text for a node.
    #[must_use]
    pub fn node_text<'a>(&'a self, node: tree_sitter::Node<'a>) -> &'a str {
        let range = node.byte_range();
        self.source.get(range).unwrap_or("")
    }

    /// Get all descendants of a node matching a predicate.
    #[expect(unreachable_code, reason = "tree-sitter cursor loop pattern")]
    pub fn find_nodes<'a, F>(&self, root: tree_sitter::Node<'a>, predicate: F) -> Vec<tree_sitter::Node<'a>>
    where
        F: Fn(tree_sitter::Node<'a>) -> bool,
    {
        let mut results = Vec::new();
        let mut cursor = root.walk();

        loop {
            let node = cursor.node();
            if predicate(node) {
                results.push(node);
            }

            if cursor.goto_first_child() {
                continue;
            }

            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return results;
                }
            }
        }

        results
    }

    /// Find all nodes of a specific kind.
    #[must_use]
    pub fn nodes_of_kind(&self, kind: &str) -> Vec<tree_sitter::Node<'_>> {
        self.find_nodes(self.tree.root_node(), |n| n.kind() == kind)
    }

    /// Get the text range of a node as (start_byte, end_byte).
    #[must_use]
    pub fn node_range(&self, node: tree_sitter::Node<'_>) -> (usize, usize) {
        (node.start_byte(), node.end_byte())
    }
}

impl Default for MultiLanguageParser {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self { parsers: HashMap::new() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_parsing() {
        let mut parser = MultiLanguageParser::new().unwrap();
        let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let result = parser.parse(code, SupportedLanguage::Rust).unwrap();
        assert!(result.tree.root_node().child_count() > 0);
    }

    #[test]
    fn test_python_parsing() {
        let mut parser = MultiLanguageParser::new().unwrap();
        let code = r#"
def hello():
    print("Hello, world!")
"#;
        let result = parser.parse(code, SupportedLanguage::Python).unwrap();
        assert!(result.tree.root_node().child_count() > 0);
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(SupportedLanguage::detect_from_extension("rs"), Some(SupportedLanguage::Rust));
        assert_eq!(SupportedLanguage::detect_from_extension("py"), Some(SupportedLanguage::Python));
        assert_eq!(SupportedLanguage::detect_from_extension("js"), Some(SupportedLanguage::JavaScript));
        assert_eq!(SupportedLanguage::detect_from_extension("ts"), Some(SupportedLanguage::TypeScript));
        assert_eq!(SupportedLanguage::detect_from_extension("tsx"), Some(SupportedLanguage::TSX));
        assert_eq!(SupportedLanguage::detect_from_extension("md"), Some(SupportedLanguage::Markdown));
        assert_eq!(SupportedLanguage::detect_from_extension("yaml"), Some(SupportedLanguage::YAML));
        assert_eq!(SupportedLanguage::detect_from_extension("yml"), Some(SupportedLanguage::YAML));
        assert_eq!(SupportedLanguage::detect_from_extension("toml"), Some(SupportedLanguage::TOML));
        assert_eq!(SupportedLanguage::detect_from_extension("json"), Some(SupportedLanguage::JSON));
        assert_eq!(SupportedLanguage::detect_from_extension("sh"), Some(SupportedLanguage::Bash));
        assert_eq!(SupportedLanguage::detect_from_extension("sql"), Some(SupportedLanguage::SQL));
    }
}
