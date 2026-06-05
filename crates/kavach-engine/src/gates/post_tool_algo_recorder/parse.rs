//! Parse the structured `// ALGO:` comment block into typed fields.
use super::datetime::{current_month, current_year};

/// Parsed fields from the structured `// ALGO:` comment block.
pub(super) struct AlgoComment {
    pub(super) chosen: String,
    pub(super) problem_class: String,
    pub(super) time_complexity: String,
    pub(super) space_complexity: String,
    pub(super) year_published: i64,
    pub(super) search_year: i64,
    pub(super) search_month: i64,
    pub(super) benchmark_source: Option<String>,
}

/// Extract the `// ALGO:` structured comment block. `None` if no valid block.
pub(super) fn extract_algo_comment(content: &str) -> Option<AlgoComment> {
    // Required fields — abort if any are missing.
    let chosen = extract_field(content, "ALGO:")?;
    let problem_class = extract_field(content, "PROBLEM_CLASS:")?;

    // Optional fields. TIME/SPACE format: "O(n log n) | SPACE: O(log n)" — take
    // only the part before "|".
    let time_complexity = before_pipe(content, "TIME:");
    let space_complexity = before_pipe(content, "SPACE:");

    // YEAR may carry an inline SEARCHED suffix: "2021 | SEARCHED: 2026-04".
    let (year_published, inline_searched) =
        extract_field(content, "YEAR:").map_or((0, None), |v| {
            let mut parts = v.splitn(2, '|');
            let year_raw = parts
                .next()
                .map_or_else(|| v.clone(), |s| s.trim().to_owned());
            let year: i64 = year_raw.parse().unwrap_or_default();
            let searched = parts.next().and_then(|suffix| {
                suffix
                    .trim()
                    .strip_prefix("SEARCHED:")
                    .map(|s| s.trim().to_owned())
            });
            (year, searched)
        });
    let benchmark_source = extract_field(content, "BENCHMARK:");

    // Standalone "// SEARCHED:" line takes precedence; fall back to inline suffix.
    let searched_raw = extract_field(content, "SEARCHED:").or(inline_searched);
    let (search_year, search_month) = searched_raw.map_or_else(
        || (current_year(), current_month()),
        |v| {
            let mut parts = v.splitn(2, '-');
            let y = parts
                .next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or_else(current_year);
            let m = parts
                .next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or_else(current_month);
            (y, m)
        },
    );

    Some(AlgoComment {
        chosen,
        problem_class,
        time_complexity,
        space_complexity,
        year_published,
        search_year,
        search_month,
        benchmark_source,
    })
}

/// Field value truncated at the first `|`, defaulting to "unknown".
fn before_pipe(content: &str, key: &str) -> String {
    extract_field(content, key).map_or_else(
        || "unknown".into(),
        |v| {
            v.split('|')
                .next()
                .map_or_else(|| "unknown".into(), |s| s.trim().to_owned())
        },
    )
}

/// Extract the value after `// <field_key> ` on any line in the content.
pub(super) fn extract_field(content: &str, field_key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("//") {
            continue;
        }
        let after_slashes = trimmed.trim_start_matches('/').trim();
        if let Some(rest) = after_slashes.strip_prefix(field_key) {
            let value = rest.trim().to_owned();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}
