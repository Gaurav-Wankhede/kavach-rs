#[test]
fn audit_module_exists() {
    let _f = cmd::audit::Finding {
        lens: cmd::audit::Lens::Yagni,
        detector: "test".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        severity: cmd::audit::Severity::Warn,
        hint: "test".to_string(),
        fix: "test".to_string(),
    };
}
