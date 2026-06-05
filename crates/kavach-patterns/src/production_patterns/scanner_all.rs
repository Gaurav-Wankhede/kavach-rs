//! Scan across all pattern categories.

use super::scanners::scan_filtered;
use super::types::PatternMatch;

pub(super) fn scan(content: &str) -> Vec<PatternMatch> {
    scan_filtered(content, None)
}
