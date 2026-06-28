use super::scan;
use crate::cmd::audit::finding::Lens;

#[test]
fn findings_are_security_lens() {
    let f = scan("a.rs", "fn z() { let v = items[0]; }\n");
    assert!(f.iter().all(|x| x.lens == Lens::Security));
}

#[test]
fn detector_names_the_attack_lens() {
    let f = scan("a.rs", "fn z() { let v = items[0]; }\n");
    assert!(f.iter().any(|x| x.detector.starts_with("loophole:")));
}
