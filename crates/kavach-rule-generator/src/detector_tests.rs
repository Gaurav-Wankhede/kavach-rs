//! Tests for detector module.

use super::*;

#[test]
fn test_detect_rust_language() {
    let files = vec!["src/main.rs", "src/lib.rs", "Cargo.toml"];
    let results = detect_patterns(&files, "");
    let rust = results.iter().find(|p| p.name == "rust");
    assert!(rust.is_some(), "should detect rust language");
    if let Some(r) = rust {
        assert!(r.confidence > 0.0);
    }
}

#[test]
fn test_detect_axum_framework() {
    let files = vec!["src/main.rs", "Cargo.toml"];
    let content = "use axum::Router;\nlet app = Router::new();";
    let results = detect_patterns(&files, content);
    let axum = results.iter().find(|p| p.name == "axum");
    assert!(axum.is_some(), "should detect axum framework");
    if let Some(a) = axum {
        assert!(a.confidence >= 0.80);
    }
}

#[test]
fn test_detect_empty_files() {
    let results = detect_patterns(&[], "");
    assert!(results.is_empty(), "no patterns from empty input");
}

#[test]
fn test_detect_go_language() {
    let files = vec!["main.go", "handler.go", "go.mod"];
    let results = detect_patterns(&files, "");
    let go = results.iter().find(|p| p.name == "go");
    assert!(go.is_some(), "should detect go language");
}

#[test]
fn test_confidence_ordering() {
    let files = vec!["src/main.rs", "Cargo.toml"];
    let content = "use axum::Router;\nRouter::new();\naxum::extract";
    let results = detect_patterns(&files, content);
    for pair in results.windows(2) {
        assert!(!pair.is_empty(), "pair has 2 elements");
        if let (Some(first), Some(second)) = (pair.first(), pair.last()) {
            assert!(
                first.confidence >= second.confidence,
                "results should be sorted by confidence desc"
            );
        }
    }
}
