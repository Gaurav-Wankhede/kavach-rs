//! Pattern collection and compilation.

use super::types::{PatternCategory, PatternMatch, Severity};
use std::sync::LazyLock;

pub(super) struct CategoryPatterns {
    pub(super) category: PatternCategory,
    pub(super) patterns: Vec<(regex::Regex, &'static str, &'static str, Severity)>,
}

/// Scan `content` against every category pattern, optionally filtered to a
/// single `category`. Results are sorted by ascending severity. This is the one
/// scan loop; `scanner_all` and `scanner_category` are thin filters over it.
pub(super) fn scan_filtered(content: &str, category: Option<PatternCategory>) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    for cat in ALL_PATTERNS.iter() {
        if category.is_some_and(|want| cat.category != want) {
            continue;
        }
        for (regex, code, message, severity) in &cat.patterns {
            for m in regex.find_iter(content) {
                let line = content
                    .get(..m.start())
                    .map_or(0, |slice| slice.lines().count())
                    .saturating_add(1);
                matches.push(PatternMatch {
                    category: cat.category,
                    code,
                    message,
                    severity: *severity,
                    line,
                    matched: m.as_str().chars().take(100).collect(),
                });
            }
        }
    }
    matches.sort_by_key(|m| m.severity);
    matches
}

pub(super) static ALL_PATTERNS: LazyLock<Vec<CategoryPatterns>> = LazyLock::new(|| {
    vec![
        CategoryPatterns {
            category: PatternCategory::BusinessLogic,
            patterns: super::types::compiled(super::business_logic::build()),
        },
        CategoryPatterns {
            category: PatternCategory::ErrorHandling,
            patterns: super::types::compiled(super::error_handling::build()),
        },
        CategoryPatterns {
            category: PatternCategory::DataValidation,
            patterns: super::types::compiled(super::data_validation::build()),
        },
        CategoryPatterns {
            category: PatternCategory::ApiInteraction,
            patterns: super::types::compiled(super::api_interaction::build()),
        },
        CategoryPatterns {
            category: PatternCategory::Security,
            patterns: super::types::compiled(super::security::build()),
        },
        CategoryPatterns {
            category: PatternCategory::Database,
            patterns: super::types::compiled(super::database::build()),
        },
        CategoryPatterns {
            category: PatternCategory::RowLevelSecurity,
            patterns: super::types::compiled(super::rls::build()),
        },
        CategoryPatterns {
            category: PatternCategory::Proxy,
            patterns: super::types::compiled(super::proxy::build()),
        },
        CategoryPatterns {
            category: PatternCategory::Scalability,
            patterns: super::types::compiled(super::scalability::build()),
        },
        CategoryPatterns {
            category: PatternCategory::SystemDesign,
            patterns: super::types::compiled(super::system_design::build()),
        },
    ]
});
