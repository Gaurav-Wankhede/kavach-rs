use std::path::Path;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::{AstGrep, Node};
use ast_grep_language::SupportLang;
use ignore::WalkBuilder;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub(crate) struct SymbolDef {
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub snippet: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct SymbolCaller {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub caller_scope: String,
    pub snippet: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct SymbolCallee {
    pub symbol: String,
    pub path: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct TraceReport {
    pub target_symbol: String,
    pub root_path: String,
    pub definitions: Vec<SymbolDef>,
    pub callers: Vec<SymbolCaller>,
    pub callees: Vec<SymbolCallee>,
    pub files_scanned: usize,
}

fn resolve_lang(path: &str) -> Option<SupportLang> {
    let ext = Path::new(path).extension()?.to_str()?;
    match ext {
        "rs" => Some(SupportLang::Rust),
        "ts" | "mts" | "cts" => Some(SupportLang::TypeScript),
        "tsx" => Some(SupportLang::Tsx),
        "js" | "mjs" | "cjs" => Some(SupportLang::JavaScript),
        "py" => Some(SupportLang::Python),
        "go" => Some(SupportLang::Go),
        "c" | "h" => Some(SupportLang::C),
        "cpp" | "cc" | "cxx" | "hpp" => Some(SupportLang::Cpp),
        "cs" => Some(SupportLang::CSharp),
        "java" => Some(SupportLang::Java),
        _ => None,
    }
}

fn offset_to_line_and_col(content: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, b) in content.bytes().enumerate() {
        if idx >= byte_offset {
            break;
        }
        if b == 10 {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn is_definition_node(node: &Node<'_, StrDoc<SupportLang>>, symbol: &str) -> Option<String> {
    let kind = node.kind();
    
    // Rust definitions
    if kind == "function_item" || kind == "struct_item" || kind == "enum_item" || kind == "trait_item" || kind == "type_item" || kind == "const_item" {
        if let Some(id_node) = node.field("name") {
            if id_node.text() == symbol {
                return Some(kind.to_string());
            }
        }
    }
    // TypeScript / JS
    if kind == "function_declaration" || kind == "class_declaration" || kind == "interface_declaration" || kind == "type_alias_declaration" || kind == "method_definition" {
        if let Some(id_node) = node.field("name") {
            if id_node.text() == symbol {
                return Some(kind.to_string());
            }
        }
    }
    // Python
    if kind == "function_definition" || kind == "class_definition" {
        if let Some(id_node) = node.field("name") {
            if id_node.text() == symbol {
                return Some(kind.to_string());
            }
        }
    }
    // Go
    if kind == "function_declaration" || kind == "method_declaration" || kind == "type_spec" {
        if let Some(id_node) = node.field("name") {
            if id_node.text() == symbol {
                return Some(kind.to_string());
            }
        }
    }

    None
}

fn find_enclosing_scope(node: &Node<'_, StrDoc<SupportLang>>) -> String {
    let mut current = node.parent();
    while let Some(p) = current {
        let k = p.kind();
        if k == "function_item" || k == "function_declaration" || k == "function_definition" || k == "method_declaration" || k == "method_definition" {
            if let Some(name_node) = p.field("name") {
                return name_node.text().to_string();
            }
            return k.to_string();
        }
        current = p.parent();
    }
    "<top-level>".to_string()
}

pub(crate) fn run(
    symbol: &str,
    path: &str,
    _depth: usize,
    callers_only: bool,
    callees_only: bool,
    json: bool,
) -> i32 {
    let target_path = Path::new(path);
    if !target_path.exists() {
        eprintln!("kavach trace: path '{}' does not exist", path);
        return 2;
    }

    let mut definitions = Vec::new();
    let mut callers = Vec::new();
    let mut callees = Vec::new();
    let mut files_scanned = 0;

    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        let p_str = entry.path().to_string_lossy();
        let lang = match resolve_lang(&p_str) {
            Some(l) => l,
            None => continue,
        };

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !content.contains(symbol) {
            continue;
        }

        files_scanned += 1;
        let grep = AstGrep::new(&content, lang);
        let root = grep.root();
        let all_lines: Vec<&str> = content.lines().collect();

        // 1. Check definitions & callees
        if !callers_only {
            for node in root.dfs() {
                if let Some(kind) = is_definition_node(&node, symbol) {
                    let range = node.range();
                    let (line, col) = offset_to_line_and_col(&content, range.start);
                    let line_idx = line.saturating_sub(1);
                    let snippet = all_lines.get(line_idx).copied().unwrap_or("").trim().to_string();

                    definitions.push(SymbolDef {
                        kind,
                        path: p_str.to_string(),
                        line,
                        col,
                        snippet,
                    });

                    // Collect outbound callees within this definition
                    for child in node.dfs() {
                        let c_kind = child.kind();
                        if c_kind == "call_expression" || c_kind == "method_call_expression" {
                            if let Some(fn_node) = child.field("function") {
                                let c_sym = fn_node.text().to_string();
                                if c_sym != symbol && !c_sym.is_empty() {
                                    let c_range = child.range();
                                    let (c_line, c_col) = offset_to_line_and_col(&content, c_range.start);
                                    callees.push(SymbolCallee {
                                        symbol: c_sym,
                                        path: p_str.to_string(),
                                        line: c_line,
                                        col: c_col,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Check inbound callers / references
        if !callees_only {
            for node in root.dfs() {
                if node.kind() == "identifier" && node.text() == symbol {
                    // Check if this identifier is part of a definition itself
                    let is_def = node.parent().map_or(false, |p| {
                        is_definition_node(&p, symbol).is_some()
                    });
                    if is_def {
                        continue;
                    }

                    let scope = find_enclosing_scope(&node);
                    let range = node.range();
                    let (line, col) = offset_to_line_and_col(&content, range.start);
                    let line_idx = line.saturating_sub(1);
                    let snippet = all_lines.get(line_idx).copied().unwrap_or("").trim().to_string();

                    callers.push(SymbolCaller {
                        path: p_str.to_string(),
                        line,
                        col,
                        caller_scope: scope,
                        snippet,
                    });
                }
            }
        }
    }

    if json {
        let report = TraceReport {
            target_symbol: symbol.to_string(),
            root_path: path.to_string(),
            definitions,
            callers,
            callees,
            files_scanned,
        };
        match serde_json::to_string_pretty(&report) {
            Ok(j) => println!("{}", j),
            Err(e) => {
                eprintln!("Error serializing trace output: {e}");
                return 1;
            }
        }
        return 0;
    }

    // Human & Agent ASCII format
    println!("Symbol: {} (scanned {} candidate files)", symbol, files_scanned);
    
    if !callees_only {
        println!("├── DEFINITIONS ({})", definitions.len());
        if definitions.is_empty() {
            println!("│   └── <none located>");
        } else {
            for (idx, def) in definitions.iter().enumerate() {
                let prefix = if idx == definitions.len() - 1 && callers.is_empty() && callees.is_empty() { "└──" } else { "├──" };
                println!("│   {} [{}] {}:{}:{}: {}", prefix, def.kind, def.path, def.line, def.col, def.snippet);
            }
        }
    }

    if !callees_only {
        println!("├── CALLED BY / INBOUND REFERENCES ({})", callers.len());
        if callers.is_empty() {
            println!("│   └── <none located>");
        } else {
            for (idx, caller) in callers.iter().enumerate() {
                let prefix = if idx == callers.len() - 1 && callees.is_empty() { "└──" } else { "├──" };
                println!("│   {} [{}] {}:{}:{}: {}", prefix, caller.caller_scope, caller.path, caller.line, caller.col, caller.snippet);
            }
        }
    }

    if !callers_only {
        println!("└── OUTBOUND CALLEES / DEPENDENCIES ({})", callees.len());
        if callees.is_empty() {
            println!("    └── <none located>");
        } else {
            for (idx, callee) in callees.iter().enumerate() {
                let prefix = if idx == callees.len() - 1 { "└──" } else { "├──" };
                println!("    {} {} at {}:{}:{}", prefix, callee.symbol, callee.path, callee.line, callee.col);
            }
        }
    }

    0
}
