//! TDD pre-write gate: block production code unless its test came first (Red).
//! Fast-path = the unit's test was touched earlier this turn, or the write
//! carries an in-file `#[test]`. Escape: `KAVACH_TDD_BYPASS=1`.

use crate::gates::pre_write_context::WriteContext;

/// `Some(reason)` blocks the write when production code lacks a test-first signal.
pub(super) fn check(ctx: &WriteContext<'_>, session: &kavach_session::SessionState) -> Option<String> {
    if std::env::var_os("KAVACH_TDD_BYPASS").is_some() {
        return None;
    }
    if !ctx.is_code || is_test_path(ctx.file_path) {
        return None;
    }
    let stem = unit_stem(ctx.file_path);
    // Inline tests are FORBIDDEN — tests live in a separate mapped file.
    if has_inline_test(&ctx.effective_content) {
        return Some(format!(
            "[TDD:P0] BLOCKED. `{stem}` carries an inline test — tests must live in a \
             SEPARATE file (e.g. `{stem}/tests.rs` mapped via `#[path]`), never inside \
             the production file. Move the test out, then write the code. \
             Bypass (emergencies only): KAVACH_TDD_BYPASS=1."
        ));
    }
    // The unit's separate test file must have come first THIS turn (Red).
    if session
        .files_modified_this_turn
        .iter()
        .any(|f| test_matches_unit(f, stem))
    {
        return None;
    }
    Some(format!(
        "[TDD:P0] BLOCKED. Production code for `{stem}` has no test-first (Red). \
         Write the FAILING test in a SEPARATE file (`{stem}_test.rs` or \
         `{stem}/tests.rs` mapped via `#[path]`) THIS turn, confirm it fails, THEN \
         write the code. Bypass (emergencies only): KAVACH_TDD_BYPASS=1."
    ))
}

/// True for any test file — the shared detector OR the `_test(s).rs` suffix the
/// shared `is_test_file` misses. A test write is never gated.
fn is_test_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    kavach_patterns::is_test_file(path)
        || name.ends_with("_test.rs")
        || name.ends_with("_tests.rs")
}

/// Basename without the `.rs` extension.
pub(super) fn unit_stem(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path).trim_end_matches(".rs")
}

/// True when `test_path` is a recognised test for `stem`: sibling
/// `{stem}_test(s).rs`, a `tests/{stem}.rs` integration test, or the in-engine
/// `{stem}/tests.rs` subdir module.
pub(super) fn test_matches_unit(test_path: &str, stem: &str) -> bool {
    let s = test_path.replace('\\', "/");
    let name = s.rsplit('/').next().unwrap_or(&s);
    if s.contains("/tests/") && name == format!("{stem}.rs") {
        return true;
    }
    if name == "tests.rs" && s.contains(format!("/{stem}/tests.rs").as_str()) {
        return true;
    }
    name == format!("{stem}_test.rs") || name == format!("{stem}_tests.rs")
}

/// True when the content carries an inline test (forbidden in production files):
/// a `#[test]`/`#[tokio::test]` fn or an inline `#[cfg(test)] mod`. A `#[path]`
/// declaration pointing at an external test file is NOT inline.
pub(super) fn has_inline_test(content: &str) -> bool {
    content.contains("#[test]")
        || content.contains("#[tokio::test]")
        || (content.contains("#[cfg(test)]") && !content.contains("#[path"))
}

#[cfg(test)]
#[path = "tdd_guard/tests.rs"]
mod tests;
