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

    if dry_run {
        println!("[DRY RUN] Found {} occurrence(s) in {}", count, path);
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

    println!("[REPLACE] Successfully replaced {} occurrence(s) in {}", count, path);
    0
}
