//! Content-reference extraction: `[[wikilinks]]` + `INVOKE skill` directives,
//! file-extension→skill mapping, and the qualified-name builder. Shared by the
//! file-write and memory-entry graph projections.

/// Map file extension to skill name for graph edges.
pub(super) fn skill_for_file(path: &str) -> &'static str {
    let p = std::path::Path::new(path);
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "sql" => "sql",
        "jsx" | "astro" => "frontend",
        "css" => "css",
        "toml" | "yaml" | "yml" => "config",
        "md" => "docs",
        _ => "",
    }
}

/// Extract referenced names from file content: `[[wikilinks]]` and
/// `INVOKE skill-name` directives, sorted + deduped.
//
// TIME: O(n log n) | SPACE: O(1) extra
// YEAR: 2026 | SEARCHED: 2026-05
pub(super) fn extract_content_references(content: &str) -> Vec<String> {
    let mut refs: Vec<String> = Vec::new();
    for line in content.lines() {
        collect_wikilinks(line, &mut refs);
        collect_invoke(line, &mut refs);
    }
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn collect_wikilinks(line: &str, refs: &mut Vec<String>) {
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        let Some(after_open) = rest.get(start.saturating_add(2)..) else {
            break;
        };
        let Some(end) = after_open.find("]]") else {
            break;
        };
        if let Some(name) = after_open.get(..end) {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                refs.push(trimmed.to_owned());
            }
        }
        let Some(s) = after_open.get(end.saturating_add(2)..) else {
            break;
        };
        rest = s;
    }
}

const INVOKE_PREFIX: &str = "INVOKE ";

fn collect_invoke(line: &str, refs: &mut Vec<String>) {
    let Some(pos) = line.find(INVOKE_PREFIX) else {
        return;
    };
    let offset = pos.saturating_add(INVOKE_PREFIX.len());
    let Some(after) = line.get(offset..) else {
        return;
    };
    let Some(skill_raw) = after.split_whitespace().next() else {
        return;
    };
    let skill = skill_raw.trim_end_matches('.');
    if !skill.is_empty() {
        refs.push(skill.to_owned());
    }
}

/// Build the legacy namespaced name for a memory entry:
/// `<project>/<category>/<entry_key>`. Public so kavach-cli and any other graph
/// reader can construct lookup keys that match what projections write.
#[must_use]
pub fn memory_entry_qualified_name(category: &str, entry_key: &str, project_slug: &str) -> String {
    if project_slug.is_empty() {
        entry_key.to_owned()
    } else {
        format!("{project_slug}/{category}/{entry_key}")
    }
}

/// Pure-fn parser exposed for kavach-cli's inline projection. Both the CLI
/// direct-DB variant and the engine RPC variant share this as the single source
/// of truth for `[[wikilink]]` + `INVOKE` markers.
#[must_use]
pub fn extract_memory_entry_references(content: &str) -> Vec<String> {
    extract_content_references(content)
}
#[cfg(test)]
#[path = "refs_test.rs"]
#[cfg(test)]
#[path = "refs_test.rs"]
mod tests;