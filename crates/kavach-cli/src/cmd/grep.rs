fn is_executable_code_file(path_str: &str, lang_opt: Option<&str>) -> bool {
    let p = Path::new(path_str);
    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
        let e = ext.to_lowercase();
        // Explicitly exclude data, config, markup and prose
        if matches!(e.as_str(), "json" | "yaml" | "yml" | "toml" | "xml" | "md" | "txt" | "markdown" | "rst" | "adoc" | "log" | "csv" | "tsv") {
            return false;
        }
        // Known programming languages
        if matches!(e.as_str(), "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "cs" | "java" | "sh" | "bash" | "swift" | "kt" | "kts" | "lua" | "rb" | "php" | "sol" | "ex" | "exs" | "dart" | "scala" | "hs") {
            return true;
        }
    }
    if let Some(l) = lang_opt {
        let l_low = l.to_lowercase();
        if !matches!(l_low.as_str(), "json" | "yaml" | "yml" | "toml" | "markdown" | "text") {
            return resolve_lang(path_str, Some(l)).is_some();
        }
    }
    false
}

use ast_grep_core::{AstGrep, Node};
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;
use glob_match::glob_match;
use ignore::WalkBuilder;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub(crate) fn run(
    pattern: &str,
    path: &str,
    def_opt: Option<&str>,
    call_opt: Option<&str>,
    type_opt: Option<&str>,
    is_ast: bool,
    lang_opt: Option<&str>,
    glob_filter: Option<&str>,
    ignore_case: bool,
    word_regexp: bool,
    scope: bool,
    json_mode: bool,
    _max_count: usize,
) -> i32 {
    let p = Path::new(path);
    if !p.exists() {
        eprintln!("kavach grep: path '{}' does not exist", path);
        return 2;
    }
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut total_matches = 0usize;

    let has_ast_intent = def_opt.is_some() || call_opt.is_some() || type_opt.is_some() || pattern.contains("$");

    if p.is_file() {
        let path_str = p.to_string_lossy();
        let p_path = Path::new(&*path_str);
        if let Some(ext) = p_path.extension().and_then(|e| e.to_str()) {
            let e = ext.to_lowercase();
            match e.as_str() {
                "json" | "jsonl" | "jsonc" => {
                    eprintln!(
                        "error: 'kavach grep' does not operate on JSON files ('{}').\n                         Use the sovereign JSON tool instead:\n                             kavach json \"<query>\" {}",
                        path_str, path_str
                    );
                    return 2;
                }
                "yaml" | "yml" => {
                    eprintln!(
                        "error: 'kavach grep' does not operate on YAML files ('{}').\n                         Use the sovereign YAML tool instead:\n                             kavach yaml \"<query>\" {}",
                        path_str, path_str
                    );
                    return 2;
                }
                "toml" => {
                    eprintln!(
                        "error: 'kavach grep' does not operate on TOML files ('{}').\n                         Use the sovereign TOML tool instead:\n                             kavach toml \"<query>\" {}",
                        path_str, path_str
                    );
                    return 2;
                }
                "html" | "htm" => {
                    eprintln!(
                        "error: 'kavach grep' does not operate on HTML files ('{}').\n                         Use the sovereign HTML tool instead:\n                             kavach html \"<selector>\" {}",
                        path_str, path_str
                    );
                    return 2;
                }
                "svg" => {
                    eprintln!(
                        "error: 'kavach grep' does not operate on SVG vector files ('{}').\n                         Use the sovereign SVG tool instead:\n                             kavach svg {}",
                        path_str, path_str
                    );
                    return 2;
                }
                "env" => {
                    eprintln!(
                        "error: 'kavach grep' is forbidden on .env files to eliminate secret leakage ('{}').\n                         Use the sovereign zero-leak ENV tool instead:\n                             kavach env {}",
                        path_str, path_str
                    );
                    return 2;
                }
                _ => {}
            }
        }
        if p_path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with(".env")) {
            eprintln!(
                "error: 'kavach grep' is forbidden on .env files to eliminate secret leakage ('{}').\n                 Use the sovereign zero-leak ENV tool instead:\n                     kavach env {}",
                path_str, path_str
            );
            return 2;
        }
        if is_executable_code_file(&path_str, lang_opt) && !has_ast_intent {
            eprintln!(
                "error: arbitrary text pattern search is forbidden in code files ('{}').
                 Code files require AST semantic flags:
                     kavach grep --def <symbol> [path]
                     kavach grep --call <symbol> [path]
                     kavach grep --type <symbol> [path]
                 For full declaration extraction, add --scope.
                 Arbitrary text patterns are strictly reserved for documentation and prose (.md, .txt).",
                path_str
            );
            return 2;
        }
        let lang = resolve_lang(&path_str, lang_opt);
        process_file(
            &path_str,
            pattern,
            def_opt,
            call_opt,
            type_opt,
            is_ast,
            lang,
            ignore_case,
            word_regexp,
            scope,
            json_mode,
            &mut total_matches,
            &mut out,
        );
    } else {
        let mut builder = WalkBuilder::new(path);
        builder.hidden(false).git_ignore(true).git_global(true).git_exclude(true);
        let walker = builder.build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path().to_string_lossy();
            if let Some(glob_pat) = glob_filter {
                let clean_file_path = file_path.strip_prefix("./").unwrap_or(&file_path);
                let file_name = entry.file_name().to_string_lossy();
                if !glob_match(glob_pat, clean_file_path) && !glob_match(glob_pat, &file_path) && !glob_match(glob_pat, &file_name) {
                    continue;
                }
            }

            let lang = resolve_lang(&file_path, lang_opt);
            process_file(
                &file_path,
                pattern,
                def_opt,
                call_opt,
                type_opt,
                is_ast,
                lang,
                ignore_case,
                word_regexp,
                scope,
                json_mode,
                &mut total_matches,
                &mut out,
            );
        }
    }

    let _ = out.flush();
    0
}

fn resolve_lang(path: &str, lang_opt: Option<&str>) -> Option<SupportLang> {
    use ast_grep_language::Language;
    if let Some(l) = lang_opt {
        match l.to_lowercase().as_str() {
            "rs" | "rust" => return Some(SupportLang::Rust),
            "ts" | "typescript" => return Some(SupportLang::TypeScript),
            "tsx" => return Some(SupportLang::Tsx),
            "js" | "javascript" => return Some(SupportLang::JavaScript),
            "py" | "python" => return Some(SupportLang::Python),
            "go" => return Some(SupportLang::Go),
            "c" => return Some(SupportLang::C),
            "cpp" | "c++" => return Some(SupportLang::Cpp),
            "cs" | "csharp" => return Some(SupportLang::CSharp),
            "java" => return Some(SupportLang::Java),
            "sh" | "bash" => return Some(SupportLang::Bash),
            "json" => return Some(SupportLang::Json),
            "yaml" | "yml" => return Some(SupportLang::Yaml),
            "html" => return Some(SupportLang::Html),
            "css" => return Some(SupportLang::Css),
            "swift" => return Some(SupportLang::Swift),
            "kotlin" => return Some(SupportLang::Kotlin),
            "lua" => return Some(SupportLang::Lua),
            "ruby" | "rb" => return Some(SupportLang::Ruby),
            "php" => return Some(SupportLang::Php),
            "solidity" | "sol" => return Some(SupportLang::Solidity),
            "elixir" | "ex" => return Some(SupportLang::Elixir),
            "dart" => return Some(SupportLang::Dart),
            "scala" => return Some(SupportLang::Scala),
            "haskell" | "hs" => return Some(SupportLang::Haskell),
            _ => return None,
        }
    }
    SupportLang::from_path(Path::new(path))
}

fn offset_to_line_and_col(content: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, b) in content.bytes().enumerate() {
        if idx >= byte_offset {
            break;
        }
        if b == b"\n"[0] {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn find_enclosing_declaration(node: &Node<'_, StrDoc<SupportLang>>) -> Option<(usize, usize)> {
    let mut current = Some(node.clone());
    while let Some(curr) = current {
        let kind = curr.kind();
        if matches!(
            kind.as_ref(),
            "function_item"
                | "struct_item"
                | "enum_item"
                | "impl_item"
                | "trait_item"
                | "type_item"
                | "const_item"
                | "static_item"
                | "function_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "type_alias_declaration"
                | "method_definition"
                | "function_definition"
                | "class_definition"
        ) {
            let range = curr.range();
            return Some((range.start, range.end));
        }
        current = curr.parent();
    }
    None
}

fn matches_word_boundary(line: &str, start_col: usize, match_len: usize) -> bool {
    let before = if start_col > 1 {
        line.chars().nth(start_col - 2)
    } else {
        None
    };
    if let Some(c) = before {
        if is_word_char(c) {
            return false;
        }
    }

    let after_idx = start_col - 1 + match_len;
    let after = line.chars().nth(after_idx);
    if let Some(c) = after {
        if is_word_char(c) {
            return false;
        }
    }

    true
}

fn process_file(
    file_path: &str,
    pattern: &str,
    def_opt: Option<&str>,
    call_opt: Option<&str>,
    type_opt: Option<&str>,
    is_ast: bool,
    lang_opt: Option<SupportLang>,
    ignore_case: bool,
    word_regexp: bool,
    scope: bool,
    json_mode: bool,
    total_matches: &mut usize,
    out: &mut BufWriter<io::StdoutLock<'_>>,
) {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let all_lines: Vec<&str> = content.lines().collect();
    if all_lines.is_empty() {
        return;
    }

    // Polyglot AST search: check --def, --call, --type, or AST pattern
    let (ast_matches, scope_intervals): (Vec<(usize, usize, usize, usize)>, Vec<(usize, usize)>) = if lang_opt.is_some() && (def_opt.is_some() || call_opt.is_some() || type_opt.is_some() || (is_ast && !pattern.is_empty())) {
        let lang = lang_opt.unwrap();
        let grep = AstGrep::new(&content, lang);
        let root = grep.root();
        let mut matches = Vec::new();
        let mut scopes = Vec::new();

        if let Some(def_sym) = def_opt {
            for node in root.dfs() {
                let k = node.kind();
                if k.contains("declaration") || k.contains("definition") || k.ends_with("_item") || k.contains("type_spec") {
                    if let Some(name_node) = node.field("name").or_else(|| node.field("identifier")) {
                        if def_sym == "$NAME" || def_sym == "*" || name_node.text() == def_sym {
                            let range = name_node.range();
                            let (s_line, s_col) = offset_to_line_and_col(&content, range.start);
                            let (e_line, e_col) = offset_to_line_and_col(&content, range.end);
                            matches.push((s_line, s_col, e_line, e_col));
                            if scope {
                                let decl_range = node.range();
                                let (sc_s_line, _) = offset_to_line_and_col(&content, decl_range.start);
                                let (sc_e_line, _) = offset_to_line_and_col(&content, decl_range.end);
                                scopes.push((sc_s_line, sc_e_line));
                            }
                        }
                    }
                }
            }
        } else if let Some(call_sym) = call_opt {
            for node in root.dfs() {
                let k = node.kind();
                if k == "call_expression" || k == "method_call_expression" || k == "call" {
                    if let Some(fn_node) = node.field("function") {
                        let fn_txt = fn_node.text();
                        if call_sym == "$NAME" || call_sym == "*" || fn_txt == call_sym || fn_txt.ends_with(&format!(".{}", call_sym)) || fn_txt.ends_with(&format!("::{}", call_sym)) {
                            let range = fn_node.range();
                            let (s_line, s_col) = offset_to_line_and_col(&content, range.start);
                            let (e_line, e_col) = offset_to_line_and_col(&content, range.end);
                            matches.push((s_line, s_col, e_line, e_col));
                            if scope {
                                if let Some((scope_start, scope_end)) = find_enclosing_declaration(&node) {
                                    let (sc_s_line, _) = offset_to_line_and_col(&content, scope_start);
                                    let (sc_e_line, _) = offset_to_line_and_col(&content, scope_end);
                                    scopes.push((sc_s_line, sc_e_line));
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(type_sym) = type_opt {
            for node in root.dfs() {
                let k = node.kind();
                if k.contains("struct") || k.contains("class") || k.contains("interface") || k.contains("enum") || k.contains("type") {
                    if let Some(name_node) = node.field("name").or_else(|| node.field("identifier")) {
                        if type_sym == "$NAME" || type_sym == "*" || name_node.text() == type_sym {
                            let range = name_node.range();
                            let (s_line, s_col) = offset_to_line_and_col(&content, range.start);
                            let (e_line, e_col) = offset_to_line_and_col(&content, range.end);
                            matches.push((s_line, s_col, e_line, e_col));
                            if scope {
                                let decl_range = node.range();
                                let (sc_s_line, _) = offset_to_line_and_col(&content, decl_range.start);
                                let (sc_e_line, _) = offset_to_line_and_col(&content, decl_range.end);
                                scopes.push((sc_s_line, sc_e_line));
                            }
                        }
                    }
                }
            }
        } else if is_ast && !pattern.is_empty() {
            if let Ok(p) = ast_grep_core::Pattern::try_new(pattern, lang) {
                for m in root.find_all(&p) {
                    let range = m.range();
                    let (s_line, s_col) = offset_to_line_and_col(&content, range.start);
                    let (e_line, e_col) = offset_to_line_and_col(&content, range.end);
                    matches.push((s_line, s_col, e_line, e_col));
                    if scope {
                        if let Some((scope_start, scope_end)) = find_enclosing_declaration(&m) {
                            let (sc_s_line, _) = offset_to_line_and_col(&content, scope_start);
                            let (sc_e_line, _) = offset_to_line_and_col(&content, scope_end);
                            scopes.push((sc_s_line, sc_e_line));
                        }
                    }
                }
            }
        }
        (matches, scopes)
    } else {
        (Vec::new(), Vec::new())
    };

    let spans: Vec<(usize, usize, usize, usize)> = if !ast_matches.is_empty() || def_opt.is_some() || call_opt.is_some() || type_opt.is_some() || (pattern.contains("$") && lang_opt.is_some()) {
        ast_matches
    } else if is_executable_code_file(file_path, None) {
        // Strict boundary: code files are never searched via arbitrary literal text fallback!
        Vec::new()
    } else {
        let mut literal_spans = Vec::new();
        let target_pat = if ignore_case { pattern.to_lowercase() } else { pattern.to_string() };

        for (idx, line) in all_lines.iter().enumerate() {
            let search_line = if ignore_case { line.to_lowercase() } else { line.to_string() };
            let mut search_start = 0;
            while let Some(rel_col) = search_line[search_start..].find(&target_pat) {
                let col_idx = search_start + rel_col;
                let start_line = idx + 1;
                let start_col = col_idx + 1;
                let end_line = start_line;
                let end_col = start_col + pattern.len();

                if !word_regexp || matches_word_boundary(line, start_col, pattern.len()) {
                    literal_spans.push((start_line, start_col, end_line, end_col));
                }
                search_start = col_idx + pattern.len().max(1);
            }
        }
        literal_spans
    };

    if spans.is_empty() {
        return;
    }

    *total_matches = total_matches.saturating_add(spans.len());

    if json_mode {
        for &(start_line, start_col, end_line, end_col) in &spans {
            let line_idx = start_line.saturating_sub(1);
            let exact_raw_line = all_lines.get(line_idx).copied().unwrap_or("");
            let record = json!({
                "file": file_path,
                "line": start_line,
                "col": start_col,
                "end_line": end_line,
                "end_col": end_col,
                "exact_line": exact_raw_line,
            });
            let _ = writeln!(out, "{}", record);
        }
        return;
    }

    // Grouped header format
    let _ = writeln!(out, "{}", file_path);

    let mut matched_lines = BTreeSet::new();
    let mut match_first_col = std::collections::BTreeMap::new();
    for &(s_line, s_col, _, _) in &spans {
        matched_lines.insert(s_line);
        match_first_col.entry(s_line).or_insert(s_col);
    }

    if scope && !scope_intervals.is_empty() {
        let mut intervals: Vec<(usize, usize)> = Vec::new();
        for &(sc_start, sc_end) in &scope_intervals {
            let start = sc_start.max(1);
            let end = sc_end.min(all_lines.len());
            if let Some(last) = intervals.last_mut() {
                if start <= last.1 + 1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            intervals.push((start, end));
        }
        for (int_idx, &(start_line, end_line)) in intervals.iter().enumerate() {
            if int_idx > 0 {
                let _ = writeln!(out, "--");
            }
            for l_num in start_line..=end_line {
                let l_idx = l_num - 1;
                let line_content = all_lines.get(l_idx).copied().unwrap_or("");
                if matched_lines.contains(&l_num) {
                    let col = match_first_col.get(&l_num).copied().unwrap_or(1);
                    let _ = writeln!(out, "{}:{}:{}", l_num, col, line_content);
                } else {
                    let _ = writeln!(out, "{}-{}", l_num, line_content);
                }
            }
        }
    } else {
        // Precise file:line,col with exact line and indentation (zero range spam)
        for &m_line in &matched_lines {
            let l_idx = m_line - 1;
            let line_content = all_lines.get(l_idx).copied().unwrap_or("");
            let col = match_first_col.get(&m_line).copied().unwrap_or(1);
            let _ = writeln!(out, "{}:{}:{}", m_line, col, line_content);
        }
    }
    let _ = writeln!(out);
}
