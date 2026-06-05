//! File-pattern skill-enforcement predicate: applies on every Write, and on
//! Edit only for frontend component files (`.tsx`/`.jsx`/`.astro`).
use crate::gates::pre_write_context::WriteContext;

/// True when file-pattern skill enforcement should run for this write.
pub(super) fn should_check_patterns(ctx: &WriteContext<'_>, is_low_risk: bool) -> bool {
    let is_frontend = std::path::Path::new(ctx.file_path)
        .extension()
        .is_some_and(|e| {
            e.eq_ignore_ascii_case("tsx")
                || e.eq_ignore_ascii_case("jsx")
                || e.eq_ignore_ascii_case("astro")
        });
    ctx.is_code
        && !is_low_risk
        && (ctx.tool_name == "Write" || (ctx.tool_name == "Edit" && is_frontend))
}
