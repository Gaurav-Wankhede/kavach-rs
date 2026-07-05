//! Content-oriented security stages: Python ban, hardcoded secrets, empty-write
//! to code files, and memory-file writes (forced to kavach-db).
use super::SecurityResult;
use crate::gates::pre_write_checks::is_code_write;
use crate::gates::pre_write_context::WriteContext;

/// `.py` files are banned inside the session workdir — they bypass the hook pipeline.
pub(super) fn python_ban(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    let is_py = std::path::Path::new(ctx.file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("py"));
    if !is_py {
        return None;
    }
    let session_wd = kavach_session::get_or_create_session().work_dir;
    (!session_wd.is_empty() && ctx.file_path.starts_with(&session_wd)).then(|| {
        SecurityResult::Block(
            "BLOCKED: .py files are banned in this workspace. \
             Python bypasses the kavach hook pipeline. \
             SQL ops: Write tool → .sql file → sqlx migrate run. \
             Scripts: use Bash directly or implement as a Rust binary."
                .to_owned(),
        )
    })
}

/// Hardcoded credentials in content — hard block.
///
/// Exempts only GENUINE test files (fixtures legitimately carry dummy secrets),
/// not every non-code file. `ctx.is_test` is true for ANY non-code path
/// (`is_test_or_exempt` returns true when `!is_code_write`), which wrongly
/// captured `.env`/credential files — letting a real `AWS_SECRET_ACCESS_KEY=AKIA…`
/// write through. Gate on an actual test-path marker instead.
/// SOURCE: loophole audit (cursor-edge), runtime-proven.
pub(super) fn hardcoded_secret(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    let is_real_test_file = ["_tests.rs", "_test.rs", "tests/", "test_", "/fixtures/"]
        .iter()
        .any(|pat| ctx.file_path.contains(pat));
    if ctx.content.is_empty() || is_real_test_file {
        return None;
    }
    kavach_config::has_secret_in_content(ctx.content).map(|secret_msg| {
        SecurityResult::Block(format!(
            "[SECRETS] {secret_msg} in {} -> move the credential to an env var (runtime script, value never in context) -> retry.",
            ctx.file_path
        ))
    })
}

/// Empty content on a Write to a code file silently bypasses every content guard.
pub(super) fn empty_code_write(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    (ctx.tool_name == "Write" && is_code_write(ctx.file_path) && ctx.content.is_empty()).then(
        || {
            SecurityResult::Block(
                "BLOCKED: Write called with empty content on a code file. \
             All content guards (rust, ts, sql, css) would be silently bypassed."
                    .to_owned(),
            )
        },
    )
}

/// Memory file writes are redirected to kavach-db.
pub(super) fn memory_file(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    super::super::memory_write_guard::is_memory_file(ctx.file_path).then(|| {
        SecurityResult::Block(super::super::memory_write_guard::block_message(
            ctx.file_path,
        ))
    })
}
