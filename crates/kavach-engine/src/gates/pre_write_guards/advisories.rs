//! Style advisories (P2) — algo complexity, secrecy, alloc, a11y. Concatenated
//! into a single advisory string assigned to `algo_advisory` when non-empty.
use super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// Build the concatenated algo/secrecy/alloc/a11y advisory string.
pub(super) fn collect(ctx: &WriteContext<'_>, acc: &mut Acc) {
    if ctx.is_test {
        return;
    }
    let mut advisories = String::new();
    if let Some(a) =
        kavach_patterns::algo_complexity_guard::advise(ctx.file_path, &ctx.effective_content)
    {
        advisories.push_str(&a);
    }
    if let Some(a) = kavach_patterns::secrecy_guard::advise(ctx.file_path, &ctx.effective_content) {
        advisories.push_str(&a);
    }
    if let Some(a) = kavach_patterns::alloc_guard::advise(ctx.file_path, &ctx.effective_content) {
        advisories.push_str(&a);
    }
    if let Some(a) = kavach_patterns::a11y_guard::advise(ctx.file_path, &ctx.effective_content) {
        advisories.push_str(&a);
    }
    if let Some(a) =
        kavach_patterns::comment_noise_guard::advise(ctx.file_path, &ctx.effective_content)
    {
        advisories.push_str(&a);
    }
    if let Some(a) = kavach_patterns::lint_profile_guard::advise(ctx.file_path) {
        advisories.push_str(&a);
    }
    let old = std::fs::read_to_string(ctx.file_path).unwrap_or_default();
    if let Some(a) =
        kavach_patterns::reuse_ladder_guard::advise(ctx.file_path, &old, &ctx.effective_content)
    {
        advisories.push_str(&a);
    }
    if !advisories.is_empty() {
        acc.algo_advisory = Some(advisories);
    }
}
