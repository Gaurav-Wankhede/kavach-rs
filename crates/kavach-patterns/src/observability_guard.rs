// split: Observability gate — instrumentation/cardinality nudges. Advisory tier.
//
// [RCA]
// symptom:    handlers ship without spans → unsearchable in incidents; high-cardinality metrics blow up telemetry bills
// repro:      pub async fn list(State(pool): State<PgPool>) with no #[tracing::instrument] passes existing gates
// why1:       no gate flags missing #[tracing::instrument] on async handlers
// why2:       existing kavach gates check security/SOLID/DSA, not observability instrumentation
// why3:       invariant violated — every prod async handler must emit a structured span
// why4:       telemetry quality + cardinality is a backend invariant on par with SQL injection
// why5:       missing observability detection layer
// root_cause: no observability_guard module
// class:      knowledge_gap
// blast_radius: every Rust backend async fn handler in workspace
// research:   https://spacelift.io/blog/observability-best-practices
//             https://www.microsoft.com/en-us/microsoft-cloud/blog/2026/04/16/your-ai-steering-committees-2026-checklist-observability/
// fix_strategy: 6-pattern advisory module (P1/P2 only); wire into pre_write_guards.rs alongside other advisories
//
//! Observability Gate — Rust Backend Instrumentation (2026)

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub enum ObsSeverity {
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct ObsViolation {
    pub severity: ObsSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
}

pub(crate) static PATTERNS: LazyLock<Vec<Option<Regex>>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?s)(?:pub\s+)?async\s+fn\s+\w+\s*\([^)]*(?:State|Path|Json|Query|Extension)[^)]*\)[^{]*\{").ok(),
        Regex::new(r#"tracing::(?:error|warn)!\s*\(\s*"[^"]*"\s*\)"#).ok(),
        Regex::new(r#"(?:counter|histogram|gauge)!\s*\(\s*"[^"]*\{[^}]*\}"#).ok(),
        Regex::new(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,400}?tracing::(?:info|debug|trace)!").ok(),
        Regex::new(r"\b(?:println|eprintln|dbg)!\s*\(").ok(),
        Regex::new(r"\.map_err\(\|_\|\s*[^)]+\)").ok(),
    ]
});

fn is_target_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    if p.ends_with("/build.rs") {
        return false;
    }
    p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/repository/")
        || p.contains("/grpc/")
        || content.contains("axum::")
        || content.contains("tonic::")
        || content.contains("async fn")
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<ObsViolation> {
    if !is_target_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let mut v = Vec::new();
    let has_instrument =
        content.contains("#[tracing::instrument") || content.contains("info_span!");
    if PATTERNS
        .first()
        .is_some_and(|opt| opt.as_ref().is_some_and(|p| p.is_match(content)))
        && !has_instrument
    {
        v.push(ObsViolation { severity: ObsSeverity::P1Advisory,
            pattern: "handler-no-instrument",
            fix: "Axum/tonic handler without #[tracing::instrument] or info_span!. Add structured spans for prod traceability." });
    }
    if PATTERNS
        .get(1)
        .is_some_and(|opt| opt.as_ref().is_some_and(|p| p.is_match(content)))
    {
        v.push(ObsViolation { severity: ObsSeverity::P2Warning,
            pattern: "tracing-no-structured-fields",
            fix: "tracing::error!/warn! with bare string. Use key=%value structured fields for queryable logs." });
    }
    if PATTERNS
        .get(2)
        .is_some_and(|opt| opt.as_ref().is_some_and(|p| p.is_match(content)))
    {
        v.push(ObsViolation { severity: ObsSeverity::P1Advisory,
            pattern: "metric-high-cardinality",
            fix: "Metric name uses interpolation. Cardinality explodes; use labels with bounded value sets instead." });
    }
    if PATTERNS
        .get(3)
        .is_some_and(|opt| opt.as_ref().is_some_and(|p| p.is_match(content)))
    {
        v.push(ObsViolation { severity: ObsSeverity::P1Advisory,
            pattern: "tracing-in-hot-loop",
            fix: "tracing::info!/debug! inside loop = log volume blowup. Move outside, sample, or downgrade to trace!." });
    }
    if PATTERNS
        .get(4)
        .is_some_and(|opt| opt.as_ref().is_some_and(|p| p.is_match(content)))
    {
        v.push(ObsViolation { severity: ObsSeverity::P2Warning,
            pattern: "println-in-prod",
            fix: "println!/eprintln!/dbg! bypass tracing pipeline. Replace with tracing::info!/error!." });
    }
    if PATTERNS
        .get(5)
        .is_some_and(|opt| opt.as_ref().is_some_and(|p| p.is_match(content)))
    {
        v.push(ObsViolation { severity: ObsSeverity::P1Advisory,
            pattern: "map-err-loses-context",
            fix: ".map_err should preserve error context. Preserve via #[source] (thiserror) or .context() (anyhow)." });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_without_instrument_flagged() {
        let src = "use axum;\npub async fn list(State(pool): State<sqlx::PgPool>) -> Vec<u64> { vec![] }\n";
        let r = detect("src/handlers/users.rs", src);
        assert!(r.iter().any(|v| v.pattern == "handler-no-instrument"));
    }

    #[test]
    fn handler_with_instrument_ok() {
        let src = "use axum;\n#[tracing::instrument(skip(pool))]\npub async fn list(State(pool): State<sqlx::PgPool>) -> Vec<u64> { vec![] }\n";
        let r = detect("src/handlers/users.rs", src);
        assert!(!r.iter().any(|v| v.pattern == "handler-no-instrument"));
    }

    #[test]
    fn tracing_bare_string_flagged() {
        let src = "use tracing;\nasync fn x() {}\nfn handle() { tracing::error!(\"something failed\"); }\n";
        let r = detect("src/handlers/x.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "tracing-no-structured-fields")
        );
    }

    #[test]
    fn tracing_in_loop_flagged() {
        let src = "async fn x() {}\nfn h(ids: Vec<u64>) { for id in ids { tracing::info!(\"id processed\"); } }\n";
        let r = detect("src/services/h.rs", src);
        assert!(r.iter().any(|v| v.pattern == "tracing-in-hot-loop"));
    }

    #[test]
    fn println_flagged() {
        let src = "async fn x() {}\nfn h() { println!(\"debug\"); }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(r.iter().any(|v| v.pattern == "println-in-prod"));
    }

    #[test]
    fn map_err_underscore_flagged() {
        let src = "async fn x() {}\nfn h() -> Result<(),()> { let _: Result<u64, ()> = \"x\".parse().map_err(|_| ()); Ok(()) }\n";
        let r = detect("src/handlers/h.rs", src);
        assert!(r.iter().any(|v| v.pattern == "map-err-loses-context"));
    }

    #[test]
    fn non_target_skipped() {
        let r = detect("src/index.ts", "console.log('x')");
        assert!(r.is_empty());
    }

    #[test]
    fn test_file_skipped() {
        let src = "async fn x() {}\nfn h() { println!(\"ok\"); }\n";
        let r = detect("crate/tests/h.rs", src);
        assert!(r.is_empty());
    }
}
