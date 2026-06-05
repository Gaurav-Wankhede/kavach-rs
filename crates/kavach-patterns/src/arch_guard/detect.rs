//! Architecture pattern detection.

use super::triggers::find_matches;
use super::types::ArchFinding;

/// Detect architectural patterns in content.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<ArchFinding> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    // Skip pattern-detection files
    if file_path.contains("kavach-patterns/src/") {
        return vec![];
    }

    let mut findings = Vec::new();

    for (byte_pos, keyword, scope) in find_matches(content) {
        let line_num = content
            .get(..byte_pos)
            .map_or(1, |prefix| prefix.matches('\n').count().saturating_add(1));
        // Skip if overlaps with algo_guard (lru_cache handled there)
        if keyword == "lru_cache" {
            continue;
        }
        findings.push(ArchFinding {
            keyword: keyword.to_owned(),
            scope,
            line: line_num,
        });
    }
    findings
}

/// Check if content has a valid // ARCH: comment.
#[must_use]
pub fn has_arch_comment(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("// ARCH:")
    })
}

/// Count required ARCH comment fields present.
#[must_use]
pub fn count_arch_fields(content: &str) -> usize {
    const REQUIRED: &[&str] = &[
        "// ARCH:",
        "// SCOPE:",
        "// CAP:",
        "// QPS:",
        "// STORAGE:",
        "// FAILURE_MODE:",
        "// TRADEOFF:",
        "// SEARCHED:",
        "// REFERENCE:",
    ];
    REQUIRED.iter().filter(|&f| content.contains(f)).count()
}
