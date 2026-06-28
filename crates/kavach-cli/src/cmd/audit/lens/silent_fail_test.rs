#[test]
fn detects_silent_io() {
    let findings = super::scan("test.rs", "let _ = writeln!(...)");
    assert!(!findings.is_empty());
}

#[test]
fn clean_code_no_findings() {
    let findings = super::scan("test.rs", "let x = 5;");
    assert!(findings.is_empty());
}
