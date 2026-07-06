use crate::methods::recall_integrity::{validate_recalled_row, ValidityCheck};

#[test]
fn valid_row_matching_project_and_category() {
    let result = validate_recalled_row("my-project", "my-project", "decision");
    assert_eq!(
        result,
        ValidityCheck::Safe,
        "matching project and valid category must pass"
    );
}

#[test]
fn valid_row_all_categories() {
    let valid_categories = ["decision", "research", "pattern", "proposal", "roadmap", "app_spec"];
    for cat in &valid_categories {
        let result = validate_recalled_row("proj-a", "proj-a", cat);
        assert_eq!(
            result, ValidityCheck::Safe,
            "category '{}' should be valid",
            cat
        );
    }
}

#[test]
fn reject_foreign_project() {
    let result = validate_recalled_row("expected-project", "poisoned-project", "decision");
    assert_eq!(
        result,
        ValidityCheck::ForeignProject,
        "mismatched project (ASI06) must be rejected"
    );
}

#[test]
fn reject_malformed_category() {
    let result = validate_recalled_row("my-project", "my-project", "hacked");
    assert_eq!(
        result,
        ValidityCheck::MalformedCategory,
        "unknown category must be rejected"
    );
}

#[test]
fn reject_empty_category() {
    let result = validate_recalled_row("my-project", "my-project", "");
    assert_eq!(
        result,
        ValidityCheck::MalformedCategory,
        "empty category must be rejected"
    );
}
