//! Jaccard identity/disjointness, threshold classification, and pairwise check.
use super::decision::{DupDecision, check, classify};
use super::shingle::{jaccard, shingles};

fn body_a() -> &'static str {
    "let mut total = 0; \
     let mut count = 0; \
     let limit = config.max_items; \
     for item in input.iter().take(limit) { \
         if item.is_valid() && item.weight > 0 { \
             total = total + item.weight; \
             count = count + 1; \
         } else if item.is_pending() { \
             pending_queue.push(item.clone()); \
             continue; \
         } else { \
             skipped.push(item.id); \
         } \
     } \
     tracing::info!(count, total, \"computed\"); \
     return Ok((total, count));"
}

fn body_a_renamed() -> &'static str {
    body_a()
}

fn body_b() -> &'static str {
    "fn render(node: &Node) -> Html { \
         let header = h1(&node.title).with_class(\"title\"); \
         let body = paragraph(&node.body).with_class(\"text\"); \
         let footer = byline(&node.author); \
         let metadata = span(format!(\"{} views\", node.views)); \
         container(vec![header, body, footer, metadata]) \
             .with_class(\"article-card\") \
             .with_aria(\"article\") \
             .build() \
     }"
}

#[test]
fn jaccard_identical_is_one() {
    let a = shingles(body_a());
    let b = shingles(body_a_renamed());
    assert!(jaccard(&a, &b) > 0.99);
}

#[test]
fn jaccard_unrelated_is_low() {
    let a = shingles(body_a());
    let b = shingles(body_b());
    assert!(
        jaccard(&a, &b) < 0.30,
        "unrelated bodies should score < 0.30"
    );
}

#[test]
fn classify_thresholds() {
    assert_eq!(classify(0.95), DupDecision::Block);
    assert_eq!(classify(0.85), DupDecision::Block);
    assert_eq!(classify(0.80), DupDecision::Advise);
    assert_eq!(classify(0.70), DupDecision::Advise);
    assert_eq!(classify(0.50), DupDecision::Clean);
    assert_eq!(classify(0.0), DupDecision::Clean);
}

#[test]
fn short_bodies_return_clean() {
    let result = check("fn x() { 1 }", &[body_a()]);
    assert_eq!(result.0, DupDecision::Clean);
}

#[test]
fn check_finds_duplicate() {
    let (decision, idx) = check(body_a(), &[body_b(), body_a_renamed()]);
    assert_eq!(decision, DupDecision::Block);
    assert_eq!(idx, Some(1));
}

#[test]
fn check_clean_when_no_duplicates() {
    let (decision, _) = check(body_a(), &[body_b()]);
    assert_eq!(decision, DupDecision::Clean);
}
