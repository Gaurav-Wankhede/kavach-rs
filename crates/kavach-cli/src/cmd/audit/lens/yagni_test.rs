#[test]
fn double_clone_detected() {
    let findings = super::scan("test.rs", "x.clone().clone()");
    assert!(!findings.is_empty());
    assert_eq!(findings[0].hint, "double clone");
}

#[test]
fn dead_code_allow_detected() {
    let findings = super::scan("test.rs", "#[allow(dead_code)]");
    assert!(!findings.is_empty());
    assert_eq!(findings[0].hint, "dead-code allow");
}

#[test]
fn clean_code_no_findings() {
    let findings = super::scan("test.rs", "let x = 5;");
    assert!(findings.is_empty());
}
