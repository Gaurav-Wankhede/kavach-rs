//! Opportunistic precision: suggest a sharper tool only if it's already on PATH.

use std::path::Path;
use std::process::Command;

/// Hint at a precise resolver IF one is installed; `None` when none is available.
pub(super) fn tool_hint(root: &Path) -> Option<String> {
    let cargo = root.join("Cargo.toml").exists();
    if cargo && on_path("rust-analyzer") {
        return Some("hint: rust-analyzer is installed — for exact go-to-definition open the file in your editor's LSP.".to_owned());
    }
    if on_path("sg") {
        return Some("hint: ast-grep (sg) is installed — `sg -p '<decl pattern>'` can confirm the AST shape.".to_owned());
    }
    None
}

fn on_path(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(test)]
#[path = "refine_test.rs"]
mod refine_test;
