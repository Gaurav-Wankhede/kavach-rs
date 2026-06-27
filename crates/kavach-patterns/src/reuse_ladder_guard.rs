// YAGNI reuse-ladder advisory: nudge a write that adds a new pub symbol to climb the ladder first.
use std::collections::BTreeSet;
const KINDS: &[&str] = &[
    "pub fn ",
    "pub struct ",
    "pub enum ",
    "pub trait ",
    "pub const ",
    "pub type ",
];
fn is_rust(path: &str) -> bool {
    path.to_lowercase().ends_with(".rs")
}
/// The public symbol name declared on a `pub <kind> Name` line, if any.
fn pub_symbol(line: &str) -> Option<String> {
    let t = line.trim_start();
    let kind = KINDS.iter().find(|k| t.starts_with(**k))?;
    let rest = t.get(kind.len()..)?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}
fn pub_symbols(content: &str) -> BTreeSet<String> {
    content.lines().filter_map(pub_symbol).collect()
}
/// Advisory (P1) listing public symbols this write ADDS (absent from `old`) with
/// the reuse-ladder. `None` when no new symbol, non-Rust, or a test file.
#[must_use]
pub fn advise(file_path: &str, old: &str, new: &str) -> Option<String> {
    if !is_rust(file_path) || crate::is_test_file(file_path) {
        return None;
    }
    let before = pub_symbols(old);
    let added: Vec<String> = pub_symbols(new)
        .into_iter()
        .filter(|s| !before.contains(s))
        .collect();
    if added.is_empty() {
        return None;
    }
    let mut msg = format!(
        "[REUSE_LADDER] {file_path} adds public symbol(s): {}. Climb the ladder FIRST: \
         does this need to exist? reuse an existing one (`rg`/`fd`/`ast-grep` for the name)? \
         stdlib/dep already do it? one line? Write the minimum — no abstraction for one caller.\n",
        added.join(", "),
    );
    msg.push_str("  proof: search the tree for each name before adding it.\n");
    Some(msg)
}
#[cfg(test)]
#[path = "reuse_ladder_guard_test.rs"]
#[cfg(test)]
mod tests;
