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

pub(crate) fn resolve_lang(path: &str) -> Option<SupportLang> {
    use ast_grep_language::Language;
    SupportLang::from_path(Path::new(path))
}

pub(crate) fn offset_to_line_and_col(content: &str, byte_offset: usize) -> (usize, usize) {
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

/// Clean a callee expression or function identifier down to its base symbol name
pub(crate) fn extract_symbol_name(text: &str) -> String {
    let trimmed = text.trim();
    // Handle path or member separators (e.g. obj.method, Mod::func, a->b)
    if let Some(last) = trimmed.split(|c| c == '.' || c == ':').filter(|s| !s.is_empty()).last() {
        // Strip trailing parentheses or call arguments if present
        let clean = last.split('(').next().unwrap_or(last).trim();
        clean.to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn is_definition_node(node: &Node<'_, StrDoc<SupportLang>>, symbol: &str) -> Option<String> {
    let (kind, name) = any_definition_node(node)?;
    if name == symbol {
        Some(kind)
    } else {
        None
    }
}

pub(crate) fn any_definition_node(node: &Node<'_, StrDoc<SupportLang>>) -> Option<(String, String)> {
    let kind = node.kind();
    
    // 1. Direct declaration name field (Rust, TS/JS, Python, Go, C/C++, Java, C#)
    let is_decl_kind = kind.contains("item")
        || kind.contains("declaration")
        || kind.contains("definition")
        || kind.contains("spec")
        || kind.contains("clause");

    if is_decl_kind {
        if let Some(id_node) = node.field("name") {
            let sym = id_node.text().to_string();
            if !sym.is_empty() {
                if kind.contains("function") || kind.contains("method") || kind.contains("fn") {
                    return Some(("function".to_string(), sym));
                }
                if kind.contains("class") || kind.contains("struct") {
                    return Some(("class".to_string(), sym));
                }
                if kind.contains("interface") || kind.contains("trait") {
                    return Some(("interface".to_string(), sym));
                }
                if kind.contains("type") {
                    return Some(("type".to_string(), sym));
                }
                if kind.contains("enum") {
                    return Some(("enum".to_string(), sym));
                }
                if kind.contains("const") {
                    return Some(("const".to_string(), sym));
                }
                return Some((kind.to_string(), sym));
            }
        }
    }

    // 2. Variable declarators / Arrow functions (JS/TS/Go)
    // const foo = () => {}, let bar = function()
    if kind == "variable_declarator" || kind == "lexical_declaration" || kind == "short_var_declaration" {
        if let Some(id_node) = node.field("name") {
            let sym = id_node.text().to_string();
            if let Some(val_node) = node.field("value") {
                let vk = val_node.kind();
                if vk == "arrow_function" || vk == "function_expression" || vk == "function" {
                    return Some(("function".to_string(), sym));
                }
            }
            return Some(("variable".to_string(), sym));
        }
    }

    // 3. Python assignment / lambda (foo = lambda x: ...)
    if kind == "assignment" {
        if let Some(left) = node.field("left") {
            let sym = left.text().to_string();
            if let Some(right) = node.field("right") {
                if right.kind() == "lambda" {
                    return Some(("function".to_string(), sym));
                }
            }
            return Some(("assignment".to_string(), sym));
        }
    }

    None
}

fn find_enclosing_scope(node: &Node<'_, StrDoc<SupportLang>>) -> String {
    let mut current = node.parent();
    while let Some(p) = current {
        if let Some((kind, name)) = any_definition_node(&p) {
            // Only functional/callable scopes should count as enclosing caller scope
            if kind == "function" || kind.contains("fn") || kind.contains("method") {
                return name;
            }
        }
        current = p.parent();
    }
    "<top-level>".to_string()
}

/// Check if a node is an invoked callee expression (function being called)
pub(crate) fn is_callee_expression(node: &Node<'_, StrDoc<SupportLang>>, symbol: &str) -> bool {
    let mut curr = Some(node.clone());
    while let Some(c) = curr {
        if let Some(parent) = c.parent() {
            let pk = parent.kind();
            if pk == "call_expression" || pk == "method_call_expression" || pk == "call" {
                // If the parent is a call, check if c is the invoked function/method (field name 'function' or 'callee')
                if let Some(fn_node) = parent.field("function").or_else(|| parent.field("callee")) {
                    let fn_text = fn_node.text();
                    let extracted = extract_symbol_name(&fn_text);
                    if extracted == symbol {
                        return true;
                    }
                }
            }
            curr = Some(parent);
        } else {
            break;
        }
        // Limit upward traversal to avoid escaping statement/expression context
        if curr.as_ref().map_or(false, |n| n.kind().contains("statement")) {
            break;
        }
    }
    false
}

pub(crate) fn run(
    symbol: &str,
    path: &str,
    _depth: usize,
    callers_only: bool,
    callees_only: bool,
    json: bool,
    mermaid: bool,
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

        files_scanned += 1;

        if !content.contains(symbol) {
            continue;
        }
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
                                let raw_fn = fn_node.text().to_string();
                                let c_sym = extract_symbol_name(&raw_fn);
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

                    // Check if this identifier is invoked as a function/method call
                    if is_callee_expression(&node, symbol) {
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
    }

    if mermaid {
        println!("flowchart TD");
        let safe_symbol_id = symbol.replace(|c: char| !c.is_alphanumeric(), "_");
        println!("    subgraph Target[\"{}\"]", symbol);
        println!("        node_{}[\"{}\"]:::target", safe_symbol_id, symbol);
        println!("    end");

        if !callees_only && !callers.is_empty() {
            println!("    subgraph Callers[\"Inbound Callers\"]");
            for (idx, caller) in callers.iter().enumerate() {
                let caller_id = format!("c_{}_{}", idx, caller.caller_scope.replace(|c: char| !c.is_alphanumeric(), "_"));
                println!("        {}[\"{}\"] --> node_{}", caller_id, caller.caller_scope, safe_symbol_id);
            }
            println!("    end");
        }

        if !callers_only && !callees.is_empty() {
            println!("    subgraph Callees[\"Outbound Dependencies\"]");
            for (idx, callee) in callees.iter().enumerate() {
                let callee_id = format!("e_{}_{}", idx, callee.symbol.replace(|c: char| !c.is_alphanumeric(), "_"));
                println!("        node_{} --> {}[\"{}\"]", safe_symbol_id, callee_id, callee.symbol);
            }
            println!("    end");
        }

        println!("    classDef target fill:#4f46e5,stroke:#312e81,stroke-width:2px,color:#fff;");
        return 0;
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

    // Tab-indented TOON output with Unix colon locations
    println!("trace:");
    println!("\tsymbol: {}", symbol);
    println!("\tscanned: {}", files_scanned);
    
    let total_matches = definitions.len() + callers.len() + callees.len();
    if total_matches == 0 {
        println!("\tstatus: not_found (0 definitions, 0 callers, 0 callees found across {} scanned files)", files_scanned);
        return 1;
    }

    use std::collections::BTreeMap;

    if !callees_only {
        println!("\tdefinitions[{}]{{kind,loc,snippet}}:", definitions.len());
        let mut defs_by_file: BTreeMap<&str, Vec<&SymbolDef>> = BTreeMap::new();
        for def in &definitions {
            defs_by_file.entry(&def.path).or_default().push(def);
        }
        for (fpath, file_defs) in defs_by_file {
            println!("\t  {}:", fpath);
            for def in file_defs {
                println!("\t    {}:{}\t{}\t{}", def.line, def.col, def.kind, def.snippet);
            }
        }
    }

    if !callers_only && !callees_only {
        println!("\tcallers[{}]{{caller_scope,loc,snippet}}:", callers.len());
        let mut callers_by_file: BTreeMap<&str, Vec<&SymbolCaller>> = BTreeMap::new();
        for caller in &callers {
            callers_by_file.entry(&caller.path).or_default().push(caller);
        }
        for (fpath, file_callers) in callers_by_file {
            println!("\t  {}:", fpath);
            for caller in file_callers {
                println!("\t    {}:{}\t{}\t{}", caller.line, caller.col, caller.caller_scope, caller.snippet);
            }
        }
    }

    if !callers_only && !callees_only || callees_only {
        println!("\tcallees[{}]{{symbol,loc}}:", callees.len());
        let mut callees_by_file: BTreeMap<&str, Vec<&SymbolCallee>> = BTreeMap::new();
        for callee in &callees {
            callees_by_file.entry(&callee.path).or_default().push(callee);
        }
        for (fpath, file_callees) in callees_by_file {
            println!("\t  {}:", fpath);
            for callee in file_callees {
                println!("\t    {}:{}\t{}", callee.line, callee.col, callee.symbol);
            }
        }
    }

    0
}
