//! Universal component-monolith detector: >100-line frontend files with 2+
//! exported components should be split one-component-per-file.

const COMPONENT_FILE_LINE_LIMIT: usize = 100;

/// Universal component oversized-file detector for frontend files.
///
/// Triggers on ANY `.tsx/.jsx/.astro` file (not test, not config) when:
/// 1. File exceeds 100 lines
/// 2. Contains 2+ exported component functions
///
/// Escape hatch: `// split:` or `{/* split: */}` to suppress.
pub(crate) fn check_component_oversized(file_path: &str, content: &str) -> Option<String> {
    if !kavach_patterns::is_frontend_file(file_path) || kavach_patterns::is_test_file(file_path) {
        return None;
    }
    if content.lines().count() <= COMPONENT_FILE_LINE_LIMIT {
        return None;
    }
    let lc = content.to_lowercase();
    if lc.contains("// split:") || lc.contains("{/* split:") {
        return None;
    }
    let export_count = count_exported_components(content);
    if export_count <= 1 {
        return None;
    }
    Some(format!(
        "COMPONENT_MONOLITH: File has {export_count} exported components over {} lines. \
         One component per file. Split each into its own file. \
         Add `// split:` comment to suppress if intentional.",
        content.lines().count()
    ))
}

/// Count exported component definitions in frontend code.
fn count_exported_components(content: &str) -> usize {
    let mut count = 0usize;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("export type ") || trimmed.starts_with("export interface ") {
            continue;
        }
        if trimmed.starts_with("export function ")
            || trimmed.starts_with("export default function")
            || trimmed.starts_with("export async function ")
        {
            count = count.saturating_add(1);
            continue;
        }
        // export const ComponentName = (uppercase = component heuristic)
        if trimmed.starts_with("export const ")
            && trimmed.contains(" = ")
            && let Some(rest) = trimmed.strip_prefix("export const ")
            && let Some(c) = rest.chars().next()
            && c.is_uppercase()
        {
            count = count.saturating_add(1);
        }
    }
    count
}
