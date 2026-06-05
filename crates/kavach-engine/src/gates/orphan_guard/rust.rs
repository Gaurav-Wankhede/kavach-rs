//! Rust orphan detection: file not declared in parent `mod.rs`, plus a
//! wire-check nudge for new `pub` exports.
use std::path::Path;

/// Check if a `.rs` file is declared in its parent `mod.rs` and flag new pub exports.
pub(super) fn check_rust_orphan(file_path: &str, content: &str) -> Option<String> {
    let path = Path::new(file_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem == "mod" || stem == "lib" || stem == "main" {
        return None;
    }
    let mut warnings: Vec<String> = Vec::new();

    if let Some(parent) = path.parent() {
        let mod_rs = parent.join("mod.rs");
        if mod_rs.exists()
            && let Ok(mod_content) = std::fs::read_to_string(&mod_rs)
            && !mod_content.contains(&format!("mod {stem}"))
        {
            warnings.push(format!(
                "ORPHAN: `{stem}.rs` not declared in `mod.rs`\n\
                 FIX: Add `pub mod {stem};` or `pub(crate) mod {stem};` to {}/mod.rs",
                parent.display()
            ));
        }
    }

    let pub_count = count_pub_exports(content);
    if pub_count > 0 {
        warnings.push(format!(
            "WIRE_CHECK: {pub_count} pub export(s) in `{stem}.rs`\n\
             ACTION: Ensure each is imported and used at its call site.\n\
             Rule: Create + Import + Use in the SAME turn."
        ));
    }
    if warnings.is_empty() {
        return None;
    }
    Some(format!(
        "[ORPHAN_GUARD]\nfile: {file_path}\n\n{}",
        warnings.join("\n\n")
    ))
}

/// Count `pub`/`pub(crate)` fn/struct/enum/type/const declarations.
pub(super) fn count_pub_exports(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let t = line.trim();
            (t.starts_with("pub fn ")
                || t.starts_with("pub struct ")
                || t.starts_with("pub enum ")
                || t.starts_with("pub type ")
                || t.starts_with("pub const ")
                || t.starts_with("pub(crate) fn "))
                && !t.contains("#[test]")
        })
        .count()
}
