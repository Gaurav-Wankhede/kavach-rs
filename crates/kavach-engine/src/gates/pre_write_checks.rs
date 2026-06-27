//! Pre-write gate helpers, grouped by concern:
//! `context` builds the chain/approval context strings, `classify` decides
//! code-file / test-file status, `checkbox` flags bulk plan-checkbox writes.
mod checkbox;
mod classify;
mod context;
#[cfg(test)]
#[path = "pre_write_checks_test.rs"]
#[path = "pre_write_checks_test.rs"]
mod tests;
pub(crate) use checkbox::detect_bulk_checkbox;
pub(crate) use classify::{is_code_write, is_test_or_exempt};
pub(crate) use context::{build_approval_context, extract_write_context};
