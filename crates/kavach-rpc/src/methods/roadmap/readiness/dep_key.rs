/// Check if a single dependency key is satisfied.
#[must_use]
pub fn dep_key_satisfied(dep_key: &str, all: &[kavach_surreal::MemoryEntry]) -> bool {
    all.iter()
        .find(|e| e.entry_key == dep_key)
        .is_some_and(|e| matches!(e.entry_status_str(), "verified" | "done"))
}

/// Parse `BLOCKED_BY:` / `DEPENDS_ON:` declarations from a card's content.
///
/// Convention: a line whose trimmed form starts with `BLOCKED_BY:` or
/// `DEPENDS_ON:`, followed by comma- or whitespace-separated keys, OR a
/// following indented `- key` bullet list. Tolerant: a card with no such
/// line yields an empty Vec.
#[must_use]
pub fn parse_declared_deps(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_dep_block = false;
    for raw in content.lines() {
        let line = raw.trim();
        let header = line
            .strip_prefix("BLOCKED_BY:")
            .or_else(|| line.strip_prefix("DEPENDS_ON:"));
        if let Some(rest) = header {
            in_dep_block = true;
            for tok in rest.split([',', ' ', '\t']) {
                let key = tok.trim();
                if !key.is_empty() {
                    deps.push(key.to_owned());
                }
            }
            continue;
        }
        if in_dep_block {
            if let Some(bullet) = line.strip_prefix("- ") {
                if let Some(key) = bullet.split_whitespace().next()
                    && !key.is_empty()
                {
                    deps.push(key.to_owned());
                }
                continue;
            }
            if !line.is_empty() {
                in_dep_block = false;
            }
        }
    }
    deps
}
