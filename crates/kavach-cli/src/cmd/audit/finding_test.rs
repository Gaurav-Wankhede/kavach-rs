#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn lens_slug() {
        assert_eq!(Lens::Yagni.slug(), "yagni");
        assert_eq!(Lens::SilentFail.slug(), "silent-fail");
        assert_eq!(Lens::WorstPractice.slug(), "worst-practice");
        assert_eq!(Lens::Security.slug(), "security");
    }

    #[test]
    fn severity_label() {
        assert_eq!(Severity::Block.label(), "BLOCK");
        assert_eq!(Severity::Warn.label(), "WARN");
        assert_eq!(Severity::Advisory.label(), "ADVISORY");
    }

    #[test]
    fn finding_dedup_key() {
        let f = Finding {
            lens: Lens::Yagni,
            detector: "reuse_ladder_guard".to_string(),
            file: "src/lib.rs".to_string(),
            line: 42,
            severity: Severity::Warn,
            hint: "unused symbol".to_string(),
            fix: "delete or use".to_string(),
        };
        assert_eq!(
            f.dedup_key(),
            "yagni|reuse_ladder_guard|src/lib.rs|42"
        );
    }
}
