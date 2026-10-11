use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;
use glob_match::glob_match;
use ignore::WalkBuilder;
use serde_json::json;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub(crate) fn run(
    pattern: &str,
    path: &str,
    is_ast: bool,
    lang_opt: Option<&str>,
    glob_filter: Option<&str>,
    json_mode: bool,
    max_count: usize,
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
        if let Some(l) = lang {
            process_file(
                &path_str,
                pattern,
                is_ast,
                l,
                json_mode,
                context_lines,
                &mut total_matches,
                max_count,
                &mut out,
            );
        }
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

            let lang = match resolve_lang(&file_path, lang_opt) {
                Some(l) => l,
                None => continue,
            };

            process_file(
                &file_path,
                pattern,
                is_ast,
                lang,
                json_mode,
                context_lines,
                &mut total_matches,
                max_count,
                &mut out,
            );
            if total_matches >= max_count {
                break;
            }
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

fn process_file(
    file_path: &str,
    pattern: &str,
    _is_ast: bool,
    lang: SupportLang,
    json_mode: bool,
    context_lines: usize,
    total_matches: &mut usize,
    max_count: usize,
    out: &mut BufWriter<io::StdoutLock<'_>>,
) {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let all_lines: Vec<&str> = content.lines().collect();

    let ast_matches: Vec<(usize, usize, usize, usize)> = match ast_grep_core::Pattern::try_new(pattern, lang) {
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
    };

    let spans: Vec<(usize, usize, usize, usize)> = if !ast_matches.is_empty() || pattern.contains("$") {
        ast_matches
    } else {
        let mut literal_spans = Vec::new();
        for (idx, line) in all_lines.iter().enumerate() {
            if let Some(col_idx) = line.find(pattern) {
                let start_line = idx + 1;
                let start_col = col_idx + 1;
                let end_line = start_line;
                let end_col = start_col + pattern.len();
                literal_spans.push((start_line, start_col, end_line, end_col));
            }
        }
        literal_spans
    };

    for &(start_line, start_col, end_line, end_col) in &spans {
        let line_idx = start_line.saturating_sub(1);
        let exact_raw_line = all_lines.get(line_idx).copied().unwrap_or("");

        if json_mode {
            let record = json!({
                "file": file_path,
                "line": start_line,
                "col": start_col,
                "end_line": end_line,
                "end_col": end_col,
                "exact_line": exact_raw_line,
            });
            let _ = writeln!(out, "{}", record);
        } else {
            if context_lines > 0 {
                let start_ctx = line_idx.saturating_sub(context_lines);
                for i in start_ctx..line_idx {
                    if let Some(ctx_line) = all_lines.get(i) {
                        let _ = writeln!(out, "{}-{}-{}", file_path, i + 1, ctx_line);
                    }
                }
            }

            let _ = writeln!(out, "{}:{}:{}:{}", file_path, start_line, start_col, exact_raw_line);

            if context_lines > 0 {
                let end_ctx = (line_idx + 1 + context_lines).min(all_lines.len());
                for i in (line_idx + 1)..end_ctx {
                    if let Some(ctx_line) = all_lines.get(i) {
                        let _ = writeln!(out, "{}-{}-{}", file_path, i + 1, ctx_line);
                    }
                }
            }
        }

        *total_matches = total_matches.saturating_add(1);
        if *total_matches >= max_count {
            break;
        }
    }
}
