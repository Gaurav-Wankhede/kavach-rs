//! §bloatware tombstone-comment routing: a governed source file that ADDS a
//! comment documenting a removal is a hard `P0Block` (delete the thing; git
//! history is the record — decision.bloatware.no-tombstone-comments).
//! SOURCE: <https://arxiv.org/pdf/2604.00478>
use super::super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// `Some(reason)` blocks the write when a tombstone comment is present.
pub(super) fn bloatware(ctx: &WriteContext<'_>, _acc: &mut Acc) -> Option<String> {
    use kavach_patterns::severity::Severity::P0Block;
    for v in &kavach_patterns::bloatware_guard::detect(ctx.file_path, ctx.content) {
        if v.severity == P0Block {
            return Some(format!("[BLOAT_P0/{}] {}", v.pattern, v.fix));
        }
    }
    None
}
