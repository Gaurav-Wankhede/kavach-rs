//! TDD pre-write gate: block production code unless its test came first (Red).
//! Fast-path = the unit's test was touched earlier this turn, or the write
//! carries an in-file `#[test]`. Escape: `KAVACH_TDD_BYPASS=1`.

use crate::gates::pre_write_context::WriteContext;

/// `Some(reason)` blocks the write when production code lacks a test-first signal.
pub(super) fn check(
    ctx: &WriteContext<'_>,
    session: &kavach_session::SessionState,
) -> Option<String> {
    if std::env::var_os("KAVACH_TDD_BYPASS").is_some() {
        return None;
    }
    if !ctx.is_code || is_test_path(ctx.file_path) {
        return None;
    }
    // A comment/doc-only change (the written/edited text is all comments+blanks)
    // carries no executable code → exempt, so the comment sweep needs no per-file
    // test. Uses ctx.content (the changed text), not the whole post-edit file.
    if is_comment_only(ctx.content) {
        return None;
    }
    let stem = unit_stem(ctx.file_path);
    // Inline tests are FORBIDDEN — tests live in a separate mapped file.
    if has_inline_test(ctx.content) {
        return Some(format!(
            "[TDD:P0] MOVE THE TEST OUT, THEN WRITE THE CODE. `{stem}` carries an inline \
             test — put it in a SEPARATE file (e.g. `{stem}/tests.rs` mapped via `#[path]`), \
             never inside the production file. If this looks wrong, READ this guard's \
             source and fix the real cause — never route around it."
        ));
    }
    // The unit's test must have been OBSERVED RED this turn (recorded in
    // `tdd_red_units` by the post-tool Bash gate) — a test-FILE touch alone is a
    // vacuous after-the-fact test. SOURCE: decision.tdd.red-phase-live-oracle.
    if session.tdd_red_units.iter().any(|u| u == stem) {
        return None;
    }
    // Transitional fallback: the unit's test file was touched but no Red was
    // observed yet — direct the agent to RUN it and watch it fail, naming the
    // missing proof explicitly rather than silently passing on a touch.
    let touched = session
        .files_modified_this_turn
        .iter()
        .any(|f| test_matches_unit(f, stem));
    let red_hint = if touched {
        format!(
            "Its test file is touched but was NOT observed RED. RUN `cargo nextest run` \
             for `{stem}` and confirm it FAILS first"
        )
    } else {
        format!(
            "Write the FAILING test in a SEPARATE file (`{stem}_test.rs` or \
             `{stem}/tests.rs` mapped via `#[path]`), RUN it, confirm it FAILS"
        )
    };
    Some(format!(
        "[TDD:P0] WRITE THE TEST FIRST. Production code for `{stem}` has no observed-Red \
         test-first. {red_hint}, THEN write the code. If this looks wrong, READ this \
         guard's source and fix the real cause — never route around it."
    ))
}

/// True for any test file — the shared detector OR the `_test(s).rs` suffix the
/// shared `is_test_file` misses. A test write is never gated.
fn is_test_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    kavach_patterns::is_test_file(path) || name.ends_with("_test.rs") || name.ends_with("_tests.rs")
}

/// True when every non-blank line of `changed` is a comment (`//`/`///`/`//!`) or
/// an attribute — i.e. the change adds no executable code.
fn is_comment_only(changed: &str) -> bool {
    let mut saw_line = false;
    for line in changed.lines() {
        let t = line.trim_start();
        if t.is_empty() {
            continue;
        }
        saw_line = true;
        if !(t.starts_with("//") || t.starts_with("#[") || t.starts_with("#!")) {
            return false;
        }
    }
    saw_line
}

/// Comment-only check that treats lines already in `current` as unchanged context.
fn is_comment_only_added(changed: &str, current: &str) -> bool {
    let mut saw_line = false;
    for line in changed.lines() {
        let t = line.trim_start();
        if t.is_empty() {
            continue;
        }
        saw_line = true;
        let is_comment = t.starts_with("//") || t.starts_with("#[") || t.starts_with("#!");
        if !is_comment && !current.contains(line) {
            return false;
        }
    }
    saw_line
}

/// Basename without the `.rs` extension.
pub(crate) fn unit_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".rs")
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

pub(crate) fn production_stem_of(test_path: &str) -> Option<String> {
    let s = test_path.replace('\\', "/");
    let name = s.rsplit('/').next().unwrap_or(&s);
    if name == "tests.rs" {
        let parent = s.strip_suffix("/tests.rs").and_then(|p| p.rsplit('/').next());
        return parent.filter(|p| *p != "src" && !p.is_empty()).map(str::to_owned);
    }
    if let Some(file) = name.strip_suffix(".rs") {
        if let Some(stem) = file.strip_suffix("_test").or_else(|| file.strip_suffix("_tests")) {
            return (!stem.is_empty()).then(|| stem.to_owned());
        }
        if s.contains("/tests/") {
            return (!file.is_empty()).then(|| file.to_owned());
        }
    }
    None
}

/// True when the content carries an inline test (forbidden in production files):
/// using the `#[` attribute markers. A `#[path]` to an external file is NOT inline.
pub(super) fn has_inline_test(content: &str) -> bool {
    content.contains("#[test]")
        || content.contains("#[tokio::test]")
        || (content.contains("#[cfg(test)]") && !content.contains("#[path"))
}

#[cfg(test)]
#[path = "tdd_guard/tests.rs"]
mod tests;
