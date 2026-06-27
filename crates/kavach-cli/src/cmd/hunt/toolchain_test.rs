use super::{parse_line, severity_of};
use crate::cmd::hunt::finding::Severity;

const SAMPLE: &str = r#"{"reason":"compiler-message","message":{"level":"warning","code":{"code":"clippy::doc_markdown"},"message":"item in documentation is missing backticks","spans":[{"file_name":"src/x.rs","line_start":19}]}}"#;

#[test]
fn parses_clippy_diagnostic_into_finding() {
    let f = parse_line(SAMPLE).expect("a located compiler-message must parse");
    assert_eq!(f.detector, "clippy");
    assert_eq!(f.file, "src/x.rs");
    assert_eq!(f.line, 19);
    assert_eq!(f.category, "clippy::doc_markdown");
    assert_eq!(f.severity, Severity::Warn);
}

#[test]
fn skips_non_diagnostic_lines() {
    assert!(parse_line("   Compiling kavach v0.1.0").is_none());
    assert!(parse_line(r#"{"reason":"build-finished","success":true}"#).is_none());
    assert!(parse_line("").is_none());
}

#[test]
fn maps_error_level_to_block() {
    assert_eq!(severity_of("error"), Severity::Block);
    assert_eq!(severity_of("warning"), Severity::Warn);
    assert_eq!(severity_of("note"), Severity::Advisory);
}
