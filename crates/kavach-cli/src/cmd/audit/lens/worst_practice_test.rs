use super::scan;
use crate::cmd::audit::finding::Lens;

#[test]
fn returns_worst_practice_lens_findings() {
    // unwrap() is a known rust_guard signature.
    let f = scan("a.rs", "fn z() { let v = parse(x).unwrap(); }\n");
    assert!(f.iter().all(|x| x.lens == Lens::WorstPractice));
}

#[test]
fn clean_source_is_empty() {
    assert!(scan("a.rs", "fn ok() {}\n").is_empty());
}
