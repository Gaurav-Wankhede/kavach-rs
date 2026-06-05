use super::*;
use std::collections::HashMap;

#[test]
fn test_aegis_dangerous_command() {
    let mut input = HashMap::new();
    input.insert(
        "command".into(),
        serde_json::Value::String("rm -rf /".into()),
    );
    let v = aegis_verify(None, "Bash", &input);
    assert!(!v.passed);
    assert_eq!(v.threat_level, "high");
}

#[test]
fn test_aegis_sensitive_path() {
    let mut input = HashMap::new();
    input.insert(
        "file_path".into(),
        serde_json::Value::String("/etc/shadow".into()),
    );
    let v = aegis_verify(None, "Read", &input);
    assert!(!v.passed);
}

#[test]
fn test_aegis_pass() {
    let mut input = HashMap::new();
    input.insert(
        "file_path".into(),
        serde_json::Value::String("src/main.rs".into()),
    );
    let v = aegis_verify(None, "Read", &input);
    assert!(v.passed);
    assert!((v.security_score - 1.0).abs() < f64::EPSILON);
}
