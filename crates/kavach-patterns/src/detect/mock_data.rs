//! Mock data detection.
use std::sync::OnceLock;

use crate::file_types::{is_allowlisted, is_backend_file, is_frontend_file};
use crate::regex_patterns::P;
use regex::Regex;

/// Fallback for an out-of-range static-table index: never matches. The tables
/// are compile-time constants, so this path is unreachable in practice; the
/// debug_assert fires in tests instead of panicking in production.
static NEVER_MATCH: OnceLock<Regex> = OnceLock::new();

#[inline]
fn idx_or_skip(r: &[Regex], i: usize) -> &Regex {
    match r.get(i) {
        Some(re) => re,
        None => {
            debug_assert!(false, "regex table index {i} out of range");
            r.first().unwrap_or(NEVER_MATCH.get_or_init(|| {
                Regex::new("$^").expect("trivial regex compiles")
            }))
        }
    }
}

/// Detect hardcoded mock/stub data in code.
#[must_use]
pub fn detect_mock_data(fp: &str, content: &str) -> Option<String> {
    if content.is_empty() || is_allowlisted(fp) {
        return None;
    }
    let r = &*P;
    if is_frontend_file(fp) {
        if let Some(m) = idx_or_skip(r, 46).find(content) {
            return Some(format!("frontend_mock_const:{}", m.as_str().trim()));
        }
        if idx_or_skip(r, 47).is_match(content) {
            return Some("frontend_hardcoded_array".into());
        }
        if idx_or_skip(r, 48).is_match(content) {
            return Some("frontend_useState_hardcoded".into());
        }
        if let Some(m) = idx_or_skip(r, 49).find(content) {
            return Some(format!("frontend_fake_engagement:{}", m.as_str()));
        }
    }
    if is_backend_file(fp) && idx_or_skip(r, 50).is_match(content) {
        return Some("backend_hardcoded_json_vec".into());
    }
    None
}
