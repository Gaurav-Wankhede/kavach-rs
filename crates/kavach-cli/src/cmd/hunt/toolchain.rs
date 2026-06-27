//! rustc + clippy backend: parse `cargo clippy --message-format=json` into Findings.

use super::finding::{Finding, Severity};
use std::path::Path;
use std::process::Command;

/// Run `cargo clippy` over the cargo project at `root`, parsing diagnostics into Findings.
#[must_use]
pub(super) fn run_clippy(root: &Path) -> Vec<Finding> {
    if !root.join("Cargo.toml").exists() {
        return Vec::new();
    }
    let Ok(output) = Command::new("cargo")
        .args([
            "clippy",
            "--message-format=json",
            "--quiet",
            "--",
            "-W",
            "clippy::pedantic",
        ])
        .current_dir(root)
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().filter_map(parse_line).collect()
}

/// Parse one NDJSON line into a `Finding`, or `None` if not a located diagnostic.
fn parse_line(line: &str) -> Option<Finding> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    if v.get("reason")?.as_str()? != "compiler-message" {
        return None;
    }
    let msg = v.get("message")?;
    let level = msg.get("level")?.as_str()?;
    let span = msg.get("spans")?.as_array()?.first()?;
    let file = span.get("file_name")?.as_str()?.to_owned();
    let line_no = usize::try_from(span.get("line_start")?.as_u64()?).unwrap_or(0);
    let code = msg
        .get("code")
        .and_then(|c| c.get("code"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("rustc")
        .to_owned();
    let text = msg.get("message")?.as_str()?.to_owned();
    Some(Finding {
        detector: "clippy",
        file,
        line: line_no,
        severity: severity_of(level),
        category: code,
        snippet: String::new(),
        fix: text,
    })
}

fn severity_of(level: &str) -> Severity {
    match level {
        "error" => Severity::Block,
        "warning" => Severity::Warn,
        _ => Severity::Advisory,
    }
}

#[cfg(test)]
#[path = "toolchain_test.rs"]
mod toolchain_test;
