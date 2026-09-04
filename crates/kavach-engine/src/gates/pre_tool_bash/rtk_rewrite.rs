//! Transparent Bash command rewriting via internal RTK subsystem.
use std::process::Command;

#[cfg(test)]
#[path = "rtk_rewrite_test.rs"]
mod tests;

/// Return path to RTK binary if available on the system.
fn rtk_binary() -> Option<&'static str> {
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

    let bin = rtk_binary()?;

    // Map raw environment inspections to rtk env to avoid secret dumps.
    if trimmed == "env" || trimmed == "printenv" {
        return Some("rtk env".to_owned());
    }
    if let Some(rest) = trimmed.strip_prefix("printenv ") {
        let var = rest.trim();
        if !var.is_empty() && !var.contains(' ') {
            return Some(format!("rtk env -f {var}"));
        }
    }

    // Rewrite tool calls to rtk-compressed variants.
    let output = Command::new(bin)
        .arg("rewrite")
        .arg(trimmed)
        .output()
        .ok()?;

    if output.status.success() {
        let rewritten = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !rewritten.is_empty() && rewritten != trimmed {
            return Some(rewritten);
        }
    }

    None
}
