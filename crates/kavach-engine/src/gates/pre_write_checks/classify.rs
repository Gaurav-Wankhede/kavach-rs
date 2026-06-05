//! File-path classification for the pre-write gate: is it a code file, and is it
//! a test file / otherwise exempt from test enforcement?

/// Check if file is a code file (delegates to patterns).
#[must_use]
pub(crate) fn is_code_write(path: &str) -> bool {
    kavach_patterns::is_code_file(path)
}

/// Check if a file path is a test file or exempt from test enforcement.
/// Test files, configs, docs, and non-code files bypass the test gate.
#[must_use]
pub(crate) fn is_test_or_exempt(path: &str) -> bool {
    if !is_code_write(path) {
        return true;
    }
    let test_patterns = ["_tests.rs", "_test.rs", "tests/", "test_"];
    test_patterns.iter().any(|pat| path.contains(pat))
}
