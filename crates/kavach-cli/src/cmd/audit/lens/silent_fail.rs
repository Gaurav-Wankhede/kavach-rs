//! Silent-failure lens — shared kavach_patterns::silent_io_guard, scoped as the
//! old `kavach doctor` intent. Block severity.
use crate::cmd::audit::finding::{Finding, Lens, Severity};

/// Scan one file for silent-failure patterns via the shared kernel.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    kavach_patterns::silent_io_guard::detect(file, content)
        .into_iter()
        .map(|h| Finding {
            lens: Lens::SilentFail,
            detector: "silent_fail".to_owned(),
            file: file.to_owned(),
            line: h.line,
            severity: Severity::Block,
            hint: h.category.to_owned(),
            fix: h.fix.to_owned(),
        })
        .collect()
}

#[cfg(test)]
#[path = "silent_fail_test.rs"]
mod silent_fail_test;
