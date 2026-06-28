use super::scan;
use crate::cmd::audit::finding::Lens;

#[test]
fn returns_worst_practice_lens_findings() {
    // unwrap() is a known rust_guard signature.
    let f = scan("a.rs", "fn z() { let v = parse(x).unwrap(); }\n");
    assert!(f.iter().all(|x| x.lens == Lens::WorstPractice));
}

#[test]
fn every_finding_is_worst_practice_lensed() {
    // Whatever the shared detectors surface, the lens tag is always uniform —
    // the consolidation invariant (not the detector internals).
    let f = scan("a.rs", "fn z() { let v = parse(x).unwrap(); }\n");
    assert!(!f.is_empty(), "unwrap() must surface at least one finding");
    assert!(f.iter().all(|x| x.lens == Lens::WorstPractice));
}
