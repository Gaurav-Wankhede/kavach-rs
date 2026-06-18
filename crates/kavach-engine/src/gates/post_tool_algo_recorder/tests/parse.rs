//! `extract_algo_comment` + `extract_field` parsing tests.
use super::super::parse::{extract_algo_comment, extract_field};
use super::common::full_comment;

#[test]
fn extracts_full_algo_comment() {
    let content = full_comment(2026, 4, 2021);
    let Some(algo) = extract_algo_comment(&content) else {
        panic!("expected Some")
    };
    assert_eq!(algo.chosen, "pdqsort");
    assert_eq!(algo.problem_class, "sorting");
    assert_eq!(algo.search_year, 2026);
    assert_eq!(algo.search_month, 4);
    assert_eq!(algo.year_published, 2021);
    assert!(algo.benchmark_source.is_none());
}

#[test]
fn returns_none_when_no_algo_comment() {
    assert!(extract_algo_comment("fn greet() {}").is_none());
}

#[test]
fn returns_none_when_missing_required_field() {
    assert!(extract_algo_comment("// ALGO: pdqsort\nfn sort() {}").is_none());
}

#[test]
fn extract_field_finds_value() {
    let content = "// ALGO: robin-hood-hashing\n// PROBLEM_CLASS: hash-map";
    assert_eq!(
        extract_field(content, "ALGO:").as_deref(),
        Some("robin-hood-hashing")
    );
    assert_eq!(
        extract_field(content, "PROBLEM_CLASS:").as_deref(),
        Some("hash-map")
    );
}

#[test]
fn extract_field_returns_none_when_absent() {
    assert!(extract_field("fn main() {}", "ALGO:").is_none());
}

#[test]
fn extracts_inline_searched_from_year_line() {
    // Skill template uses "YEAR: 2021 | SEARCHED: 2026-04" on one line.
    let content = "// ALGO: pdqsort\n\
                   // YEAR: 2021 | SEARCHED: 2026-04\n\
                   fn sort() {}";
    let Some(algo) = extract_algo_comment(content) else {
        panic!("expected Some")
    };
    assert_eq!(algo.year_published, 2021);
    assert_eq!(algo.search_year, 2026);
    assert_eq!(algo.search_month, 4);
}
