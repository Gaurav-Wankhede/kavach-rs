use std::process::Command;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) const CLI_PKG: &str = "kavach-cli";
pub(super) const ENGINE_PKG: &str = "kavach-engine";

/// 8-step CLI deploy: strict gates + build + install + restart daemon (step 8).
#[expect(clippy::too_many_lines, reason = "deploy orchestrator with 8 steps")]
pub(super) fn deploy_cli(skip_tests: bool) -> i32 {
    if let Err(io_err) = print_or_exit("[DEPLOY] step 1/8: cargo check --release -D warnings") {
        return into_exit_code(io_err);
    }

    if !run_cargo_strict(&["check", "--release", "-p", CLI_PKG]) {
        if let Err(io_err) = ewrite_or_exit(
            "[DEPLOY] FAIL: cargo check produced warnings or errors. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 2/8: cargo clippy --release -D warnings") {
        return into_exit_code(io_err);
    }
    if !run_cargo_strict(&[
        "clippy",
        "--release",
        "-p",
        CLI_PKG,
        "-p",
        ENGINE_PKG,
        "--",
        "-D",
        "warnings",
    ]) {
        if let Err(io_err) = ewrite_or_exit(
            "[DEPLOY] FAIL: cargo clippy produced warnings. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit(
        "[DEPLOY] step 3/8: cargo deny check (advisories + bans + licenses + sources)",
    ) {
        return into_exit_code(io_err);
    }
    if Command::new("cargo-deny").arg("--version").output().is_ok() {
        if !run_cargo(&["deny", "check"]) {
            if let Err(io_err) = ewrite_or_exit(
                "[DEPLOY] FAIL: cargo deny check failed. Fix policy violation \
                 (see deny.toml) — do not silently widen the allowlist.",
            ) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    } else if let Err(io_err) = print_or_exit(
        "[DEPLOY] step 3/8: SKIPPED (cargo-deny not installed; \
         install via `cargo binstall cargo-deny`)",
    ) {
        return into_exit_code(io_err);
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 4/8: cargo machete (unused dependencies)") {
        return into_exit_code(io_err);
    }
    if Command::new("cargo-machete")
        .arg("--version")
        .output()
        .is_ok()
    {
        if !run_cargo(&["machete"])
            && let Err(io_err) = ewrite_or_exit(
                "[DEPLOY] WARN: cargo machete found unused dependencies. \
                 Tracked in hunt.cargo-machete-unused-deps-sweep.",
            )
        {
            return into_exit_code(io_err);
        }
    } else if let Err(io_err) = print_or_exit(
        "[DEPLOY] step 4/8: SKIPPED (cargo-machete not installed; \
         install via `cargo binstall cargo-machete`)",
    ) {
        return into_exit_code(io_err);
    }

    if skip_tests {
        if let Err(io_err) = print_or_exit("[DEPLOY] step 5/8: SKIPPED (--skip-tests)") {
            return into_exit_code(io_err);
        }
    } else {
        if let Err(io_err) = print_or_exit("[DEPLOY] step 5/8: cargo nextest run") {
            return into_exit_code(io_err);
        }
        if !run_cargo(&["nextest", "run", "-p", CLI_PKG, "-p", ENGINE_PKG]) {
            if let Err(io_err) = ewrite_or_exit("[DEPLOY] FAIL: cargo nextest failed") {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 6/8: cargo build --release -D warnings") {
        return into_exit_code(io_err);
    }
    if !run_cargo_strict(&["build", "--release", "-p", CLI_PKG]) {
        if let Err(io_err) = ewrite_or_exit(
            "[DEPLOY] FAIL: cargo build produced warnings or errors. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 7/8: install to ~/.local/bin/kavach") {
        return into_exit_code(io_err);
    }
    let Some(root) = super::workspace_root() else {
        if let Err(io_err) = ewrite_or_exit("[DEPLOY] FAIL: cannot resolve cwd") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let src = root
        .join("target")
        .join(super::install::RELEASE_PROFILE)
        .join(super::install::binary_filename());
    let Some(dst) = super::install::install_dest() else {
        if let Err(io_err) = ewrite_or_exit("[DEPLOY] FAIL: cannot resolve $HOME") {
            return into_exit_code(io_err);
        }
        return 1;
    };

    if let Err(msg) = super::install::install_binary(&src, &dst) {
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) =
        print_or_exit("[DEPLOY] step 8/8: no daemon to restart (surreal server owns the DB)")
    {
        return into_exit_code(io_err);
    }

    let ok_msg = format!("[DEPLOY] OK: kavach installed to {}", dst.display());
    if let Err(io_err) = print_or_exit(&ok_msg) {
        return into_exit_code(io_err);
    }
    0
}

/// Run cargo with given args. Returns true on success.
pub(super) fn run_cargo(args: &[&str]) -> bool {
    Command::new("cargo")
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}

/// Run cargo with `RUSTFLAGS=-D warnings` so any lint warning fails the deploy.
pub(super) fn run_cargo_strict(args: &[&str]) -> bool {
    let merged = match std::env::var("RUSTFLAGS") {
        Ok(existing) if existing.is_empty() => "-D warnings".to_owned(),
        Ok(existing) => format!("{existing} -D warnings"),
        Err(std::env::VarError::NotPresent) => "-D warnings".to_owned(),
        Err(std::env::VarError::NotUnicode(_)) => {
            ewrite_or_exit(
                "[DEPLOY] FAIL: RUSTFLAGS contains non-UTF-8 bytes; refusing to override silently. \
                 Unset or fix RUSTFLAGS, then re-run kavach deploy.",
            )
            .ok();
            return false;
        }
    };
    Command::new("cargo")
        .env("RUSTFLAGS", merged)
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}
