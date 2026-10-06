//! AST-based declaration matcher using ast-grep-core and ast-grep-language.
//! Resolves true declaration nodes in syntax trees, preventing false positives
//! in doc comments, string literals, and generic test bodies.

use super::site::{Kind, Site};
use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;
use std::path::Path;

/// Detect the programming language from file extension.
pub(super) fn detect_lang(file: &str) -> Option<SupportLang> {
    let ext = Path::new(file).extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(SupportLang::Rust),
        "ts" | "mts" | "cts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "js" | "mjs" | "cjs" | "jsx" => Some(SupportLang::JavaScript),
        "py" | "pyi" => Some(SupportLang::Python),
        "go" => Some(SupportLang::Go),
        "c" | "h" => Some(SupportLang::C),
        "cpp" | "cc" | "cxx" | "hpp" => Some(SupportLang::Cpp),
        "cs" => Some(SupportLang::CSharp),
        "java" => Some(SupportLang::Java),
        "sh" | "bash" => Some(SupportLang::Bash),
        _ => None,
    }
}

const fn query_patterns(lang: SupportLang) -> &'static [(Kind, &'static str)] {
    match lang {
        SupportLang::Rust => &[
            (Kind::Function, "fn $NAME($$$) { $$$ }"),
            (Kind::Function, "pub fn $NAME($$$) { $$$ }"),
            (Kind::Function, "pub(crate) fn $NAME($$$) { $$$ }"),
            (Kind::Function, "fn $NAME($$$);"),
            (Kind::Function, "pub fn $NAME($$$);"),
            (Kind::Type, "struct $NAME { $$$ }"),
            (Kind::Type, "pub struct $NAME { $$$ }"),
            (Kind::Type, "struct $NAME($$$);"),
            (Kind::Type, "pub struct $NAME($$$);"),
            (Kind::Type, "struct $NAME;"),
            (Kind::Type, "pub struct $NAME;"),
            (Kind::Type, "enum $NAME { $$$ }"),
            (Kind::Type, "pub enum $NAME { $$$ }"),
            (Kind::Type, "trait $NAME { $$$ }"),
            (Kind::Type, "pub trait $NAME { $$$ }"),
            (Kind::Type, "type $NAME = $$$;"),
            (Kind::Type, "pub type $NAME = $$$;"),
            (Kind::Type, "union $NAME { $$$ }"),
            (Kind::Type, "pub union $NAME { $$$ }"),
            (Kind::Const, "const $NAME: $$$ = $$$;"),
            (Kind::Const, "pub const $NAME: $$$ = $$$;"),
            (Kind::Static, "static $NAME: $$$ = $$$;"),
            (Kind::Static, "pub static $NAME: $$$ = $$$;"),
            (Kind::LetBinding, "let $NAME = $$$;"),
            (Kind::LetBinding, "let mut $NAME = $$$;"),
            (Kind::LetBinding, "let $NAME: $$$ = $$$;"),
            (Kind::LetBinding, "let mut $NAME: $$$ = $$$;"),
            (Kind::Default, "impl Default for $NAME { $$$ }"),
            (Kind::ConfigField, "$NAME: $TYPE,"),
            (Kind::ConfigField, "pub $NAME: $TYPE,"),
            (Kind::ConfigField, "$NAME: $TYPE"),
            (Kind::ConfigField, "pub $NAME: $TYPE"),
            (Kind::Param, "fn $FN($$$ $NAME: $TYPE $$$) { $$$ }"),
            (Kind::Param, "fn $FN($$$ $NAME: $TYPE $$$);"),
            (Kind::EnvVar, "std::env::var($NAME)"),
            (Kind::EnvVar, "env::var($NAME)"),
        ],
        SupportLang::Python => &[
            (Kind::Function, "def $NAME($$$): $$$"),
            (Kind::Function, "async def $NAME($$$): $$$"),
            (Kind::Type, "class $NAME: $$$"),
            (Kind::Type, "class $NAME($$$): $$$"),
            (Kind::EnvVar, "os.environ[$NAME]"),
            (Kind::EnvVar, "os.environ.get($NAME, $$$)"),
            (Kind::EnvVar, "os.getenv($NAME)"),
            (Kind::EnvVar, "os.getenv($NAME, $$$)"),
        ],
        SupportLang::TypeScript | SupportLang::Tsx => &[
            (Kind::Function, "function $NAME($$$) { $$$ }"),
            (Kind::Function, "const $NAME = ($$$) => { $$$ }"),
            (Kind::Function, "const $NAME = async ($$$) => { $$$ }"),
            (Kind::Function, "export const $NAME = ($$$) => { $$$ }"),
            (Kind::Function, "export const $NAME = async ($$$) => { $$$ }"),
            (Kind::Function, "let $NAME = ($$$) => { $$$ }"),
            (Kind::Function, "var $NAME = ($$$) => { $$$ }"),
            (Kind::Type, "class $NAME { $$$ }"),
            (Kind::Type, "interface $NAME { $$$ }"),
            (Kind::Type, "type $NAME = $$$;"),
            (Kind::Type, "enum $NAME { $$$ }"),
            (Kind::Const, "const $NAME: $$$ = $$$;"),
            (Kind::Const, "const $NAME = $$$;"),
            (Kind::LetBinding, "let $NAME = $$$;"),
            (Kind::EnvVar, "process.env.$NAME"),
            (Kind::EnvVar, "process.env[$NAME]"),
        ],
        SupportLang::JavaScript => &[
            (Kind::Function, "function $NAME($$$) { $$$ }"),
            (Kind::Function, "const $NAME = ($$$) => { $$$ }"),
            (Kind::Function, "const $NAME = async ($$$) => { $$$ }"),
            (Kind::Function, "let $NAME = ($$$) => { $$$ }"),
            (Kind::Function, "var $NAME = ($$$) => { $$$ }"),
            (Kind::Type, "class $NAME { $$$ }"),
            (Kind::Const, "const $NAME = $$$;"),
            (Kind::LetBinding, "let $NAME = $$$;"),
            (Kind::EnvVar, "process.env.$NAME"),
            (Kind::EnvVar, "process.env[$NAME]"),
        ],
        SupportLang::Go => &[
            (Kind::Function, "func $NAME($$$) { $$$ }"),
            (Kind::Function, "func ($$$) $NAME($$$) { $$$ }"),
            (Kind::Type, "type $NAME struct { $$$ }"),
            (Kind::Type, "type $NAME interface { $$$ }"),
            (Kind::Const, "const $NAME = $$$"),
            (Kind::LetBinding, "var $NAME = $$$"),
            (Kind::EnvVar, "os.Getenv($NAME)"),
        ],
        _ => &[],
    }
}

fn offset_to_line(content: &str, byte_offset: usize) -> usize {
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

/// Scan a file using genuine AST queries for declarations of `name`.
pub(super) fn scan_ast(name: &str, file: &str, content: &str, lang: SupportLang) -> Vec<Site> {
    let mut sites = Vec::new();
    let grep = AstGrep::new(content, lang);
    let root = grep.root();

    for (kind, pattern) in query_patterns(lang) {
        for m in root.find_all(*pattern) {
            if let Some(env_node) = m.get_env().get_match("NAME") {
                let raw_text = env_node.text().to_string();
                let clean_text = raw_text.trim_matches(|c: char| c == '"' || c == '\'');
                if clean_text == name {
                    let start_offset = env_node.range().start;
                    let line = offset_to_line(content, start_offset);
                    sites.push(Site {
                        kind: *kind,
                        file: file.to_owned(),
                        line,
                    });
                }
            }
        }
    }

    if lang == SupportLang::Rust {
        collect_rust_enum_variants(&root, name, file, content, &mut sites);
    }

    sites
}

fn collect_rust_enum_variants(
    root: &ast_grep_core::Node<'_, ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    name: &str,
    file: &str,
    content: &str,
    sites: &mut Vec<Site>,
) {
    for m in root.find_all("enum $NAME { $$$ }") {
        for child in m.children() {
            for node in child.children() {
                if node.kind() == "enum_variant"
                    && let Some(ident) = node.child(0)
                    && ident.text() == name
                {
                    let start_offset = ident.range().start;
                    let line = offset_to_line(content, start_offset);
                    sites.push(Site {
                        kind: Kind::Variant,
                        file: file.to_owned(),
                        line,
                    });
                }
            }
        }
    }
}
