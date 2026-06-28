use super::{Finding, Lens, Severity};

#[test]
fn dedup_key_is_lens_detector_file_line() {
    let f = Finding {
        lens: Lens::Yagni,
        detector: "yagni".to_owned(),
        file: "a.rs".to_owned(),
        line: 7,
        severity: Severity::Advisory,
        hint: "h".to_owned(),
        fix: "f".to_owned(),
    };
    assert_eq!(f.dedup_key(), "yagni|yagni|a.rs|7");
}

#[test]
fn severity_labels_are_stable() {
    assert_eq!(Severity::Block.label(), "BLOCK");
    assert_eq!(Severity::Warn.label(), "WARN");
    assert_eq!(Severity::Advisory.label(), "ADVISORY");
}

#[test]
fn lens_slugs_are_stable() {
    assert_eq!(Lens::Security.slug(), "security");
    assert_eq!(Lens::SilentFail.slug(), "silent-fail");
}
