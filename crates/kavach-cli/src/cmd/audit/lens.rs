//! Audit lenses — one detector family per file, all returning unified Findings.
pub(crate) mod security;
mod selection;
pub(crate) mod silent_fail;
pub(crate) mod worst_practice;
pub(crate) mod yagni;

pub(crate) use selection::Selection;

use super::finding::Finding;

/// Run the selected lenses over one file's content.
#[must_use]
pub(crate) fn scan_file(file: &str, content: &str, sel: Selection) -> Vec<Finding> {
    // Isolate Markdown files: documentation prose must never trigger code-level lint findings.
    let is_md = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"));
    if is_md {
        return Vec::new();
    }

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
