//! Audit lenses — one detector family per file, all returning unified Findings.
pub(crate) mod security;
pub(crate) mod silent_fail;
pub(crate) mod worst_practice;
pub(crate) mod yagni;

use super::finding::Finding;

/// Which lenses to run. `All` runs every lens (default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
    Code,
    SelfAudit,
    Security,
    All,
}

/// Run the selected lenses over one file's content.
#[must_use]
pub(crate) fn scan_file(file: &str, content: &str, sel: Selection) -> Vec<Finding> {
    let mut out = Vec::new();
    if matches!(sel, Selection::Code | Selection::All) {
        out.extend(yagni::scan(file, content));
        out.extend(worst_practice::scan(file, content));
    }
    if matches!(sel, Selection::SelfAudit | Selection::All) {
        out.extend(silent_fail::scan(file, content));
    }
    if matches!(sel, Selection::Security | Selection::All) {
        out.extend(security::scan(file, content));
    }
    out
}
