//! Scan a specific pattern category.

use super::scanners::scan_filtered;
use super::types::{PatternCategory, PatternMatch};

#[allow(dead_code)]
pub(super) fn scan(category: PatternCategory, content: &str) -> Vec<PatternMatch> {
    scan_filtered(content, Some(category))
}
