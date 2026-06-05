//! `kavach toolbelt install` — provision the Rust CLI toolbelt that the gates
//! enforce (rg over grep, bat over cat, fd over find, …) so the tools ship
//! *with* kavach rather than being installed separately on each machine.
//!
//! DELIVERY MODEL (decision: arch.decision.toolbelt-binstall-subcommand):
//! CI ships kavach lean; this subcommand fetches each tool's *prebuilt* release
//! binary via `cargo binstall` (seconds, no source compile) into the user's
//! cargo bin dir, which is already on PATH for a `cargo install kavach` user.
//! No redistribution = no bundled-license obligation; provenance is surfaced
//! per-tool (`--list`) so every binary's upstream + license is auditable.
//!
//! RESEARCH: cargo-binstall resolves every crate below to a GitHub-release or
//! `QuickInstall` prebuilt artifact across linux/macos/windows × x64/aarch64
//! (dry-run verified 2026-06 for the seven riskiest: eza, erdtree, du-dust,
//! ast-grep, git-delta, watchexec-cli, rnr).

use std::process::Command;

use crate::cmd::io_safe;

/// One toolbelt entry: the command name the gates expect, the crate that
/// provides it (binstall target), and the upstream license for provenance.
struct Tool {
    /// Binary name on PATH (what a gate advisory tells the user to run).
    bin: &'static str,
    /// crates.io package name passed to `cargo binstall`.
    krate: &'static str,
    /// SPDX license identifier of the upstream crate.
    license: &'static str,
}

/// Canonical toolbelt — mirrors the `toolbelt` skill's mapping. `bun` is
/// intentionally excluded (Node runtime, not a cargo crate / not binstallable).
const TOOLBELT: &[Tool] = &[
    Tool {
        bin: "rg",
        krate: "ripgrep",
        license: "MIT OR Unlicense",
    },
    Tool {
        bin: "fd",
        krate: "fd-find",
        license: "MIT OR Apache-2.0",
    },
    Tool {
        bin: "bat",
        krate: "bat",
        license: "MIT OR Apache-2.0",
    },
    Tool {
        bin: "eza",
        krate: "eza",
        license: "MIT",
    },
    Tool {
        bin: "erd",
        krate: "erdtree",
        license: "MIT",
    },
    Tool {
        bin: "sd",
        krate: "sd",
        license: "MIT",
    },
    Tool {
        bin: "sg",
        krate: "ast-grep",
        license: "MIT",
    },
    Tool {
        bin: "rnr",
        krate: "rnr",
        license: "MIT",
    },
    Tool {
        bin: "difft",
        krate: "difftastic",
        license: "MIT",
    },
    Tool {
        bin: "delta",
        krate: "git-delta",
        license: "MIT",
    },
    Tool {
        bin: "tokei",
        krate: "tokei",
        license: "MIT OR Apache-2.0",
    },
    Tool {
        bin: "just",
        krate: "just",
        license: "CC0-1.0",
    },
    Tool {
        bin: "watchexec",
        krate: "watchexec-cli",
        license: "Apache-2.0",
    },
    Tool {
        bin: "hyperfine",
        krate: "hyperfine",
        license: "MIT OR Apache-2.0",
    },
    Tool {
        bin: "jaq",
        krate: "jaq",
        license: "MIT",
    },
    Tool {
        bin: "gron",
        krate: "gron",
        license: "MIT",
    },
    Tool {
        bin: "dasel",
        krate: "dasel",
        license: "MIT",
    },
    Tool {
        bin: "dust",
        krate: "du-dust",
        license: "MIT",
    },
    Tool {
        bin: "procs",
        krate: "procs",
        license: "MIT",
    },
    Tool {
        bin: "xh",
        krate: "xh",
        license: "MIT",
    },
    Tool {
        bin: "atuin",
        krate: "atuin",
        license: "MIT",
    },
];

/// Dispatch entry for `kavach toolbelt <action>`.
pub(crate) fn run(action: crate::cli::ToolbeltAction) -> i32 {
    use crate::cli::ToolbeltAction;
    match action {
        ToolbeltAction::Install { yes, only } => install(yes, only.as_deref()),
        ToolbeltAction::List => list(),
    }
}

/// Print every toolbelt tool with its providing crate + upstream license, so a
/// user can audit provenance before installing (honors the spirit of the
/// license-collection requirement without redistributing any binary).
fn list() -> i32 {
    if let Err(e) = io_safe::print_or_exit(
        "Kavach toolbelt — Rust CLI tools the gates enforce (provider crate · license):",
    ) {
        return io_safe::into_exit_code(e);
    }
    for t in TOOLBELT {
        let line = format!("  {:<10} {:<16} {}", t.bin, t.krate, t.license);
        if let Err(e) = io_safe::print_or_exit(&line) {
            return io_safe::into_exit_code(e);
        }
    }
    0
}

/// Install the toolbelt via `cargo binstall`. With `--only a,b,c`, restrict to
/// the named binaries (matched on `bin`). `--yes` passes `--no-confirm`.
fn install(yes: bool, only: Option<&str>) -> i32 {
    if !binstall_present() {
        return missing_binstall();
    }

    let wanted: Vec<&Tool> = match only {
        None => TOOLBELT.iter().collect(),
        Some(csv) => {
            let names: Vec<&str> = csv
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            let selected: Vec<&Tool> = TOOLBELT.iter().filter(|t| names.contains(&t.bin)).collect();
            let unknown: Vec<&str> = names
                .iter()
                .copied()
                .filter(|n| !TOOLBELT.iter().any(|t| t.bin == *n))
                .collect();
            if !unknown.is_empty() {
                let msg = format!(
                    "kavach: unknown toolbelt tool(s): {} — run `kavach toolbelt list`",
                    unknown.join(", ")
                );
                if let Err(e) = io_safe::ewrite_or_exit(&msg) {
                    return io_safe::into_exit_code(e);
                }
                return 1;
            }
            selected
        }
    };

    let crates: Vec<&str> = wanted.iter().map(|t| t.krate).collect();
    let banner = format!(
        "kavach: installing {} toolbelt tool(s) via cargo binstall: {}",
        crates.len(),
        crates.join(" ")
    );
    if let Err(e) = io_safe::print_or_exit(&banner) {
        return io_safe::into_exit_code(e);
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("binstall");
    if yes {
        cmd.arg("--no-confirm");
    }
    cmd.args(&crates);

    match cmd.status() {
        Ok(status) if status.success() => {
            if let Err(e) = io_safe::print_or_exit("kavach: toolbelt install complete.") {
                return io_safe::into_exit_code(e);
            }
            0
        }
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            let msg = format!("kavach: cargo binstall failed (exit {code})");
            if let Err(e) = io_safe::ewrite_or_exit(&msg) {
                return io_safe::into_exit_code(e);
            }
            // Propagate a non-zero so CI / scripts see the failure.
            if code == 0 { 1 } else { code }
        }
        Err(e) => spawn_error(&e),
    }
}

/// True if `cargo binstall --version` runs. We probe rather than parse so a
/// shimmed/aliased binstall still counts.
fn binstall_present() -> bool {
    Command::new("cargo")
        .args(["binstall", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// `cargo binstall` is absent — emit an actionable install hint, fail closed.
fn missing_binstall() -> i32 {
    let hint = "kavach: cargo-binstall not found. Install it first:\n  \
        cargo install cargo-binstall\nor see https://github.com/cargo-bins/cargo-binstall#installation";
    if let Err(e) = io_safe::ewrite_or_exit(hint) {
        return io_safe::into_exit_code(e);
    }
    1
}

/// `cargo` itself could not be spawned (not on PATH). Surface the OS error and
/// exit with the POSIX I/O-error code so callers see a non-zero, classifiable
/// failure rather than a swallowed spawn error.
fn spawn_error(e: &std::io::Error) -> i32 {
    let msg =
        format!("kavach: could not run `cargo` ({e}). Install a Rust toolchain: https://rustup.rs");
    if let Err(io_err) = io_safe::ewrite_or_exit(&msg) {
        return io_safe::into_exit_code(io_err);
    }
    io_safe::EX_IOERR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbelt_has_no_duplicate_bins() {
        let mut seen = std::collections::HashSet::new();
        for t in TOOLBELT {
            assert!(seen.insert(t.bin), "duplicate toolbelt bin: {}", t.bin);
        }
    }

    #[test]
    fn every_tool_has_crate_and_license() {
        for t in TOOLBELT {
            assert!(!t.krate.is_empty(), "{} missing crate", t.bin);
            assert!(!t.license.is_empty(), "{} missing license", t.bin);
        }
    }

    #[test]
    fn bun_is_excluded() {
        assert!(
            !TOOLBELT.iter().any(|t| t.bin == "bun"),
            "bun is a Node runtime — not binstallable, must stay excluded"
        );
    }

    #[test]
    fn list_runs_clean() {
        assert_eq!(list(), 0);
    }
}
