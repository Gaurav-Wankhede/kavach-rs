//! Atomic UI Production Gate — enforces Brad Frost's Atomic Design hierarchy across frameworks.

mod detectors;
mod types;
mod util;

#[cfg(test)]
#[path = "atomic_ui_guard/tests.rs"]
mod tests;

pub use types::{AtomicSeverity, AtomicViolation};

use util::{classify_path, is_ui_file};

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<AtomicViolation> {
    if !is_ui_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let level = classify_path(file_path);
    detectors::dispatch(file_path, content, level)
}
