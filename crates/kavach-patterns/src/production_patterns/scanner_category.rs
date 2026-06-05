//! Scan a specific pattern category.

use super::scanners::scan_filtered;
use super::types::{PatternCategory, PatternMatch};

pub(super) fn scan(category: PatternCategory, content: &str) -> Vec<PatternMatch> {
    scan_filtered(content, Some(category))
}
