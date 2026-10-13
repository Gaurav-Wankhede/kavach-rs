//! Dynamic AST pattern engine for `kavach-patterns`.
//!
//! Replaces fragile regexes and character-window heuristics with
//! sovereign syntax tree traversal powered by `ast-grep-core`.

use ast_grep_core::Node;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;

/// Detect the appropriate AST language from file extension.
#[must_use]
pub fn lang_for_file(path: &str) -> Option<SupportLang> {
    let ext = std::path::Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(SupportLang::Rust),
        "ts" | "mts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "js" | "mjs" | "cjs" => Some(SupportLang::JavaScript),
        "jsx" => Some(SupportLang::Tsx),
        "html" | "htm" => Some(SupportLang::Html),
        "css" => Some(SupportLang::Css),
        "json" => Some(SupportLang::Json),
        _ => None,
    }
}

/// AST Node coordinate information for zero-defect telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AstSpan {
    pub line: usize,
    pub column: usize,
}

/// Dynamic AST inspection handle.
pub type DocType = StrDoc<SupportLang>;
pub type AstNode<'a> = Node<'a, DocType>;

/// Dynamic AST inspection handle.
pub struct AstInspector {
    grep: ast_grep_core::AstGrep<DocType>,
}

impl std::fmt::Debug for AstInspector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstInspector").finish_non_exhaustive()
    }
}

impl AstInspector {
    /// Create a new inspector if the file type is supported by tree-sitter.
    #[must_use]
    pub fn new(path: &str, content: &str) -> Option<Self> {
        let lang = lang_for_file(path)?;
        let grep = ast_grep_core::AstGrep::new(content, lang);
        Some(Self { grep })
    }

    /// Walk all loop constructs across Rust, TS, and JS (`for`, `while`, `loop`).
    pub fn find_loops(&self) -> Vec<AstNode<'_>> {
        self.grep
            .root()
            .dfs()
            .filter(|node| {
                matches!(
                    node.kind().as_ref(),
                    "for_expression"
                        | "while_expression"
                        | "loop_expression"
                        | "for_statement"
                        | "for_in_statement"
                        | "while_statement"
                        | "do_statement"
                )
            })
            .collect()
    }

    /// Check if a given AST subtree contains method calls matching any name in `targets`.
    #[must_use]
    pub fn has_call_to(node: &AstNode<'_>, targets: &[&str]) -> Option<AstSpan> {
        for child in node.dfs() {
            let kind = child.kind();
            if kind == "field_identifier" || kind == "property_identifier" || kind == "identifier" {
                let name = child.text();
                if targets.iter().any(|t| *t == name.as_ref()) {
                    let pos = child.start_pos();
                    return Some(AstSpan {
                        line: pos.line().saturating_add(1),
                        column: pos.column(&child).saturating_add(1),
                    });
                }
            }
        }
        None
    }

    /// Check if an AST subtree contains a macro invocation matching target prefix.
    #[must_use]
    pub fn has_macro_call(node: &AstNode<'_>, name: &str) -> Option<AstSpan> {
        for child in node.dfs() {
            if child.kind() == "macro_invocation" {
                let text = child.text();
                if text.starts_with(name) {
                    let pos = child.start_pos();
                    return Some(AstSpan {
                        line: pos.line().saturating_add(1),
                        column: pos.column(&child).saturating_add(1),
                    });
                }
            }
        }
        None
    }

    /// Check if an AST subtree contains a compound assignment.
    #[must_use]
    pub fn has_compound_assignment(node: &AstNode<'_>) -> Option<AstSpan> {
        for child in node.dfs() {
            if child.kind() == "compound_assignment_expr" {
                let pos = child.start_pos();
                return Some(AstSpan {
                    line: pos.line().saturating_add(1),
                    column: pos.column(&child).saturating_add(1),
                });
            }
        }
        None
    }
}
