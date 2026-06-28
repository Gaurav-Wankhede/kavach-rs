//! Unified zero-LLM code auditor — consolidates the four pre-merge auditors
//! (lint-audit · doctor · hunt · loophole) behind ONE walker, ONE Finding type,
//! and ONE command. No metered LLM. SOURCE: decision.audit.unify-four-into-one.
mod finding;
mod report;
mod scan;
mod walk;
pub(crate) mod lens;

use std::path::Path;

/// `kavach audit` entry. Walks sources, runs the selected lenses, prints a
/// grouped report. Exit: 0 clean · 1 findings · 2 bad root.
#[must_use]
pub(crate) fn run(root: &Path, deep: bool, lens: lens::Selection, fix_cards: bool) -> i32 {
    let _ = (deep, fix_cards);
    if !root.exists() {
        eprintln!("audit: target path missing: {}", root.display());
        return 2;
    }
    let files = walk::source_files(root);
    let findings = scan::scan_all(root, &files, lens);
    report::report(&findings, files.len())
}
