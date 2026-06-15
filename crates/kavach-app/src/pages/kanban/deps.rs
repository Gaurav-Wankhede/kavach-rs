//! Dependency parsing for the kanban board — the GUI mirror of the CLI's
//! DAG view. A card declares prerequisites via `DEPENDS_ON:` lines (with
//! `BLOCKED_BY:` accepted as a back-compat alias) in its content, the same
//! convention the scheduler and `kavach db kanban` read. This is pure
//! topological ordering; there is no blocked/gate state.

/// Parse `DEPENDS_ON:` keys (and the `BLOCKED_BY:` alias) from a card's content.
/// Mirrors the scheduler's `parse_declared_deps` (kavach-rpc) — kept inline so
/// the WASM-targetable app pulls in no extra crate. Tolerant: no such line -> empty.
pub fn declared_deps(content: &str) -> Vec<&str> {
    let mut deps = Vec::new();
    let mut in_block = false;
    for raw in content.lines() {
        let line = raw.trim();
        if let Some(rest) = line
            .strip_prefix("BLOCKED_BY:")
            .or_else(|| line.strip_prefix("DEPENDS_ON:"))
        {
            in_block = true;
            deps.extend(rest.split([',', ' ', '\t']).map(str::trim).filter(|k| !k.is_empty()));
            continue;
        }
        if in_block {
            if let Some(bullet) = line.strip_prefix("- ") {
                if let Some(key) = bullet.split_whitespace().next() {
                    deps.push(key);
                }
                continue;
            }
            if !line.is_empty() {
                in_block = false;
            }
        }
    }
    deps
}

#[cfg(test)]
#[path = "deps_test.rs"]
mod deps_test;
