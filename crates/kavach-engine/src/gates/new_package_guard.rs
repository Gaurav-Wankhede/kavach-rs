//! Detect new package/crate creation that may indicate parallel system anti-pattern.
//!
//! When the AI creates a new Cargo.toml, package.json, or mod.rs in a new directory,
//! it's often building a parallel system instead of extending existing code.
//! Returns a context warning injected into the pre-write allow response.

use crate::gates::directive_cache::dyn_directive;

/// Package manifest filenames that signal new package creation.
const MANIFESTS: &[&str] = &["Cargo.toml", "package.json", "pyproject.toml", "go.mod"];

/// Check if the file being written is a new package manifest.
/// Returns Some(warning) if a new package is being created.
pub(crate) fn check_new_package(file_path: &str) -> Option<String> {
    let fname = file_path.rsplit('/').next()?;
    if !MANIFESTS.contains(&fname) {
        return None;
    }
    // If the manifest already exists on disk, this is an edit — not a new package
    if std::path::Path::new(file_path).exists() {
        return None;
    }
    // Tag + manifest data literal; the anti-parallel-system imperative is research-refreshed.
    let imperative = dyn_directive(
        "new-package.parallel-system-check",
        "STOP AND VERIFY:\n\
         1. Does an existing package already handle this functionality?\n\
         2. Should you EXTEND an existing crate instead of creating a new one?\n\
         3. Will this create a PARALLEL SYSTEM alongside existing code?\n\
         4. If replacing old packages, are you REMOVING old route mounts?\n\
         Extend an existing package instead of duplicating functionality. \
         Remove old route mounts when adding new ones — never mount both in parallel.",
    );
    Some(format!(
        "[NEW_PACKAGE_WARNING]\n\
         Creating NEW package manifest: {fname}\n\
         path: {file_path}\n\n\
         {imperative}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_manifest_passes() {
        assert!(check_new_package("/src/main.rs").is_none());
        assert!(check_new_package("/src/routes.rs").is_none());
    }

    #[test]
    fn existing_manifest_passes() {
        // This file exists on disk — editing, not creating
        assert!(check_new_package("Cargo.toml").is_none());
    }

    #[test]
    fn new_manifest_warns() {
        let r = check_new_package("/tmp/nonexistent-crate/Cargo.toml");
        assert!(r.is_some());
        assert!(r.unwrap().contains("NEW_PACKAGE_WARNING"));
    }

    #[test]
    fn new_package_json_warns() {
        let r = check_new_package("/tmp/nonexistent-pkg/package.json");
        assert!(r.is_some());
    }
}
