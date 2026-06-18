//! `verify_algo_comment` verification-layer tests.
use super::super::datetime::current_year;
use super::super::parse::extract_algo_comment;
use super::super::verify::verify_algo_comment;
use super::common::full_comment;

#[test]
fn verify_rejects_stale_search_year() {
    let content = full_comment(2020, 1, 2019);
    let Some(algo) = extract_algo_comment(&content) else {
        panic!("expected Some")
    };
    let result = verify_algo_comment(&algo);
    assert!(result.is_err());
    assert!(result.err().unwrap_or_default().contains("stale"));
}

#[test]
fn verify_rejects_future_search_year() {
    let now = current_year();
    let content = full_comment(now + 2, 1, 2021);
    let Some(algo) = extract_algo_comment(&content) else {
        panic!("expected Some")
    };
    let result = verify_algo_comment(&algo);
    assert!(result.is_err());
    assert!(result.err().unwrap_or_default().contains("future"));
}

#[test]
fn verify_rejects_implausible_publication_year() {
    let now = current_year();
    let content = full_comment(now, 4, 1900);
    let Some(algo) = extract_algo_comment(&content) else {
        panic!("expected Some")
    };
    let result = verify_algo_comment(&algo);
    assert!(result.is_err());
    assert!(result.err().unwrap_or_default().contains("implausible"));
}

#[test]
fn verify_passes_valid_comment_no_benchmark() {
    let now = current_year();
    // No BENCHMARK field — skip URL check. SEARCHED on its own line.
    let content = format!(
        "// ALGO: pdqsort\n\
         // TIME: O(n log n) | SPACE: O(log n)\n\
         // YEAR: 2021\n\
         // SEARCHED: {now}-04\n\
         fn sort() {{}}"
    );
    let Some(algo) = extract_algo_comment(&content) else {
        panic!("expected Some")
    };
    assert!(verify_algo_comment(&algo).is_ok());
}
