//! Extract a Rust crate name from a `.../crates/<name>/src/...` file path.

/// Extract Rust crate name from an absolute file path.
/// Looks for `.../crates/<crate-name>/src/...` pattern.
/// Returns None when the pattern is not present (e.g. workspace root files).
pub(super) fn crate_name_from_path(path: &str) -> Option<String> {
    let norm = path.replace('\\', "/");
    let marker = "/crates/";
    let pos = norm.find(marker)?;
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "pos from find() + marker.len() is bounded by string length"
    )]
    let after = norm.get(pos + marker.len()..)?;
    let name = after.split('/').next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}
