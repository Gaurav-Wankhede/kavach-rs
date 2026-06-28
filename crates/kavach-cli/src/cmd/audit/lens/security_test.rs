#[test]
fn detects_loophole_patterns() {
    let findings = super::scan("test.rs", "unwrap()");
    assert!(!findings.is_empty());
}

#[test]
fn clean_code_no_findings() {
    let findings = super::scan("test.rs", "let x = 5;");
    assert!(findings.is_empty());
}
