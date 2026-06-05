//! Mock data detection.
use crate::file_types::{is_allowlisted, is_backend_file, is_frontend_file};
use crate::regex_patterns::P;
use regex::Regex;

#[inline]
#[expect(
    clippy::indexing_slicing,
    reason = "index is a constant literal bounded by regex array size"
)]
fn idx(r: &[Regex], i: usize) -> &Regex {
    &r[i]
}

/// Detect hardcoded mock/stub data in code.
#[must_use]
pub fn detect_mock_data(fp: &str, content: &str) -> Option<String> {
    if content.is_empty() || is_allowlisted(fp) {
        return None;
    }
    let r = &*P;
    if is_frontend_file(fp) {
        if let Some(m) = idx(r, 46).find(content) {
            return Some(format!("frontend_mock_const:{}", m.as_str().trim()));
        }
        if idx(r, 47).is_match(content) {
            return Some("frontend_hardcoded_array".into());
        }
        if idx(r, 48).is_match(content) {
            return Some("frontend_useState_hardcoded".into());
        }
        if let Some(m) = idx(r, 49).find(content) {
            return Some(format!("frontend_fake_engagement:{}", m.as_str()));
        }
    }
    if is_backend_file(fp) && idx(r, 50).is_match(content) {
        return Some("backend_hardcoded_json_vec".into());
    }
    None
}
