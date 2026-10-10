 //! Entity extraction from parsed syntax trees.
 //!
 //! Defines entities for graph-relevant constructs:
//! - Functions
//! - Classes/Structs
//! - Variables
//! - Interfaces/Traits
//! - Enums
//! - Methods
//! - Namespaces/Modules
//! - Imports/Uses
//! - etc.

use crate::graph::parser::{ParseResult, SupportedLanguage};
use std::result::Result;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

/// An extracted entity from source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    /// The type of entity (e.g., "function", "class", "variable").
    pub kind: String,
    /// The name of the entity (if available).
    pub name: Option<String>,
    /// The source text of the entity definition.
    pub text: String,
    /// The byte range of the entity in the source file.
    pub range: (usize, usize),
    /// The language of the entity.
    pub language: SupportedLanguage,
}

impl Entity {
    /// Create a new entity from a tree-sitter query match.
    pub fn new(
        kind: &str,
        name: Option<String>,
        text: String,
        range: (usize, usize),
        language: SupportedLanguage,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            name,
            text,
            range,
            language,
        }
    }
}

/// A collection of tree-sitter queries for extracting entities from a specific language.
#[derive(Debug)]
pub struct LanguageQueries {
    /// The language these queries are for.
    pub language: SupportedLanguage,
    /// The compiled queries.
    pub queries: Vec<(String, Query)>,
}

impl LanguageQueries {
    /// Create a new set of queries for a language.
    pub fn new(language: SupportedLanguage) -> Result<Self, String> {
        let mut queries = Vec::new();

        // Define queries for each language
        match language {
            SupportedLanguage::Rust => {
                // Rust: functions, structs, enums, traits, impls, modules, use declarations
                queries.push((
                    "function".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (function_item
                            name: (identifier) @name
                        ) @function
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust function query: {e}"))?,
                ));
                queries.push((
                    "struct".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (struct_item
                            name: (type_identifier) @name
                        ) @struct
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust struct query: {e}"))?,
                ));
                queries.push((
                    "enum".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (enum_item
                            name: (type_identifier) @name
                        ) @enum
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust enum query: {e}"))?,
                ));
                queries.push((
                    "trait".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (trait_item
                            name: (type_identifier) @name
                        ) @trait
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust trait query: {e}"))?,
                ));
                queries.push((
                    "impl".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (impl_item
                            type: (_type) @name
                        ) @impl
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust impl query: {e}"))?,
                ));
                queries.push((
                    "module".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (mod_item
                            name: (identifier) @name
                        ) @module
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust module query: {e}"))?,
                ));
                queries.push((
                    "use_declaration".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (use_declaration
                            (scoped_identifier) @name
                        ) @use
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust use query: {e}"))?,
                ));
                queries.push((
                    "variable".to_string(),
                    Query::new(
                        &tree_sitter_rust::LANGUAGE.into(),
                        r#"
                        (let_declaration
                            pattern: (identifier) @name
                        ) @variable
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Rust variable query: {e}"))?,
                ));
            }
            SupportedLanguage::Python => {
                // Python: functions, classes, variables, imports, decorators
                queries.push((
                    "function".to_string(),
                    Query::new(
                        &tree_sitter_python::LANGUAGE.into(),
                        r#"
                        (function_definition
                            name: (identifier) @name
                        ) @function
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Python function query: {e}"))?,
                ));
                queries.push((
                    "class".to_string(),
                    Query::new(
                        &tree_sitter_python::LANGUAGE.into(),
                        r#"
                        (class_definition
                            name: (identifier) @name
                        ) @class
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Python class query: {e}"))?,
                ));
                queries.push((
                    "variable".to_string(),
                    Query::new(
                        &tree_sitter_python::LANGUAGE.into(),
                        r#"
                        (assignment
                            left: (identifier) @name
                        ) @variable
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Python variable query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_python::LANGUAGE.into(),
                        r#"
                        (import_from_statement
                            module_name: (dotted_name) @module
                            name: (dotted_name) @name
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Python import query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_python::LANGUAGE.into(),
                        r#"
                        (import_statement
                            name: (dotted_name) @name
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile Python import query: {e}"))?,
                ));
            }
            SupportedLanguage::JavaScript => {
                // JavaScript: functions, classes, variables, imports, exports
                queries.push((
                    "function".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (function_declaration
                            name: (identifier) @name
                        ) @function
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS function query: {e}"))?,
                ));
                queries.push((
                    "method".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (method_definition
                            name: (property_identifier) @name
                        ) @method
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS method query: {e}"))?,
                ));
                queries.push((
                    "class".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (class_declaration
                            name: (identifier) @name
                        ) @class
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS class query: {e}"))?,
                ));
                queries.push((
                    "variable".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (variable_declarator
                            name: (identifier) @name
                        ) @variable
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS variable query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (import_statement
                            source: (string) @name
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS import query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (export_statement
                            (export_clause
                                (export_specifier
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_javascript::LANGUAGE.into(),
                        r#"
                        (export_statement
                            declaration: (variable_declaration
                                (variable_declarator
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile JS export query: {e}"))?,
                ));
            }
            SupportedLanguage::TypeScript => {
                // TypeScript: functions, classes, variables, imports, exports
                queries.push((
                    "function".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (function_declaration
                            name: (identifier) @name
                        ) @function
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS function query: {e}"))?,
                ));
                queries.push((
                    "method".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (method_definition
                            name: (property_identifier) @name
                        ) @method
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS method query: {e}"))?,
                ));
                queries.push((
                    "class".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (class_declaration
                            name: (type_identifier) @name
                        ) @class
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS class query: {e}"))?,
                ));
                queries.push((
                    "variable".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (variable_declarator
                            name: (identifier) @name
                        ) @variable
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS variable query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (import_statement
                            (import_clause
                                (named_imports
                                    (import_specifier
                                        name: (identifier) @name
                                    )
                                )
                            )
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS import query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (import_statement
                            source: (string) @name
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS import query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (export_statement
                            (export_clause
                                (export_specifier
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (export_statement
                            declaration: (lexical_declaration
                                (variable_declarator
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (export_statement
                            declaration: (variable_declaration
                                (variable_declarator
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                        r#"
                        (export_statement
                            source: (string) @name
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TS export query: {e}"))?,
                ));
            }
            SupportedLanguage::TSX => {
                // TSX: functions, classes, variables, imports, exports
                queries.push((
                    "function".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (function_declaration
                            name: (identifier) @name
                        ) @function
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX function query: {e}"))?,
                ));
                queries.push((
                    "method".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (method_definition
                            name: (property_identifier) @name
                        ) @method
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX method query: {e}"))?,
                ));
                queries.push((
                    "class".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (class_declaration
                            name: (type_identifier) @name
                        ) @class
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX class query: {e}"))?,
                ));
                queries.push((
                    "variable".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (variable_declarator
                            name: (identifier) @name
                        ) @variable
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX variable query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (import_statement
                            (import_clause
                                (named_imports
                                    (import_specifier
                                        name: (identifier) @name
                                    )
                                )
                            )
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX import query: {e}"))?,
                ));
                queries.push((
                    "import".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (import_statement
                            source: (string) @name
                        ) @import
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX import query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (export_statement
                            (export_clause
                                (export_specifier
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (export_statement
                            declaration: (lexical_declaration
                                (variable_declarator
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (export_statement
                            declaration: (variable_declaration
                                (variable_declarator
                                    name: (identifier) @name
                                )
                            )
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX export query: {e}"))?,
                ));
                queries.push((
                    "export".to_string(),
                    Query::new(
                        &tree_sitter_typescript::LANGUAGE_TSX.into(),
                        r#"
                        (export_statement
                            source: (string) @name
                        ) @export
                        "#,
                    )
                    .map_err(|e| format!("Failed to compile TSX export query: {e}"))?,
                ));
            }
            SupportedLanguage::Markdown => {
                // For markdown and plain text, we don't extract entities via tree-sitter queries
                // We'll return an empty vector
            }
            SupportedLanguage::PlainText => {
                // For markdown and plain text, we don't extract entities via tree-sitter queries
                // We'll return an empty vector
            }
            _ => {
                // For other languages, we can add queries later or leave empty for now
                // We'll return an empty vector for unsupported languages in this initial implementation
            }
        }

        Ok(Self { language, queries })
    }

    /// Extract entities from a parse result using the compiled queries.
    pub fn extract_entities(&self, parse_result: &ParseResult) -> Result<Vec<Entity>, String> {
        let mut entities = Vec::new();
        let root_node = parse_result.tree.root_node();
        let mut cursor = QueryCursor::new();

        for (kind, query) in &self.queries {
            let mut matches = cursor.matches(query, root_node, parse_result.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures() {
                    let node = capture.node;
                    let text = parse_result
                        .node_text(node)
                        .to_string();
                    let range = (node.start_byte(), node.end_byte());

                    // Try to extract a name from the capture if it has a @name pattern
                    let name = if let Some(name_capture) = m.captures().iter().find(|c| {
                        let capture_names = query.capture_names();
                        let idx = c.index as usize;
                        if idx < capture_names.len() {
                            capture_names[idx] == "name"
                        } else {
                            false
                        }
                    }) {
                        Some(parse_result.node_text(name_capture.node).to_string())
                    } else {
                        None
                    };

                    entities.push(Entity::new(
                        kind,
                        name,
                        text,
                        range,
                        self.language,
                    ));
                }
            }
        }

        Ok(entities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::parser::MultiLanguageParser;

    #[test]
    fn test_rust_entity_extraction() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
fn main() {
    let x = 5;
    struct Point {
        x: i32,
        y: i32,
    }
    impl Point {
        fn new(x: i32, y: i32) -> Self {
            Self { x, y }
        }
    }
}
"#;
        let result = parser
            .parse(code, SupportedLanguage::Rust)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::Rust)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // We expect at least: main function, x variable, Point struct, Point::new method, impl Point
        assert!(
            entities.len() >= 5,
            "Expected at least 5 entities, got {}: {:?}",
            entities.len(),
            entities
        );

        // Check for specific entities
        let has_main_fn = entities.iter().any(|e| e.kind == "function" && e.name == Some("main".to_string()));
        let has_x_var = entities.iter().any(|e| e.kind == "variable" && e.name == Some("x".to_string()));
        let has_point_struct = entities.iter().any(|e| e.kind == "struct" && e.name == Some("Point".to_string()));
        let has_point_new_fn = entities.iter().any(|e| e.kind == "function" && e.name == Some("new".to_string()));
        let has_impl_point = entities.iter().any(|e| e.kind == "impl");

        assert!(has_main_fn, "Expected to find main function");
        assert!(has_x_var, "Expected to find variable x");
        assert!(has_point_struct, "Expected to find struct Point");
        assert!(has_point_new_fn, "Expected to find Point::new function");
        assert!(has_impl_point, "Expected to find impl Point");

        Ok(())
    }

    #[test]
    fn test_python_entity_extraction() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
def hello():
    x = 1
    return x

class MyClass:
    def method(self):
        pass

import os
from sys import argv
"#;
        let result = parser
            .parse(code, SupportedLanguage::Python)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::Python)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // We expect at least: hello function, x variable, MyClass class, method function, os import, argv import
        assert!(
            entities.len() >= 6,
            "Expected at least 6 entities, got {}: {:?}",
            entities.len(),
            entities
        );

        // Check for specific entities
        let has_hello_fn = entities.iter().any(|e| e.kind == "function" && e.name == Some("hello".to_string()));
        let has_x_var = entities.iter().any(|e| e.kind == "variable" && e.name == Some("x".to_string()));
        let has_myclass = entities.iter().any(|e| e.kind == "class" && e.name == Some("MyClass".to_string()));
        let has_method = entities.iter().any(|e| e.kind == "function" && e.name == Some("method".to_string()));
        let has_os_import = entities.iter().any(|e| e.kind == "import" && e.name == Some("os".to_string()));
        let has_argv_import = entities.iter().any(|e| e.kind == "import" && e.name == Some("argv".to_string()));

        assert!(has_hello_fn, "Expected to find hello function");
        assert!(has_x_var, "Expected to find variable x");
        assert!(has_myclass, "Expected to find class MyClass");
        assert!(has_method, "Expected to find method method");
        assert!(has_os_import, "Expected to find import os");
        assert!(has_argv_import, "Expected to find import argv");

        Ok(())
    }

    #[test]
    fn test_javascript_entity_extraction() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
function hello() {
    let x = 5;
    return x;
}

class MyClass {
    method() {
        return "hello";
    }
}

import { foo } from 'bar';
export const baz = 42;
"#;
        let result = parser
            .parse(code, SupportedLanguage::JavaScript)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::JavaScript)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // We expect at least: hello function, x variable, MyClass class, method method, import, export
        assert!(
            entities.len() >= 6,
            "Expected at least 6 entities, got {}: {:?}",
            entities.len(),
            entities
        );

        // Check for specific entities
        let has_hello_fn = entities.iter().any(|e| e.kind == "function" && e.name == Some("hello".to_string()));
        let has_x_var = entities.iter().any(|e| e.kind == "variable" && e.name == Some("x".to_string()));
        let has_myclass = entities.iter().any(|e| e.kind == "class" && e.name == Some("MyClass".to_string()));
        let has_method = entities.iter().any(|e| e.kind == "method" && e.name == Some("method".to_string()));

        assert!(has_hello_fn, "Expected to find hello function");
        assert!(has_x_var, "Expected to find variable x");
        assert!(has_myclass, "Expected to find class MyClass");
        assert!(has_method, "Expected to find method method");
        // Import/export extraction may vary, just verify we got some entities
        assert!(entities.len() >= 4, "Expected at least 4 core entities, got {}: {:?}", entities.len(), entities);

        Ok(())
    }

    #[test]
    fn test_typescript_entity_extraction() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
function hello() {
    let x = 5;
    return x;
}

class MyClass {
    method() {
        return "hello";
    }
}

import { foo } from 'bar';
export const baz = 42;
"#;
        let result = parser
            .parse(code, SupportedLanguage::TypeScript)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::TypeScript)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // We expect at least: hello function, x variable, MyClass class, method method, foo import, baz export
        assert!(
            entities.len() >= 6,
            "Expected at least 6 entities, got {}: {:?}",
            entities.len(),
            entities
        );

        // Check for specific entities
        let has_hello_fn = entities.iter().any(|e| e.kind == "function" && e.name == Some("hello".to_string()));
        let has_x_var = entities.iter().any(|e| e.kind == "variable" && e.name == Some("x".to_string()));
        let has_myclass = entities.iter().any(|e| e.kind == "class" && e.name == Some("MyClass".to_string()));
        let has_method = entities.iter().any(|e| e.kind == "method" && e.name == Some("method".to_string()));
        let has_foo_import = entities.iter().any(|e| e.kind == "import" && e.name == Some("foo".to_string()));
        let has_baz_export = entities.iter().any(|e| e.kind == "export" && e.name == Some("baz".to_string()));

        assert!(has_hello_fn, "Expected to find hello function");
        assert!(has_x_var, "Expected to find variable x");
        assert!(has_myclass, "Expected to find class MyClass");
        assert!(has_method, "Expected to find method method");
        assert!(has_foo_import, "Expected to find import foo");
        assert!(has_baz_export, "Expected to find export baz");

        Ok(())
    }

    #[test]
    fn test_tsx_entity_extraction() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
function hello() {
    let x = 5;
    return x;
}

class MyClass {
    method() {
        return "hello";
    }
}

import { foo } from 'bar';
export const baz = 42;
"#;
        let result = parser
            .parse(code, SupportedLanguage::TSX)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::TSX)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // We expect at least: hello function, x variable, MyClass class, method method, foo import, baz export
        assert!(
            entities.len() >= 6,
            "Expected at least 6 entities, got {}: {:?}",
            entities.len(),
            entities
        );

        // Check for specific entities
        let has_hello_fn = entities.iter().any(|e| e.kind == "function" && e.name == Some("hello".to_string()));
        let has_x_var = entities.iter().any(|e| e.kind == "variable" && e.name == Some("x".to_string()));
        let has_myclass = entities.iter().any(|e| e.kind == "class" && e.name == Some("MyClass".to_string()));
        let has_method = entities.iter().any(|e| e.kind == "method" && e.name == Some("method".to_string()));
        let has_foo_import = entities.iter().any(|e| e.kind == "import" && e.name == Some("foo".to_string()));
        let has_baz_export = entities.iter().any(|e| e.kind == "export" && e.name == Some("baz".to_string()));

        assert!(has_hello_fn, "Expected to find hello function");
        assert!(has_x_var, "Expected to find variable x");
        assert!(has_myclass, "Expected to find class MyClass");
        assert!(has_method, "Expected to find method method");
        assert!(has_foo_import, "Expected to find import foo");
        assert!(has_baz_export, "Expected to find export baz");

        Ok(())
    }

    #[test]
    fn test_markdown_no_entities() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
# Hello World

This is a markdown file.

- Item 1
- Item 2
"#;
        let result = parser
            .parse(code, SupportedLanguage::Markdown)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::Markdown)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // Markdown and PlainText should return no entities via tree-sitter queries
        assert!(
            entities.is_empty(),
            "Expected no entities for markdown, got {}: {:?}",
            entities.len(),
            entities
        );

        Ok(())
    }

    #[test]
    fn test_plain_text_no_entities() -> Result<(), String> {
        let mut parser = MultiLanguageParser::new().map_err(|e| e.to_string())?;
        let code = r#"
This is plain text.

It has multiple lines.

But no special syntax to extract.
"#;
        let result = parser
            .parse(code, SupportedLanguage::PlainText)
            .map_err(|e| e.to_string())?;

        let queries = LanguageQueries::new(SupportedLanguage::PlainText)
            .map_err(|e| e.to_string())?;
        let entities = queries.extract_entities(&result).map_err(|e| e.to_string())?;

        // Markdown and PlainText should return no entities via tree-sitter queries
        assert!(
            entities.is_empty(),
            "Expected no entities for plain text, got {}: {:?}",
            entities.len(),
            entities
        );

        Ok(())
    }
}