use ast_grep_core::AstGrep;
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
    is_ast: bool,
    lang_opt: Option<&str>,
    glob_filter: Option<&str>,
    ignore_case: bool,
    word_regexp: bool,
    json_mode: bool,
    _max_count: usize,
    context_lines: usize,
) -> i32 {
    let p = Path::new(path);
    if !p.exists() {
        eprintln!("kavach grep: path '{}' does not exist", path);
        return 2;
    }
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut total_matches = 0usize;

    if p.is_file() {
        let path_str = p.to_string_lossy();
        let lang = resolve_lang(&path_str, lang_opt);
        process_file(
            &path_str,
            pattern,
            is_ast,
            lang,
            ignore_case,
            word_regexp,
            json_mode,
            context_lines,
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
                let file_name = entry.file_name().to_string_lossy();
                if !glob_match(glob_pat, &file_name) {
                    continue;
                }
            }

            let lang = resolve_lang(&file_path, lang_opt);
            process_file(
                &file_path,
                pattern,
                is_ast,
                lang,
                ignore_case,
                word_regexp,
                json_mode,
                context_lines,
                &mut total_matches,
                &mut out,
            );
        }
    }

    let _ = out.flush();
    0
}

fn resolve_lang(path: &str, lang_opt: Option<&str>) -> Option<SupportLang> {
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
            _ => return None,
        }
    }

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
        "sh" | "bash" => Some(SupportLang::Bash),
        "json" => Some(SupportLang::Json),
        "yaml" | "yml" => Some(SupportLang::Yaml),
        "html" => Some(SupportLang::Html),
        "css" => Some(SupportLang::Css),
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
    _is_ast: bool,
    lang_opt: Option<SupportLang>,
    ignore_case: bool,
    word_regexp: bool,
    json_mode: bool,
    context_lines: usize,
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

    // Try AST search if grammar language is recognized
    let ast_matches: Vec<(usize, usize, usize, usize)> = if let Some(lang) = lang_opt {
        match ast_grep_core::Pattern::try_new(pattern, lang) {
            Ok(p) => {
                let grep = AstGrep::new(&content, lang);
                let root = grep.root();
                root.find_all(&p)
                    .map(|m| {
                        let range = m.range();
                        let (s_line, s_col) = offset_to_line_and_col(&content, range.start);
                        let (e_line, e_col) = offset_to_line_and_col(&content, range.end);
                        (s_line, s_col, e_line, e_col)
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let spans: Vec<(usize, usize, usize, usize)> = if !ast_matches.is_empty() || (pattern.contains("$") && lang_opt.is_some()) {
        ast_matches
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

    let mut intervals: Vec<(usize, usize)> = Vec::new();
    for &m_line in &matched_lines {
        let start = m_line.saturating_sub(context_lines).max(1);
        let end = (m_line + context_lines).min(all_lines.len());
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
    let _ = writeln!(out);
}
