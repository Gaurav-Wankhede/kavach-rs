use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(crate) fn run(path_str: &str, range_str: &str) -> i32 {
    let p = Path::new(path_str);
    if !p.exists() {
        eprintln!("kavach slice: file '{}' does not exist", path_str);
        return 2;
    }

    let (start_line, end_line) = match parse_range(range_str) {
        Some(r) => r,
        None => {
            eprintln!("kavach slice: invalid range format '{}'. Expected 'start:end' (e.g. 50:80) or single line '100'", range_str);
            return 2;
        }
    };

    let file = match File::open(p) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("kavach slice: failed to open file '{}': {}", path_str, e);
            return 1;
        }
    };

    let reader = BufReader::new(file);
    let mut current_line = 1usize;
    let mut printed_count = 0usize;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                eprintln!("kavach slice: error reading line {}: {}", current_line, e);
                return 1;
            }
        };

        if current_line >= start_line && current_line <= end_line {
            println!("{}:{}", current_line, line);
            printed_count += 1;
        }

        if current_line > end_line {
            break;
        }

        current_line += 1;
    }

    if printed_count == 0 && current_line <= start_line {
        eprintln!("kavach slice: file '{}' has only {} lines (requested start: {})", path_str, current_line - 1, start_line);
    }

    0
}

fn parse_range(s: &str) -> Option<(usize, usize)> {
    let clean = s.trim();
    if let Some((start_s, end_s)) = clean.split_once(':') {
        let start: usize = start_s.trim().parse().ok()?;
        let end: usize = end_s.trim().parse().ok()?;
        if start == 0 {
            return None;
        }
        Some((start, end.max(start)))
    } else {
        let single: usize = clean.parse().ok()?;
        if single == 0 {
            return None;
        }
        Some((single, single))
    }
}
