//! Surgical in-place file text/regex replacement using `aho-corasick` / `regex` and `tempfile`.
//! Atomic replacement with zero risk of partial file corruptions.

use aho_corasick::AhoCorasick;
use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::Path;

pub(crate) fn run(
    find: &str,
    replace: &str,
    path: &str,
    fixed_strings: bool,
    use_regex: bool,
    dry_run: bool,
) -> i32 {
    let target = Path::new(path);
    if !target.exists() {
        eprintln!("[ERROR] Target file does not exist: {}", path);
        return 1;
    }

    let original = match fs::read_to_string(target) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Failed to read {}: {}", path, e);
            return 1;
        }
    };

    let (modified, count) = if use_regex && !fixed_strings {
        let re = match Regex::new(find) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[ERROR] Invalid regex {}: {}", find, e);
                return 1;
            }
        };
        let count = re.find_iter(&original).count();
        let mod_str = re.replace_all(&original, replace).to_string();
        (mod_str, count)
    } else {
        let ac = match AhoCorasick::new([find]) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[ERROR] Failed to build matcher: {}", e);
                return 1;
            }
        };
        let count = ac.find_iter(&original).count();
        let mod_str = ac.replace_all(&original, &[replace]);
        (mod_str, count)
    };

    if count == 0 {
        eprintln!("[REPLACE] No occurrences of pattern found in {}", path);
        return 1;
    }

    // High quality location & indentation telemetry
    let occurrences = find_occurrences(&original, find, use_regex && !fixed_strings);

    if dry_run {
        println!("[DRY RUN] Found {} occurrence(s) in {}:", count, path);
        for occ in occurrences.iter().take(20) {
            println!("  -> Line {}:{} (indent: {})", occ.line, occ.col, occ.indent_desc);
            println!("     {}", occ.line_preview);
        }
        return 0;
    }

    // Atomic write via tempfile in the same directory
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = match tempfile::NamedTempFile::new_in(parent) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[ERROR] Failed to create tempfile: {}", e);
            return 1;
        }
    };

    if let Err(e) = temp.write_all(modified.as_bytes()) {
        eprintln!("[ERROR] Failed to write tempfile: {}", e);
        return 1;
    }
    if let Err(e) = temp.flush() {
        eprintln!("[ERROR] Failed to flush tempfile: {}", e);
        return 1;
    }

    if let Err(e) = temp.persist(target) {
        eprintln!("[ERROR] Failed to atomic persist to {}: {}", path, e);
        return 1;
    }

    println!("[REPLACE] Successfully replaced {} occurrence(s) in {}:", count, path);
    for occ in occurrences.iter().take(20) {
        println!("  -> Line {}:{} (indent: {})", occ.line, occ.col, occ.indent_desc);
        println!("     - {}", occ.line_preview);
    }
    0
}

struct Occurrence {
    line: usize,
    col: usize,
    indent_desc: String,
    line_preview: String,
}

fn find_occurrences(source: &str, find: &str, is_regex: bool) -> Vec<Occurrence> {
    let mut results = Vec::new();

    if is_regex {
        if let Ok(re) = Regex::new(find) {
            for m in re.find_iter(source) {
                results.push(compute_telemetry(source, m.start()));
            }
        }
    } else if let Ok(ac) = AhoCorasick::new([find]) {
        for m in ac.find_iter(source) {
            results.push(compute_telemetry(source, m.start()));
        }
    }

    results
}

fn compute_telemetry(source: &str, byte_offset: usize) -> Occurrence {
    let prefix = &source[..byte_offset];
    let line_number = prefix.lines().count().max(1);
    
    let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_end = source[byte_offset..].find('\n').map(|idx| byte_offset + idx).unwrap_or(source.len());
    let full_line = &source[line_start..line_end];

    let col_number = byte_offset.saturating_sub(line_start) + 1;

    // Detect indentation
    let indent_chars: String = full_line.chars().take_while(|c| c.is_whitespace()).collect();
    let indent_desc = if indent_chars.is_empty() {
        "0 (flush left)".to_string()
    } else {
        let spaces = indent_chars.chars().filter(|&c| c == ' ').count();
        let tabs = indent_chars.chars().filter(|&c| c == '\t').count();
        if tabs > 0 && spaces > 0 {
            format!("{} tabs, {} spaces (mixed)", tabs, spaces)
        } else if tabs > 0 {
            format!("{} tab{}", tabs, if tabs == 1 { "" } else { "s" })
        } else {
            format!("{} spaces", spaces)
        }
    };

    Occurrence {
        line: line_number,
        col: col_number,
        indent_desc,
        line_preview: full_line.trim_end().to_string(),
    }
}
