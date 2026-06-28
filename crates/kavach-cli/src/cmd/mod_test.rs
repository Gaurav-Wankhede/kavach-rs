#[test]
fn audit_module_exists() {
    let _f = audit::Finding {
        lens: audit::Lens::Yagni,
        detector: "test".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        severity: audit::Severity::Warn,
        hint: "test".to_string(),
        fix: "test".to_string(),
    };
}
