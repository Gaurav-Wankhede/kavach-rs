//! Production-code path classifier: real source files, excluding tests/builds.

/// `true` iff `path` is production source code (.rs/.ts/.tsx/.py/.go) that is
/// NOT a test file, test dir, or build artifact.
pub(super) fn is_production_code(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    #[expect(
        clippy::case_sensitive_file_extension_comparisons,
        reason = "operands are lowercased; lint is false-positive"
    )]
    let is_code = path_lower.ends_with(".rs")
        || path_lower.ends_with(".ts")
        || path_lower.ends_with(".tsx")
        || path_lower.ends_with(".py")
        || path_lower.ends_with(".go");
    if !is_code {
        return false;
    }
    let in_test_dir = path.contains("/tests/")
        || path.contains("/test/")
        || path.starts_with("tests/")
        || path.starts_with("test/");
    let in_test_file = path.contains("_test.") || path.contains(".test.");
    let in_build_dir = path.contains("/target/") || path.contains("/node_modules/");
    !in_test_dir && !in_test_file && !in_build_dir
}
