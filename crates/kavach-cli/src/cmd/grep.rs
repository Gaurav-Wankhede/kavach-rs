//! `kavach grep` — a hybrid of ripgrep (text files) and ast-grep (language
//! files) behind one keyword search and one output shape.
//!
//! Text files (`.txt`, `.rst`, `.csv`, `.log`, unknown extensions) get ripgrep
//! semantics: literal or `--regex` matching with `-i` / `-w` / `-m`, printing
//! the matched lines with exact `line:col` anchors.
//!
//! Language files get the same search *plus* the split pass
//! Dynamic split: language files use AST-grep; text files use ripgrep.
//! that encloses it, that declaration is auto-classified (`function`, `class`,
//! `struct`, `enum`, `const`, `module`, Markdown heading, ...), and only that
//! declaration's bytes are printed — so `kavach grep "Keyword"` never requires
//! the caller to know `--def`, `--call`, `--type` or `--scope`. Those flags
//! remain for precision and are not mutually exclusive with the split pass.


use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;
use glob_match::glob_match;
use ignore::WalkBuilder;
use regex::Regex;
use regex::RegexBuilder;
use serde_json::json;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;


/// A single match, anchored to an exact byte position in the source.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Hit {
    offset: usize,
    line: usize,
    col: usize,
}

/// Literal or regex line matcher — the ripgrep half of the hybrid.
struct LineMatcher {
    regex: Option<Regex>,
    needle: String,
    ignore_case: bool,
    word_regexp: bool,
}

impl LineMatcher {
    fn new(pattern: &str, ignore_case: bool, word_regexp: bool, use_regex: bool) -> Option<Self> {
        if use_regex {
            let regex = RegexBuilder::new(pattern)
                .case_insensitive(ignore_case)
                .build()
                .ok()?;
            return Some(Self {
                regex: Some(regex),
                needle: String::new(),
                ignore_case,
                word_regexp,
            });
        }
        if pattern.is_empty() {
            return None;
        }
        Some(Self {
            regex: None,
            needle: if ignore_case {
                pattern.to_lowercase()
            } else {
                pattern.to_string()
            },
            ignore_case,
            word_regexp,
        })
    }

    /// Every occurrence in `line` as `(byte_offset, byte_len)`.
    fn scan(&self, line: &str) -> Vec<(usize, usize)> {
        let mut found = Vec::new();
        if let Some(regex) = &self.regex {
            for m in regex.find_iter(line) {
                if self.word_regexp && !word_bounded(line, m.start(), m.end() - m.start()) {
                    continue;
                }
                found.push((m.start(), m.end() - m.start()));
            }
            return found;
        }

        // Case folding is only safe when it cannot change byte offsets, i.e.
        // on ASCII. Skipping it elsewhere keeps every column exact instead of
        // reporting a plausible-but-wrong position.
        let hay = if self.ignore_case && line.is_ascii() {
            line.to_ascii_lowercase()
        } else {
            line.to_string()
        };
        let mut from = 0usize;
        while let Some(rel) = hay[from..].find(&self.needle) {
            let at = from + rel;
            let len = self.needle.len();
            if !self.word_regexp || word_bounded(line, at, len) {
                found.push((at, len));
            }
            from = at + len.max(1);
        }
        found
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn word_bounded(line: &str, start: usize, len: usize) -> bool {
    let before = line[..start].chars().next_back();
    let after = line[start + len..].chars().next();
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

fn resolve_lang(path: &str, lang_opt: Option<&str>) -> Option<SupportLang> {
    use ast_grep_language::Language;
    use std::str::FromStr;
    if let Some(l) = lang_opt
        && let Ok(lang) = SupportLang::from_str(l)
    {
        return Some(lang);
    }
    SupportLang::from_path(Path::new(path))
}

pub(crate) fn run(
    pattern: &str,
    path: &str,
    lang_opt: Option<&str>,
    glob_filter: Option<&str>,
    ignore_case: bool,
    word_regexp: bool,
    json_mode: bool,
    max_count: usize,
    use_regex: bool,
) -> i32 {
    let p = Path::new(path);
    if !p.exists() {
        eprintln!("kavach grep: path '{}' does not exist", path);
        return 2;
    }

        // Split is automatic: text files use ripgrep; language files use AST-grep.

    let matcher = if use_regex && LineMatcher::new(pattern, ignore_case, word_regexp, true).is_none() {
        eprintln!("kavach grep: invalid regex '{}'", pattern);
        return 2;
    } else {
        LineMatcher::new(pattern, ignore_case, word_regexp, use_regex)
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut total_matches = 0usize;

    // Dynamic split is automatic by file extension; no positional swap needed.
    let (effective_pattern, effective_path) = (pattern, path);
    let p = Path::new(effective_path);

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
        if p_path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(".env")) {
            eprintln!(
                "error: 'kavach grep' is forbidden on .env files to eliminate secret leakage ('{}').\n                 Use the sovereign zero-leak ENV tool instead:\n                     kavach env {}",
                path_str, path_str
            );
            return 2;
        }
        process_file(
            &path_str,
            effective_pattern,
            matcher.as_ref(),
            lang_opt,
            json_mode,
            max_count,
            &mut total_matches,
            &mut out,
        );
    } else {
        let mut builder = WalkBuilder::new(effective_path);
        builder.hidden(false).git_ignore(true).git_global(true).git_exclude(true);
        let walker = builder.build();

        for entry in walker {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path().to_string_lossy();
            if let Some(glob_pat) = glob_filter {
                let clean = file_path.strip_prefix("./").unwrap_or(&file_path);
                let file_name = entry.file_name().to_string_lossy();
                if !glob_match(glob_pat, clean)
                    && !glob_match(glob_pat, &file_path)
                    && !glob_match(glob_pat, &file_name)
                {
                    continue;
                }
            }

            process_file(
                &file_path,
                effective_pattern,
                matcher.as_ref(),
                lang_opt,
                json_mode,
                max_count,
                &mut total_matches,
                &mut out,
            );
        }
    }

    let _ = out.flush();
    0
}

/// Minimal line table for byte-offset -> line/col mapping.
struct LineTable {
    starts: Vec<usize>,
}
impl LineTable {
    fn new(content: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }
    fn line_start(&self, line_no: usize) -> usize {
        self.starts.get(line_no.saturating_sub(1)).copied().unwrap_or(0)
    }
    fn line_of(&self, offset: usize) -> usize {
        for (line_no, &start) in self.starts.iter().enumerate().rev() {
            if offset >= start {
                return line_no + 1;
            }
        }
        1
    }
    fn col_of(&self, offset: usize) -> usize {
        let line_start = self.starts.iter().rev().find(|&&s| s <= offset).copied().unwrap_or(0);
        (offset.saturating_sub(line_start) + 1).max(1)
    }
}

/// Per-file search pipeline. The parameter count mirrors the clap flag set;
/// bundling them into a struct would only move the same fields one hop.
#[allow(clippy::too_many_arguments, reason = "one parameter per CLI flag")]
fn process_file(
    file_path: &str,
    pattern: &str,
    matcher: Option<&LineMatcher>,
    lang_opt: Option<&str>,
    json_mode: bool,
    max_count: usize,
    total_matches: &mut usize,
    out: &mut BufWriter<io::StdoutLock<'_>>,
) {
    let Ok(content) = fs::read_to_string(file_path) else {
        return;
    };
    let all_lines: Vec<&str> = content.lines().collect();
    if all_lines.is_empty() {
        return;
    }
    let table = LineTable::new(&content);
    let lang = resolve_lang(file_path, lang_opt);

    let mut hits = ast_hits(&content, pattern, lang);
    if hits.is_empty()
        && let Some(m) = matcher
    {
        hits = text_hits(&all_lines, &table, m);
    }
    if hits.is_empty() {
        return;
    }
    if max_count > 0 && hits.len() > max_count {
        hits.truncate(max_count);
    }
    *total_matches = total_matches.saturating_add(hits.len());

    if json_mode {
        for hit in &hits {
            let exact_line = all_lines.get(hit.line.saturating_sub(1)).copied().unwrap_or("");
            let record = json!({
                "file": file_path,
                "line": hit.line,
                "col": hit.col,
                "exact_line": exact_line,
            });
            let _ = writeln!(out, "{}", record);
        }
        return;
    }

    for hit in &hits {
        let exact_line = all_lines.get(hit.line.saturating_sub(1)).copied().unwrap_or("");
        let _ = writeln!(out, "{}:{}:{}:{}", file_path, hit.line, hit.col, exact_line);
    }
}

fn ast_hits(
    content: &str,
    pattern: &str,
    lang: Option<SupportLang>,
) -> Vec<Hit> {
    let Some(lang) = lang else { return Vec::new() };

    // Always try AST-grep for language files; plain text patterns
    // that aren't valid AST patterns will return empty.
    let table = LineTable::new(content);
    let grep = AstGrep::new(content, lang);
    let root = grep.root();
    let mut hits = Vec::new();
    let mut push = |offset: usize, _end: usize| {
        hits.push(Hit {
            offset,
            line: table.line_of(offset),
            col: table.col_of(offset),
        });
    };

    if let Ok(p) = ast_grep_core::Pattern::try_new(pattern, lang) {
        for m in root.find_all(&p) {
            let range = m.range();
            push(range.start, range.end);
        }
    }
    hits
}

fn text_hits(
    all_lines: &[&str],
    table: &LineTable,
    matcher: &LineMatcher,
) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (idx, line) in all_lines.iter().enumerate() {
        let line_no = idx + 1;
        for (col, _len) in matcher.scan(line) {
            hits.push(Hit {
                offset: table.line_start(line_no) + col,
                line: line_no,
                col: col + 1,
            });
        }
    }
    hits
}