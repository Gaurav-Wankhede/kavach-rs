//! Path classifiers for universal pre-write guards.
//! Only the universal `is_test` helper lives here — used by all content
//! guards to exempt test files from production pattern enforcement.

/// True if path is a test file — exempt from all platform guards.
pub(crate) fn is_test(path: &str) -> bool {
    path.contains(".test.")
        || path.contains(".spec.")
        || path.contains("__tests__")
        || path.contains("/tests/")
        || path.contains("_test.rs")
        || path.contains("_test.ts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_match_test_file_extensions() {
        assert!(is_test("Component.test.tsx"));
        assert!(is_test("page.spec.astro"));
    }

    #[test]
    fn should_match_test_directory_paths() {
        assert!(is_test("src/__tests__/handler.rs"));
        assert!(is_test("crate/tests/integration.rs"));
    }

    #[test]
    fn should_not_match_regular_source_files() {
        assert!(!is_test("Component.tsx"));
        assert!(!is_test("src/lib.rs"));
    }
}
