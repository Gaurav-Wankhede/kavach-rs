//! Architecture decision recorder — fires after Write/Edit on .rs files.

mod extract;
mod record;
pub(crate) mod time;

pub(crate) use record::record;

#[cfg(test)]
mod tests {
    use super::extract::{extract_arch_comment, extract_field};

    #[test]
    fn extracts_full_arch_comment() {
        let content = r"// ARCH: distributed_cache
// SCOPE: cache
// CAP: AP
// QPS: 10000 | PEAK: 3x
// STORAGE: 10GB
// FAILURE_MODE: stale reads
// TRADEOFF: consistency
// SEARCHED: 2026-04
// REFERENCE: https://example.com
fn setup_cache() {}";
        let arch = extract_arch_comment(content);
        assert!(arch.is_some());
        let a = arch.expect("arch comment");
        assert_eq!(a.pattern, "distributed_cache");
        assert_eq!(a.scope, "cache");
        assert_eq!(a.failure_mode, "stale reads");
    }

    #[test]
    fn returns_none_when_missing_required() {
        let content = "// ARCH: x\nfn foo() {}";
        assert!(extract_arch_comment(content).is_none());
    }

    #[test]
    fn extract_field_finds_value() {
        let content = "// ARCH: distributed_cache\n// SCOPE: cache";
        assert_eq!(
            extract_field(content, "ARCH:").as_deref(),
            Some("distributed_cache")
        );
    }
}
