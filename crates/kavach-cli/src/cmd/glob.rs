//! Fast, gitignore-aware path and file search using `ignore` and `glob-match`.
//! Zero intermediate heap allocation per candidate.

use glob_match::glob_match;
use ignore::WalkBuilder;
use std::io::{self, BufWriter, Write};

pub(crate) fn run(
    pattern: &str,
    path: &str,
    entry_type: Option<char>,
    hidden: bool,
    max_depth: Option<usize>,
    max_count: usize,
) -> i32 {
    let target_path = std::path::Path::new(path);
    if !target_path.exists() {
        eprintln!("kavach glob: path '{}' does not exist", path);
        return 2;
    }
    let mut builder = WalkBuilder::new(path);
    builder
        .hidden(!hidden)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    if let Some(depth) = max_depth {
        builder.max_depth(Some(depth));
    }

    let walker = builder.build();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut count = 0usize;

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_type = entry.file_type();
        if let Some(t) = entry_type {
            match t {
                'f' => {
                    if !file_type.as_ref().map_or(false, |ft| ft.is_file()) {
                        continue;
                    }
                }
                'd' => {
                    if !file_type.as_ref().map_or(false, |ft| ft.is_dir()) {
                        continue;
                    }
                }
                _ => {}
            }
        }

        let p = entry.path();
        let path_str = p.to_string_lossy();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Match against normalized path (without leading ./) or file name
        let clean_path = path_str.strip_prefix("./").unwrap_or(&path_str);
        let matched = glob_match(pattern, clean_path) || glob_match(pattern, &path_str) || glob_match(pattern, file_name);

        if matched {
            let _ = writeln!(out, "{}", path_str);
            count = count.saturating_add(1);
            if count >= max_count {
                break;
            }
        }
    }

    let _ = out.flush();
    0
}
