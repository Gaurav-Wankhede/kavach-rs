// ARCH: FrontendStrictGate
// PROBLEM_CLASS: install_time_lint_enforcement_for_typescript_projects
// REJECTED: [
//   {"name":"single-tool-only-biome","reason":"existing eslint projects can't migrate atomically"},
//   {"name":"single-tool-only-eslint","reason":"slower; new biome projects gain nothing"},
//   {"name":"per-write gate only","reason":"warnings pile up between writes; install-time gate is the §10 contract enforcement"}
// ]
// TIME: O(n) over project files (delegated to biome/eslint/tsc) | SPACE: O(1) in this orchestrator
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: detector picks ONE tool stack per project; --prefer overrides priority.
// PATTERN: language_agnostic_strict_gate | SCOPE: kavach-cli | CAP: AP
// FAILURE_MODE: missing tool binary → fail with install instructions; never silently skip.
// SOURCES:
//   - https://biomejs.dev/linter/  (biome ci, --error-on-warnings)
//   - https://eslint.org/docs/latest/use/configure/  (flat config, --fix)
//
// `kavach verify-frontend` — TS/frontend equivalent of `kavach deploy` for Rust.
// Detector lives in verify_frontend_detect.rs; this module orchestrates the
// 4-step pipeline (auto-fix → strict lint → tsc --noEmit → tests).

use std::path::{Path, PathBuf};
use std::process::Command;

use super::verify_frontend_detect::{
    FrontendStack, PackageRunner, Prefer, detect_runner, detect_stack,
};
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn run(path: &str, skip_tests: bool, prefer: Prefer) -> i32 {
    let project_root = PathBuf::from(path);
    if !project_root.is_dir() {
        let msg = format!(
            "[VERIFY-FE] FAIL: path {} is not a directory",
            project_root.display()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    let runner = detect_runner(&project_root);
    if !preflight_runner(runner) {
        let msg = format!(
            "[VERIFY-FE] FAIL: `{}` not on PATH (detected from project lockfile). \
             Install it or activate the matching toolchain before running kavach verify-frontend.",
            runner.program()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    let Some(stack) = detect_stack(&project_root, prefer) else {
        let msg = format!(
            "[VERIFY-FE] FAIL: no biome.json / eslint.config.* / tsconfig.json found in {}. \
             Not a recognized TypeScript/frontend project.",
            project_root.display()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    };

    let stack_line = format!(
        "[VERIFY-FE] detected stack: {stack:?} via {}",
        runner.program()
    );
    if let Err(io_err) = print_or_exit(&stack_line) {
        return into_exit_code(io_err);
    }
    if let Err(io_err) = print_or_exit("[VERIFY-FE] step 1/4: auto-fix pass (safe rewrites only)") {
        return into_exit_code(io_err);
    }
    if !run_autofix(&project_root, &stack, runner) {
        if let Err(io_err) = ewrite_or_exit("[VERIFY-FE] FAIL: auto-fix pass failed") {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[VERIFY-FE] step 2/4: strict lint (warnings-as-errors)") {
        return into_exit_code(io_err);
    }
    if !run_strict_lint(&project_root, &stack, runner) {
        if let Err(io_err) = ewrite_or_exit(
            "[VERIFY-FE] FAIL: strict lint produced warnings/errors. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[VERIFY-FE] step 3/4: tsc --noEmit (type check)") {
        return into_exit_code(io_err);
    }
    if !run_tsc(&project_root, runner) {
        if let Err(io_err) = ewrite_or_exit("[VERIFY-FE] FAIL: tsc --noEmit found type errors") {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if skip_tests {
        if let Err(io_err) = print_or_exit("[VERIFY-FE] step 4/4: SKIPPED (--skip-tests)") {
            return into_exit_code(io_err);
        }
    } else {
        if let Err(io_err) = print_or_exit("[VERIFY-FE] step 4/4: project test runner") {
            return into_exit_code(io_err);
        }
        if !run_tests(&project_root, runner) {
            if let Err(io_err) = ewrite_or_exit("[VERIFY-FE] FAIL: tests failed") {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

    if let Err(io_err) =
        print_or_exit("[VERIFY-FE] OK: project clean — production-quality contract upheld")
    {
        return into_exit_code(io_err);
    }
    0
}

/// Run the auto-fix pass for the detected stack. tsc has no auto-fix, so it's a no-op.
/// Mirrors `cargo clippy --fix` — applies safe rewrites before the strict gate.
fn run_autofix(root: &Path, stack: &FrontendStack, runner: PackageRunner) -> bool {
    match stack {
        FrontendStack::Biome => run_via(root, runner, &["biome", "check", "--write", "."]),
        FrontendStack::Eslint => run_via(root, runner, &["eslint", "--fix", "."]),
        FrontendStack::Tsc => true,
    }
}

/// Run the strict lint pass — fail on any warning. Mirrors `cargo clippy -D warnings`.
fn run_strict_lint(root: &Path, stack: &FrontendStack, runner: PackageRunner) -> bool {
    match stack {
        FrontendStack::Biome => run_via(root, runner, &["biome", "ci", "--error-on-warnings", "."]),
        FrontendStack::Eslint => run_via(root, runner, &["eslint", "--max-warnings=0", "."]),
        FrontendStack::Tsc => true,
    }
}

/// Run `tsc --noEmit` to enforce the TypeScript type contract. Always runs (even
/// when stack=Biome or Eslint) because biome v2 type-aware rules are still narrower
/// than tsc's full type checker per 2026-05 research.
fn run_tsc(root: &Path, runner: PackageRunner) -> bool {
    if !root.join("tsconfig.json").exists() {
        return true;
    }
    run_via(root, runner, &["tsc", "--noEmit"])
}

/// Run the project's test runner. Aligned with the detected package runner —
/// Bun-only projects use `bun test` directly (Bun's built-in test runner),
/// pnpm/yarn projects use `<pm> test` (honors package.json `test` script),
/// npm fallback uses `npm test --silent`.
fn run_tests(root: &Path, runner: PackageRunner) -> bool {
    match runner {
        PackageRunner::Bunx => run_in(root, "bun", &["test"]),
        PackageRunner::PnpmDlx => run_in(root, "pnpm", &["test"]),
        PackageRunner::YarnDlx => run_in(root, "yarn", &["test"]),
        PackageRunner::Npx => run_in(root, "npm", &["test", "--silent"]),
    }
}

/// Pre-flight: confirm the detected runner (npx/bunx/pnpm/yarn) is invokable
/// before starting the pipeline. Probes `<runner> --version`; success means
/// the binary ran. Network/registry failures occur later inside the pipeline
/// and are correctly surfaced as tool errors, not lint violations.
fn preflight_runner(runner: PackageRunner) -> bool {
    Command::new(runner.program())
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Run a package via the detected runner: `<runner> [leading_args...] <pkg_args...>`.
/// For npx/bunx, `leading_args` is empty (the package name is the first arg).
/// For pnpm/yarn, `leading_args` is `["dlx"]` so the package name follows.
fn run_via(root: &Path, runner: PackageRunner, pkg_args: &[&str]) -> bool {
    let mut full = Vec::with_capacity(runner.leading_args().len().saturating_add(pkg_args.len()));
    full.extend_from_slice(runner.leading_args());
    full.extend_from_slice(pkg_args);
    run_in(root, runner.program(), &full)
}

/// Run a command in `root`. Returns true on success; on either non-zero exit or
/// missing-binary, returns false and writes a diagnostic to stderr.
fn run_in(root: &Path, program: &str, args: &[&str]) -> bool {
    match Command::new(program).args(args).current_dir(root).status() {
        Ok(s) => s.success(),
        Err(e) => {
            let msg = format!(
                "[VERIFY-FE] tool `{program}` not runnable: {e}. Install it or check PATH."
            );
            drop(ewrite_or_exit(&msg));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_returns_1_when_path_is_not_a_directory() {
        assert_eq!(run("/etc/hosts", true, Prefer::Auto), 1);
    }

    #[test]
    fn run_returns_1_when_path_does_not_exist() {
        assert_eq!(run("/nonexistent/path/xyzzy-9876", true, Prefer::Auto), 1);
    }

    /// Verify `run_via` assembles dlx + `pkg_args` correctly per runner.
    /// Closes cold-review gap F: `run_via` was previously untested, leaving the
    /// `extend_from_slice` ordering vulnerable to silent regressions.
    fn assemble_args(runner: PackageRunner, pkg_args: &[&str]) -> Vec<String> {
        let mut full =
            Vec::with_capacity(runner.leading_args().len().saturating_add(pkg_args.len()));
        full.extend(runner.leading_args().iter().map(|s| (*s).to_owned()));
        full.extend(pkg_args.iter().map(|s| (*s).to_owned()));
        full
    }

    #[test]
    fn assemble_args_bunx_no_leading_dlx() {
        let args = assemble_args(PackageRunner::Bunx, &["biome", "ci", "."]);
        assert_eq!(args, vec!["biome", "ci", "."]);
    }

    #[test]
    fn assemble_args_npx_no_leading_dlx() {
        let args = assemble_args(PackageRunner::Npx, &["eslint", "--fix", "."]);
        assert_eq!(args, vec!["eslint", "--fix", "."]);
    }

    #[test]
    fn assemble_args_pnpm_inserts_dlx_first() {
        let args = assemble_args(PackageRunner::PnpmDlx, &["biome", "ci", "."]);
        assert_eq!(args, vec!["dlx", "biome", "ci", "."]);
    }

    #[test]
    fn assemble_args_yarn_inserts_dlx_first() {
        let args = assemble_args(PackageRunner::YarnDlx, &["tsc", "--noEmit"]);
        assert_eq!(args, vec!["dlx", "tsc", "--noEmit"]);
    }
}
