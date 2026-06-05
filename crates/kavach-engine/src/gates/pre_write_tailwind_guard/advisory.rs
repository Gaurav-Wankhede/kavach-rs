//! Advisory orchestrator: load the Tailwind Plus index, match the write target,
//! and render a `[TAILWIND_PLUS_REF]` reference block (never a hard block).
use std::fs;
use std::path::Path;

use kavach_config::{tailwind_plus_dir, tailwind_plus_index};

use super::keywords::extract_query_keywords;
use super::matching::find_best_match;

/// Returns an advisory context block if a matching Tailwind Plus component is
/// found, or a "no match" nudge. Always `None`-safe (index absent = skip).
pub(crate) fn advisory(file_path: &str, content: &str) -> Option<String> {
    if !is_frontend_file(file_path) {
        return None;
    }
    let index_path = tailwind_plus_index();
    if !index_path.exists() {
        return None;
    }
    let index_json = fs::read_to_string(&index_path).ok()?;
    let index = serde_json::from_str::<serde_json::Value>(&index_json).ok()?;
    let components = index.get("components").and_then(|v| v.as_array())?;
    let query_kw = extract_query_keywords(file_path, content);
    match find_best_match(components, &query_kw) {
        Some((score, component)) if score >= 0.3 => {
            let file_field = component.get("file").and_then(|v| v.as_str())?;
            let preview = read_component_preview(file_field);
            let mut block = format!(
                "[TAILWIND_PLUS_REF]\nmatch: {file_field} (score: {score:.2})\n\
                 action: Use as base structure. Replace colors with semantic tokens. Wire Motion.\n"
            );
            if !preview.is_empty() {
                block.push('\n');
                block.push_str(&preview);
            }
            Some(block)
        }
        _ => Some(
            "[TAILWIND_PLUS_REF]\nmatch: none\n\
             action: Check tailwindcss.com/plus for matching component before writing custom UI\n"
                .to_owned(),
        ),
    }
}

pub(super) fn is_frontend_file(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("tsx")
            || ext.eq_ignore_ascii_case("jsx")
            || ext.eq_ignore_ascii_case("astro")
    })
}

/// Read first 80 lines of a component file from `~/.claude/tailwind-plus/<file>`.
fn read_component_preview(file_field: &str) -> String {
    let path = tailwind_plus_dir().join(file_field);
    fs::read_to_string(&path).map_or_else(
        |_| String::new(),
        |body| {
            let preview: String = body.lines().take(80).collect::<Vec<_>>().join("\n");
            format!("```jsx\n{preview}\n```")
        },
    )
}
