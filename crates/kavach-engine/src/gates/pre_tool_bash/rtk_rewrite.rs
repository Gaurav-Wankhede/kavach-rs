//! Transparent Bash command rewriting via internal RTK subsystem & Rust toolbelt.
use std::process::Command;

use super::toolbelt_rewrite::{rewrite_single_toolbelt, split_pipeline};

#[cfg(test)]
#[path = "rtk_rewrite_test.rs"]
mod tests;

/// Return path to RTK binary if available on the system.
fn rtk_binary() -> Option<&'static str> {
    if cfg!(test) {
        return Some("rtk");
    }
    if std::path::Path::new("/opt/homebrew/bin/rtk").exists() {
        Some("/opt/homebrew/bin/rtk")
    } else if which::which("rtk").is_ok() {
        Some("rtk")
    } else {
        None
    }
}

/// Rewrite command to prevent token waste and secret exposure.
#[must_use]
pub(crate) fn check_rewrite(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() || trimmed.starts_with("rtk ") || trimmed == "rtk" {
        return None;
    }

    let has_rtk = rtk_binary().is_some();
    if !has_rtk {
        return super::toolbelt_rewrite::rewrite_toolbelt(trimmed);
    }

    // In-process pipeline rewriting: handles single commands, pipes, and compound operators.
    let segments = split_pipeline(trimmed);
    let mut changed = false;
    let mut reconstructed = String::with_capacity(trimmed.len().saturating_add(16));

    for (segment, sep) in segments {
        if let Some(rewritten) =
            rewrite_single_rtk(segment, has_rtk).or_else(|| rewrite_single_toolbelt(segment))
        {
            changed = true;
            reconstructed.push_str(&rewritten);
        } else {
            reconstructed.push_str(segment);
        }
        if let Some(s) = sep {
            reconstructed.push_str(s);
        }
    }

    if changed {
        return Some(reconstructed);
    }

    // Fallback: query external RTK binary if available for complex non-standard commands.
    if let Some(bin) = rtk_binary()
        && let Some(rewritten) = run_external_rtk_rewrite(bin, trimmed)
    {
        return Some(rewritten);
    }

    None
}

/// In-process deterministic rewriting for a single command segment.
fn rewrite_single_rtk(cmd: &str, has_rtk: bool) -> Option<String> {
    let trimmed = cmd.trim_start();
    if trimmed.is_empty() || trimmed.starts_with("rtk ") || trimmed == "rtk" {
        return None;
    }
    let leading_ws = cmd
        .get(..cmd.len().saturating_sub(trimmed.len()))
        .unwrap_or("");

    // Map raw environment inspections to rtk env to avoid secret dumps.
    if trimmed == "env" || trimmed == "printenv" {
        return Some(format!("{leading_ws}rtk env"));
    }
    if let Some(rest) = trimmed.strip_prefix("printenv ") {
        let var = rest.trim();
        if !var.is_empty() && !var.contains(' ') {
            return Some(format!("{leading_ws}rtk env -f {var}"));
        }
    }

    // In-process rules for heavy developer tools when RTK is active.
    if has_rtk {
        // Cargo subcommands.
        if let Some(rest) = strip_command_prefix(trimmed, "cargo") {
            let sub = rest.trim_start();
            if sub.starts_with("test")
                || sub.starts_with("check")
                || sub.starts_with("clippy")
                || sub.starts_with("build")
            {
                return Some(format!("{leading_ws}rtk cargo{rest}"));
            }
        }

        // Git subcommands.
        if let Some(rest) = strip_command_prefix(trimmed, "git") {
            let sub = rest.trim_start();
            if sub.starts_with("status")
                || sub.starts_with("diff")
                || sub.starts_with("log")
                || sub.starts_with("add")
            {
                return Some(format!("{leading_ws}rtk git{rest}"));
            }
        }

        // Test runners.
        if trimmed.starts_with("npm test")
            || trimmed.starts_with("pnpm test")
            || trimmed.starts_with("yarn test")
            || trimmed.starts_with("bun test")
        {
            return Some(format!("{leading_ws}rtk test"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "pytest") {
            return Some(format!("{leading_ws}rtk pytest{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "vitest") {
            return Some(format!("{leading_ws}rtk vitest{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "jest") {
            return Some(format!("{leading_ws}rtk jest{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "go") {
            let sub = rest.trim_start();
            if sub.starts_with("test") {
                return Some(format!("{leading_ws}rtk go{rest}"));
            }
        }

        // Toolbelt tools with RTK-native compression.
        if let Some(rest) = strip_command_prefix(trimmed, "find") {
            return Some(format!("{leading_ws}rtk find{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "ls") {
            return Some(format!("{leading_ws}rtk ls{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "tree") {
            return Some(format!("{leading_ws}rtk tree{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "diff") {
            return Some(format!("{leading_ws}rtk diff{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "curl") {
            return Some(format!("{leading_ws}rtk curl{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "grep") {
            return Some(format!("{leading_ws}rtk grep{rest}"));
        }
        if let Some(rest) = strip_command_prefix(trimmed, "rg") {
            return Some(format!("{leading_ws}rtk rg{rest}"));
        }
    }

    None
}

/// Check if a command starts with the given command word.
fn strip_command_prefix<'a>(cmd: &'a str, word: &str) -> Option<&'a str> {
    if let Some(rest) = cmd.strip_prefix(word)
        && (rest.is_empty() || rest.starts_with(char::is_whitespace))
    {
        return Some(rest);
    }
    None
}

/// Run external RTK binary rewrite as last-mile fallback.
fn run_external_rtk_rewrite(bin: &str, trimmed: &str) -> Option<String> {
    let output = Command::new(bin)
        .arg("rewrite")
        .arg(trimmed)
        .output()
        .ok()?;

    let code = output.status.code().unwrap_or(1);
    // RTK rewrite exits 0 or 3 on successful transformation.
    if code == 0 || code == 3 {
        let rewritten = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !rewritten.is_empty() && rewritten != trimmed {
            return Some(rewritten);
        }
    }
    None
}
