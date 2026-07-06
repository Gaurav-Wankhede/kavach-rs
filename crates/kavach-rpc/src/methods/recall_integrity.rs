// SOURCE: OWASP ASI06 — read-side integrity check for recalled memory rows before injection.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidityCheck {
    Safe,
    ForeignProject,
    MalformedCategory,
}

/// Validate a recalled row: check project matches + category is in allowed set.
#[must_use]
pub fn validate_recalled_row(
    expected_project_slug: &str,
    actual_project_slug: &str,
    category: &str,
) -> ValidityCheck {
    if actual_project_slug != expected_project_slug {
        return ValidityCheck::ForeignProject;
    }

    const VALID_CATEGORIES: &[&str] = &["decision", "research", "pattern", "proposal", "roadmap", "app_spec"];

    if VALID_CATEGORIES.contains(&category) {
        ValidityCheck::Safe
    } else {
        ValidityCheck::MalformedCategory
    }
}

#[cfg(test)]
#[path = "recall_integrity_test.rs"]
mod recall_integrity_test;
