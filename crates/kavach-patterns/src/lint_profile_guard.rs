//! Nudges `kavach lint init` when a source edit lands in a project missing its
//! strict-rules profile.
//!
//! Detects the stack manifest (Cargo.toml / package.json / go.mod) by walking up
//! from the file; advises only when the strict profile is absent. Fail-soft: no
//! manifest, or one already strict, is silent. SOURCE: decision.lint.language-profile-template.
use std::path::Path;

const SOURCE_EXTS: &[&str] = &[".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".go"];

fn is_source(path: &str) -> bool {
    let p = path.to_lowercase();
    SOURCE_EXTS.iter().any(|e| p.ends_with(e))
}

/// Advisory (P1) when the edited source sits in a project missing its strict
/// profile. `None` when clean, non-source, or no manifest is found upward.
#[must_use]
pub fn advise(file_path: &str) -> Option<String> {
    if !is_source(file_path) || crate::is_test_file(file_path) {
        return None;
    }
    let start = Path::new(file_path).parent()?;
    let (root, manifest, has_strict) = find_project(start)?;
    if has_strict {
        return None;
    }
    Some(format!(
        "[LINT_PROFILE_P1] {root} has a {manifest} but no strict-rules profile — run \
         `kavach lint init` so the build FAILS on bad patterns (no suppression). \
         SOURCE: decision.lint.language-profile-template.\n",
        root = root.display(),
    ))
}

/// Walk up from `start` to the nearest project manifest; return its root dir,
/// the manifest name, and whether a strict profile is already installed.
fn find_project(start: &Path) -> Option<(std::path::PathBuf, &'static str, bool)> {
    let mut probe = Some(start);
    while let Some(dir) = probe {
        if dir.join("Cargo.toml").is_file() {
            return Some((dir.to_path_buf(), "Cargo.toml", rust_is_strict(dir)));
        }
        if dir.join("package.json").is_file() || dir.join("tsconfig.json").is_file() {
            return Some((
                dir.to_path_buf(),
                "package.json",
                dir.join("tsconfig.json").is_file(),
            ));
        }
        if dir.join("go.mod").is_file() {
            return Some((
                dir.to_path_buf(),
                "go.mod",
                dir.join(".golangci.yml").is_file(),
            ));
        }
        probe = dir.parent();
    }
    None
}

/// A Rust project is strict if its Cargo.toml carries a lints table (workspace
/// or per-crate). Read failure = treat as not-strict (advise, never panic).
fn rust_is_strict(dir: &Path) -> bool {
    let toml = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    toml.contains("[workspace.lints") || toml.contains("[lints")
}

#[cfg(test)]
#[path = "lint_profile_guard_test.rs"]
mod tests;
