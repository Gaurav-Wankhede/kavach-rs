//! §DEDUP recall-not-redefine routing: a governed file that redefines a name it
//! already imports is a hard `P0Block` (recall the central object, never redefine).
//! No P1 tier — a redefinition of an import is, by construction, a duplication.
//! Stricter governed-path sibling of rustc's `hidden_glob_reexports` warn-lint.
//! SOURCE: <https://martinfowler.com/bliki/SingleSourceOfTruth.html>
use super::super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// `Some(reason)` blocks the write when an imported name is redefined locally.
pub(super) fn dedup(ctx: &WriteContext<'_>, _acc: &mut Acc) -> Option<String> {
    use kavach_patterns::severity::Severity::P0Block;
    for v in &kavach_patterns::dedup_guard::detect(ctx.file_path, ctx.content) {
        if v.severity == P0Block {
            return Some(format!("[DEDUP_P0/{}] {}", v.pattern, v.fix));
        }
    }
    None
}
