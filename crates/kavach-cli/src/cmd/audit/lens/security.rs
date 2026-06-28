//! Security lens — six attack lenses via the shared loophole kernel, from
//! `cmd/loophole/detect.rs`.
use crate::cmd::audit::finding::{Finding, Lens, Severity};

/// Scan one file across the six attack lenses via the shared kernel.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    kavach_patterns::loophole_lens::scan_text(content)
        .into_iter()
        .map(|f| Finding {
            lens: Lens::Security,
            detector: format!("loophole:{}", f.lens.slug()),
            file: file.to_owned(),
            line: f.line,
            severity: Severity::Warn,
            hint: f.hint.to_owned(),
            fix: "root-cause via the named attack lens; fix at source or prove N/A".to_owned(),
        })
        .collect()
}

#[cfg(test)]
#[path = "security_test.rs"]
mod security_test;
