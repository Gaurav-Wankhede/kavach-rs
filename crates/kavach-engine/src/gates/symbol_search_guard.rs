//! Advise `kavach origin` when a Grep/Glob pattern is a bare symbol.
use super::grep_guard::origin_pointer;

/// Some(advisory) when tool is Grep/Glob AND pattern is a bare identifier; else None.
pub(crate) fn check_tool_search(tool_name: &str, pattern: &str) -> Option<String> {
    if tool_name != "Grep" && tool_name != "Glob" {
        return None;
    }
    if !is_symbol_shaped(pattern) {
        return None;
    }
    Some(origin_pointer(pattern))
}

fn is_symbol_shaped(p: &str) -> bool {
    let p = p.trim();
    !p.is_empty()
        && p.len() <= 64
        && p.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
#[path = "symbol_search_guard_test.rs"]
mod symbol_search_guard_test;
