//! nextest advisories: suggest `cargo nextest run` over plain `cargo test`, and
//! scaffold a tuned `.config/nextest.toml` in a supervised Rust project that
//! lacks one. Both quote-aware + fail-soft (never block a routine test run).
mod template;

use crate::gates::pre_tool_bash::strip_quoted_regions;
use template::NEXTEST_TEMPLATE;

/// Advisory: suggest `cargo nextest run` when plain `cargo test` is used.
/// Quote-aware: the phrase inside another tool's quoted arg is data, not a
/// command-position invocation (CWE-184). RESEARCH:
/// <https://cwe.mitre.org/data/definitions/184.html>
pub(in crate::gates::pre_tool_bash) fn check_nextest_advisory(cmd: &str) -> Option<String> {
    let stripped = strip_quoted_regions(cmd.trim());
    if !stripped.contains("cargo test") || stripped.contains("cargo nextest") {
        return None;
    }
    Some(
        "[NEXTEST_ADVISORY] `cargo test` detected. Consider `cargo nextest run` instead.\n\
         nextest runs each test in a separate process in parallel — up to 3x faster.\n\
         Install: `cargo install cargo-nextest --locked`\n\
         Run: `cargo nextest run` (or `cargo nextest run -p <crate>` for a single crate)"
            .to_owned(),
    )
}

/// Walk ancestors of `start` to find the nearest directory holding `Cargo.toml`.
/// `None` ⇒ not inside a Rust project — the scaffold must not fire.
fn cargo_workspace_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").is_file())
        .map(std::path::Path::to_path_buf)
}

/// When a test command runs in a supervised Rust project that lacks
/// `.config/nextest.toml`, scaffold the harness's tuned config there. Idempotent:
/// a project that already has the file is left untouched. Returns an advisory
/// naming the written file, or `None` when nothing was scaffolded.
///
/// Fail-soft: a quote-buried phrase, a non-Rust cwd, or an I/O error all yield
/// `None` — a routine `cargo test` is never blocked by this.
pub(in crate::gates::pre_tool_bash) fn scaffold_nextest_config(
    cmd: &str,
    cwd: &std::path::Path,
) -> Option<String> {
    let stripped = strip_quoted_regions(cmd.trim());
    if !(stripped.contains("cargo test") || stripped.contains("cargo nextest")) {
        return None;
    }
    let root = cargo_workspace_root(cwd)?;
    let config_dir = root.join(".config");
    let config_path = config_dir.join("nextest.toml");
    if config_path.exists() {
        return None;
    }
    std::fs::create_dir_all(&config_dir).ok()?;
    std::fs::write(&config_path, NEXTEST_TEMPLATE).ok()?;
    Some(format!(
        "[NEXTEST_SCAFFOLD] No .config/nextest.toml in this project — wrote one.\n\
         {} now configures parallel per-test isolation, a 60s hung-test \
         kill, and a serial group for env-mutating tests.\n\
         Commit it so the whole team's test runs are tuned identically.",
        config_path.display()
    ))
}
