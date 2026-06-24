// `kavach lint audit` — whole-repo over-engineering scan. Ranked delete:/stdlib:/
// native:/yagni:/shrink: findings, biggest-cut first. Report-only, applies nothing.
// SOURCE: ponytail-audit/SKILL.md + ponytail-review/SKILL.md.
use std::path::Path;

use crate::cmd::io_safe;
use crate::cmd::lint::walk::walk_rs;

/// One over-engineering finding. `weight` orders the ranking (bigger cut first).
struct Finding {
    tag: &'static str,
    what: String,
    loc: String,
    weight: u32,
}

/// Tag a single line if it carries a high-signal over-engineering pattern.
/// Conservative on purpose — a report nudges, it never blocks.
fn tag_line(line: &str) -> Option<(&'static str, &'static str, u32)> {
    let t = line.trim_start();
    if t.starts_with("#[allow(dead_code)]") {
        return Some(("delete", "dead-code allow — remove the code or the allow", 5));
    }
    if t.contains(".clone().clone()") {
        return Some(("shrink", "double clone — one suffices", 2));
    }
    if t.contains(".iter().cloned().collect::<Vec") || t.contains(".to_vec().clone()") {
        return Some(("shrink", "needless collect/clone roundtrip", 2));
    }
    if t.contains(".unwrap_or_else(|| Vec::new())") || t.contains(".unwrap_or(Vec::new())") {
        return Some(("stdlib", "unwrap_or_else(Vec::new) — use unwrap_or_default()", 1));
    }
    if t.contains(".map(|x| x)") || t.contains(".map(|x| x.clone())") {
        return Some(("shrink", "identity map — drop it", 2));
    }
    None
}

/// Scan `root`; print ranked findings + net conclusion. Returns 0 (report-only).
pub(crate) fn run(root: &Path) -> i32 {
    let mut found: Vec<Finding> = Vec::new();
    walk_rs(root, root, &mut |rel, content| {
        if is_test_path(rel) {
            return;
        }
        for (i, line) in content.lines().enumerate() {
            if let Some((tag, what, weight)) = tag_line(line) {
                found.push(Finding {
                    tag,
                    what: what.to_owned(),
                    loc: format!("{rel}:{}", i.saturating_add(1)),
                    weight,
                });
            }
        }
    });
    emit(&mut found)
}

fn is_test_path(rel: &str) -> bool {
    rel.ends_with("_test.rs") || rel.contains("/tests/") || rel.ends_with("/tests.rs")
}

fn emit(found: &mut [Finding]) -> i32 {
    if found.is_empty() {
        return io_safe::print_or_exit("Lean already. Ship.")
            .map_or_else(io_safe::into_exit_code, |()| 0);
    }
    found.sort_by_key(|f| std::cmp::Reverse(f.weight));
    for f in found.iter() {
        let line = format!("  {} {}. [{}]", f.tag, f.what, f.loc);
        if let Err(e) = io_safe::print_or_exit(&line) {
            return io_safe::into_exit_code(e);
        }
    }
    let net: u32 = found.iter().map(|f| f.weight).sum();
    let summary = format!("net: ~-{net} lines possible across {} finding(s).", found.len());
    io_safe::print_or_exit(&summary).map_or_else(io_safe::into_exit_code, |()| 0)
}

#[cfg(test)]
#[path = "audit_test.rs"]
mod tests;
