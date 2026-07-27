//! Microservice guard — P1 ADVISORY. The 100-line guidance steers splitting but no
//! longer hard-blocks, because the rewrite loop it triggered burned more tokens than
//! the split saved. Suppression comments (`// hub:` / `// split:`) still apply.
use crate::gates::pre_write_context::WriteContext;

/// Microservice guard — P1 ADVISORY.
pub(super) fn microservice(ctx: &WriteContext<'_>) -> Option<String> {
    if ctx.is_test {
        return None;
    }
    let full_content_buf = (ctx.tool_name == "Edit" && !ctx.file_path.is_empty())
        .then(|| std::fs::read_to_string(ctx.file_path).ok())
        .flatten();
    let full_pc = full_content_buf.as_deref().unwrap_or(ctx.content);
    // Merge suppression comments from edit into full file for microservice guard.
    // This prevents catch-22 where you can't add // split: because the edit is blocked.
    let merged_for_suppression;
    let ms_content = if ctx.tool_name == "Edit" {
        let edit_has_hub = ctx.content.contains("// hub:");
        let edit_has_split = ctx.content.contains("// split:");
        let file_has_hub = full_pc.contains("// hub:");
        let file_has_split = full_pc.contains("// split:");
        if (edit_has_hub && !file_has_hub) || (edit_has_split && !file_has_split) {
            merged_for_suppression = format!("{}\n{full_pc}", ctx.content);
            merged_for_suppression.as_str()
        } else {
            full_pc
        }
    } else {
        full_pc
    };
    super::super::pre_write_microservice_guard::check(ctx.file_path, ms_content).map(|msg| {
        format!(
            "[MICROSERVICE_P1] {msg}\n\nConsider splitting; add `// split:` or `// hub:` comment if intentional."
        )
    })
}
