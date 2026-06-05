// SPEC: roadmap.unit.oversized-toolbelt-sweep — tokei shell-out replaces in-tree walker.
// SOURCE: https://github.com/XAMPPRocky/tokei
// SOURCE: https://rust-lang.github.io/rust-clippy/master/index.html#too_many_lines
use std::process::Command;

use crate::cli::OversizedFormat;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(dir: &str, threshold: u32, format: OversizedFormat) -> i32 {
    let out = match Command::new("tokei")
        .args(["--types", "Rust", "--output", "json", dir])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => {
            return emit_err(&format!(
                "tokei exit {}: {}",
                o.status,
                String::from_utf8_lossy(&o.stderr)
            ));
        }
        Err(e) => {
            return emit_err(&format!(
                "tokei not on PATH ({e}). Install via `brew install tokei`."
            ));
        }
    };
    let v: serde_json::Value = match serde_json::from_slice(&out) {
        Ok(v) => v,
        Err(e) => return emit_err(&format!("tokei json parse: {e}")),
    };
    let Some(reports) = v.pointer("/Rust/reports").and_then(|r| r.as_array()) else {
        return emit_err("tokei output missing /Rust/reports");
    };
    let offenders = collect_offenders(reports, threshold);
    match format {
        OversizedFormat::Text => emit_text(&offenders, threshold),
        OversizedFormat::Json => emit_json(&offenders, threshold),
    }
}

fn collect_offenders(reports: &[serde_json::Value], threshold: u32) -> Vec<(u64, String)> {
    let mut out: Vec<(u64, String)> = Vec::new();
    for r in reports {
        let Some(name) = r.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        if is_excluded(name) {
            continue;
        }
        let Some(code) = r.pointer("/stats/code").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        if code <= u64::from(threshold) {
            continue;
        }
        if has_split_marker(name) {
            continue;
        }
        out.push((code, name.to_owned()));
    }
    // SOURCE: https://rust-lang.github.io/rust-clippy/rust-1.95.0/index.html#unnecessary_sort_by
    out.sort_by_key(|b| std::cmp::Reverse(b.0));
    out
}

fn is_excluded(path: &str) -> bool {
    path.ends_with("/tests.rs")
        || path.ends_with("_tests.rs")
        || path.contains("/target/")
        || path.contains("/tests/")
}

fn has_split_marker(path: &str) -> bool {
    use std::io::{BufRead, BufReader};
    let Ok(f) = std::fs::File::open(path) else {
        return false;
    };
    let reader = BufReader::new(f);
    for line in reader.lines().take(5).map_while(Result::ok) {
        if line.contains("// split:") {
            return true;
        }
    }
    false
}

fn emit_text(offenders: &[(u64, String)], threshold: u32) -> i32 {
    if offenders.is_empty() {
        return match print_or_exit(&format!("ok — no files exceed {threshold} code-LOC")) {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        };
    }
    for (code, name) in offenders {
        let line = format!("OVERSIZED  {name}  {code} code-LOC");
        if let Err(io) = print_or_exit(&line) {
            return into_exit_code(io);
        }
    }
    1
}

fn emit_json(offenders: &[(u64, String)], threshold: u32) -> i32 {
    let arr: Vec<serde_json::Value> = offenders
        .iter()
        .map(|(code, name)| serde_json::json!({ "path": name, "code_loc": code }))
        .collect();
    let payload =
        serde_json::json!({ "offenders": arr, "total": offenders.len(), "threshold": threshold });
    let s = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(e) => return emit_err(&format!("json serialize: {e}")),
    };
    match print_or_exit(&s) {
        Ok(()) => i32::from(!offenders.is_empty()),
        Err(io) => into_exit_code(io),
    }
}

fn emit_err(msg: &str) -> i32 {
    let line = format!("error: {msg}");
    match ewrite_or_exit(&line) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}
