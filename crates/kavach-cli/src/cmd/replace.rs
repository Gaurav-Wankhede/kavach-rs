//! Surgical in-place file text/regex replacement using `aho-corasick` / `regex` and `tempfile`.
//! Atomic replacement with zero risk of partial file corruptions.
//! Supports:
//!   1. Positional legacy format: `kavach replace <FIND> <REPLACE> <PATH>`
//!   2. Single or multi-pair format: `kavach replace --pair <FIND> <REPLACE> <PATH>`
//!   3. Inverted path format: `kavach replace <PATH> --pair <FIND> <REPLACE>`

use aho_corasick::{AhoCorasick, MatchKind};
use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug)]
struct SubPair {
    find: String,
    replace: String,
}

pub(crate) fn run(
    arg1: Option<String>,
    arg2: Option<String>,
    arg3: Option<String>,
    pairs_raw: Vec<String>,
    fixed_strings: bool,
    use_regex: bool,
    dry_run: bool,
) -> i32 {
    let mut pairs: Vec<SubPair> = Vec::new();

    // Parse repeated --pair <FIND> <REPLACE>
    let mut i = 0;
    while i + 1 < pairs_raw.len() {
        pairs.push(SubPair {
            find: pairs_raw[i].clone(),
            replace: pairs_raw[i + 1].clone(),
        });
        i += 2;
    }

    // Determine target path and any positional pairs
    let target_path = if !pairs.is_empty() {
        // When using --pair:
        // Either arg1 or arg3 is the file path
        if let Some(p) = arg1 {
            p
        } else if let Some(p) = arg3 {
            p
        } else {
            eprintln!("[ERROR] No target file path specified. Usage: `kavach replace --pair <FIND> <REPLACE> <PATH>`");
            return 2;
        }
    } else {
        // When NOT using --pair:
        // Must have arg1 (<FIND>), arg2 (<REPLACE>), arg3 (<PATH>)
        match (arg1, arg2, arg3) {
            (Some(f), Some(r), Some(p)) => {
                pairs.push(SubPair { find: f, replace: r });
                p
            }
            (Some(_p), Some(_f), None) => {
                // If user wrote `kavach replace <PATH> <FIND>` by mistake
                eprintln!("[ERROR] Missing replacement argument. Usage: `kavach replace <FIND> <REPLACE> <PATH>`");
                return 2;
            }
            _ => {
                eprintln!("[ERROR] Invalid arguments. Usage: `kavach replace <FIND> <REPLACE> <PATH>` or `kavach replace --pair <FIND> <REPLACE> <PATH>`");
                return 2;
            }
        }
    };

    let target = Path::new(&target_path);
    if !target.exists() {
        eprintln!("[ERROR] Target file does not exist: {}", target_path);
        return 1;
    }

    let original = match fs::read_to_string(target) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Failed to read {}: {}", target_path, e);
            return 1;
        }
    };

    let (modified, count, occurrences) = if use_regex && !fixed_strings {
        let mut count = 0;
        let mut curr = original.clone();
        let mut occs = Vec::new();

        for p in &pairs {
            let re = match Regex::new(&p.find) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[ERROR] Invalid regex {}: {}", p.find, e);
                    return 1;
                }
            };
            let match_count = re.find_iter(&curr).count();
            if match_count > 0 {
                count += match_count;
                // Only build up to 20 telemetry entries for display performance
                for m in re.find_iter(&curr).take(20) {
                    let replaced_segment = re.replace(m.as_str(), &p.replace).to_string();
                    occs.push(compute_telemetry(&curr, m.start(), m.end(), &replaced_segment));
                }
                curr = re.replace_all(&curr, &p.replace).to_string();
            }
        }
        (curr, count, occs)
    } else {
        let patterns: Vec<&str> = pairs.iter().map(|p| p.find.as_str()).collect();
        let replacements: Vec<&str> = pairs.iter().map(|p| p.replace.as_str()).collect();

        let ac = match AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostFirst)
            .build(&patterns)
        {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[ERROR] Failed to build matcher: {}", e);
                return 1;
            }
        };

        let mut occs = Vec::new();
        let mut count = 0;
        for m in ac.find_iter(&original) {
            count += 1;
            if occs.len() < 20 {
                let pat_idx = m.pattern().as_usize();
                let rep_str = replacements[pat_idx];
                occs.push(compute_telemetry(&original, m.start(), m.end(), rep_str));
            }
        }

        let mod_str = ac.replace_all(&original, &replacements);
        (mod_str, count, occs)
    };

    if count == 0 {
        eprintln!("[REPLACE] No occurrences of pattern found in {}", target_path);
        return 1;
    }

    if dry_run {
        println!("[DRY RUN] Found {} occurrence(s) in {}:", count, target_path);
        for occ in occurrences.iter().take(20) {
            println!("  @@ Line {}:{} (indent: {}) @@", occ.line, occ.col, occ.indent_desc);
            for old_l in &occ.old_lines {
                println!("\x1b[31m-   {}\x1b[0m", old_l);
            }
            for new_l in &occ.new_lines {
                println!("\x1b[32m+   {}\x1b[0m", new_l);
            }
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
        eprintln!("[ERROR] Failed to atomic persist to {}: {}", target_path, e);
        return 1;
    }

    println!("[REPLACE] Successfully replaced {} occurrence(s) in {}:", count, target_path);
    for occ in occurrences.iter().take(20) {
        println!("  @@ Line {}:{} (indent: {}) @@", occ.line, occ.col, occ.indent_desc);
        for old_l in &occ.old_lines {
            println!("\x1b[31m-   {}\x1b[0m", old_l);
        }
        for new_l in &occ.new_lines {
            println!("\x1b[32m+   {}\x1b[0m", new_l);
        }
    }
    0
}

struct Occurrence {
    line: usize,
    col: usize,
    indent_desc: String,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn compute_telemetry(source: &str, start_offset: usize, end_offset: usize, replacement: &str) -> Occurrence {
    let prefix = &source[..start_offset];
    let line = prefix.lines().count().max(1);
    
    let last_nl = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let next_nl = source[end_offset..].find('\n').map(|idx| end_offset + idx).unwrap_or(source.len());
    let col = start_offset.saturating_sub(last_nl) + 1;

    let full_line = &source[last_nl..next_nl];
    let leading_ws: String = full_line.chars().take_while(|c| c.is_whitespace()).collect();
    let indent_desc = if leading_ws.is_empty() {
        "0 (flush left)".to_string()
    } else {
        let spaces = leading_ws.chars().filter(|c| *c == ' ').count();
        let tabs = leading_ws.chars().filter(|c| *c == '\t').count();
        if tabs > 0 {
            format!("{} tabs, {} spaces", tabs, spaces)
        } else {
            format!("{} spaces", spaces)
        }
    };

    let old_match = &source[start_offset..end_offset];
    let old_lines: Vec<String> = old_match.lines().map(|s| s.to_string()).collect();
    let new_lines: Vec<String> = replacement.lines().map(|s| s.to_string()).collect();

    Occurrence {
        line,
        col,
        indent_desc,
        old_lines,
        new_lines,
    }
}
