//! Worst-practice "antivirus" lens — shared kavach_patterns detectors, from `cmd/hunt`.
use crate::cmd::audit::finding::{Finding, Lens, Severity};
use kavach_patterns::{dsa_guard, database_ops_guard, pii_data_guard, migration_safety_guard, owasp_guard, rust_196_guard, rust_guard, silent_io_guard};

/// Run every kavach_patterns detector over one file, mapped into unified findings.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for h in silent_io_guard::detect(file, content) {
        out.push(mk(
            "silent_io",
            file,
            h.line,
            Severity::Block,
            h.category,
            h.fix,
        ));
    }
    for f in owasp_guard::detect(file, content) {
        out.push(mk(
            "owasp",
            file,
            f.line,
            Severity::Block,
            f.category,
            f.fix,
        ));
    }
    for v in rust_guard::detect(file, content) {
        out.push(mk(
            "rust",
            file,
            v.line,
            Severity::Advisory,
            &v.pattern,
            &v.fix,
        ));
    }
    for v in rust_196_guard::detect(file, content) {
        use kavach_patterns::rust_196_guard::Rust196Severity as S;
        let sev = match v.severity {
            S::P0Block => Severity::Block,
            S::P1Advisory => Severity::Advisory,
            S::P2Warning => Severity::Warn,
        };
        out.push(mk("rust1.96", file, v.line, sev, v.pattern, v.fix));
    }
    for v in dsa_guard::detect(file, content) {
        use kavach_patterns::dsa_guard::DsaSeverity as DS;
        let sev = match v.severity {
            DS::P1Advisory => Severity::Warn,
            DS::P2Warning => Severity::Advisory,
        };
        out.push(mk("dsa", file, v.line, sev, v.pattern, v.fix));
    }
    for v in database_ops_guard::detect(file, content) {
        use kavach_patterns::database_ops_guard::DbOpsSeverity as DBS;
        let sev = match v.severity {
            DBS::P0Block => Severity::Block,
            DBS::P1Advisory => Severity::Warn,
            DBS::P2Warning => Severity::Advisory,
        };
        out.push(mk("database_ops", file, v.line, sev, v.pattern, v.fix));
    }
    for v in pii_data_guard::detect(file, content) {
        out.push(mk("pii", file, 0, Severity::Block, v.pattern, v.fix));
    }
    for v in migration_safety_guard::detect(file, content) {
        out.push(mk("migration", file, 0, Severity::Block, v.pattern, v.fix));
    }
    out
}

fn mk(
    detector: &str,
    file: &str,
    line: usize,
    severity: Severity,
    hint: &str,
    fix: &str,
) -> Finding {
    Finding {
        lens: Lens::WorstPractice,
        detector: detector.to_owned(),
        file: file.to_owned(),
        line,
        column: 0,
        severity,
        hint: hint.to_owned(),
        fix: fix.to_owned(),
    }
}

#[cfg(test)]
#[path = "worst_practice_test.rs"]
mod worst_practice_test;
