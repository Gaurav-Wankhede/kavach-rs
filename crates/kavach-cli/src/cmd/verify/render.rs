// TIME: O(n) in arg count | SPACE: O(n)
//! Render the resolved cargo command + stderr head so a verify FAIL is diagnosable.
/// `cargo check -p <crate>` (or workspace) as a single display string.
#[must_use]
pub(super) fn cargo_cmd(sub: &[&str], crate_name: Option<&str>) -> String {
    let mut s = String::from("cargo");
    for a in sub {
        s.push(' ');
        s.push_str(a);
    }
    if let Some(name) = crate_name {
        s.push_str(" -p ");
        s.push_str(name);
    }
    s
}
/// First `n` non-empty lines of captured stderr, for an inline FAIL excerpt.
#[must_use]
pub(super) fn stderr_head(stderr: &str, n: usize) -> String {
    stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(n)
        .collect::<Vec<_>>()
        .join("\n")
}
#[cfg(test)]
#[path = "render_test.rs"]
#[path = "render_test.rs"]
mod tests;
