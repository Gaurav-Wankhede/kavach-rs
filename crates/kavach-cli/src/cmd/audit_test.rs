use super::audit::{Finding, Lens, Severity};

#[test]
fn audit_module_exports() {
    let _f = Finding {
        lens: Lens::Yagni,
        detector: "test".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        severity: Severity::Warn,
        hint: "test".to_string(),
        fix: "test".to_string(),
    };
}
