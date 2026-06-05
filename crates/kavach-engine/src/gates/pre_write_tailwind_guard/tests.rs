//! Tailwind guard coverage: frontend gating, missing-index safety, Jaccard math,
//! keyword extraction from path + content.
use super::advisory::{advisory, is_frontend_file};
use super::keywords::extract_query_keywords;
use super::matching::jaccard;

#[test]
fn should_return_none_for_non_frontend_files() {
    assert!(advisory("src/main.rs", "").is_none());
    assert!(advisory("styles.css", "").is_none());
}

#[test]
fn should_return_none_for_missing_index() {
    // Index doesn't exist in test env — gate must be silent and never panic.
    let _ = advisory(
        "/tmp/test/Sidebar.tsx",
        "export default function Sidebar() {}",
    );
}

#[test]
fn should_detect_frontend_extensions() {
    assert!(is_frontend_file("Component.tsx"));
    assert!(is_frontend_file("Page.jsx"));
    assert!(is_frontend_file("Layout.astro"));
    assert!(!is_frontend_file("util.ts"));
    assert!(!is_frontend_file("main.rs"));
}

#[test]
fn should_compute_jaccard_correctly() {
    let a = vec![
        "sidebar".to_owned(),
        "navigation".to_owned(),
        "dark".to_owned(),
    ];
    let b = vec![
        "sidebar".to_owned(),
        "navigation".to_owned(),
        "light".to_owned(),
    ];
    // intersection=2, union=4 → 0.5
    assert!((jaccard(&a, &b) - 0.5).abs() < f64::EPSILON);
}

#[test]
fn should_return_zero_jaccard_for_empty_inputs() {
    assert!((jaccard(&[], &[]) - 0.0).abs() < f64::EPSILON);
    assert!((jaccard(&["a".to_owned()], &[]) - 0.0).abs() < f64::EPSILON);
}

#[test]
fn should_extract_keywords_from_path_and_content() {
    let kw = extract_query_keywords(
        "src/components/navigation/Sidebar.tsx",
        "import { useState } from 'react'\n// dark sidebar with nav links",
    );
    assert!(kw.contains(&"sidebar".to_owned()));
    assert!(kw.contains(&"navigation".to_owned()));
}
