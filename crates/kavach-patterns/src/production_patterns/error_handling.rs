//! Error handling anti-patterns.

use super::types::{Severity, mk};
use crate::config::j;

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    let unw = j(&[r"\.unw", r"rap\(\)"]);
    let exp = j(&[r"\.exp", r"ect\("]);
    let uor = j(&[r"\.unw", "rap_or"]);
    let pnc = j(&["pan", "ic!"]);

    vec![
        (
            mk(&unw),
            "UNWRAP",
            "unwrap() crashes on error — use ? or match",
            Severity::P0Critical,
        ),
        (
            mk(&exp),
            "EXPECT",
            "expect() crashes on error — use ? or match",
            Severity::P0Critical,
        ),
        (
            mk(&uor),
            "UNWRAP_OR",
            "unwrap_or hides error — use map_err + ?",
            Severity::P1High,
        ),
        (
            mk(&format!(r"\b{pnc}\s*\(")),
            "PANIC",
            "panic! is unrecoverable — return Result",
            Severity::P0Critical,
        ),
        (
            mk(r"\.ok\(\)"),
            "SILENT_OK",
            ".ok() discards error — log or propagate",
            Severity::P1High,
        ),
        (
            mk(r"\.map_err\(\|_\|"),
            "CONTEXT_LOST",
            "Error context discarded — preserve with .context()",
            Severity::P1High,
        ),
        (
            mk(r#"Err\(\s*"[^"]+"\s*\.into\(\)\s*\)"#),
            "GENERIC_ERR",
            "String error — use typed error with thiserror",
            Severity::P1High,
        ),
        (
            mk(r"Err\(_\)\s*=>"),
            "CATCH_ALL_ERR",
            "Catch-all Err(_) — match specific variants",
            Severity::P1High,
        ),
        (
            mk(r"Err\([^)]*\)\s*=>\s*\{\s*\}"),
            "EMPTY_CATCH",
            "Empty error handler — log or propagate",
            Severity::P0Critical,
        ),
        (
            mk(r"\.unwrap_or_default\(\)"),
            "SWALLOW_DEFAULT",
            "unwrap_or_default hides errors — explicit handling",
            Severity::P1High,
        ),
    ]
}
