use super::scan;
use crate::cmd::audit::finding::{Lens, Severity};

#[test]
fn findings_are_silent_fail_block() {
    let f = scan("a.rs", "fn z() { let _ = client.send(req); }\n");
    assert!(f.iter().all(|x| x.lens == Lens::SilentFail && x.severity == Severity::Block));
}

#[test]
fn clean_source_is_empty() {
    assert!(scan("a.rs", "fn ok() {}\n").is_empty());
}
