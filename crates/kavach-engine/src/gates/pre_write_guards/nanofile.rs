//! Nano-file guard (mod.rs ban, depth cap, <=100-LOC split) + microservice
//! guard. The nano-file guard's `P0Block` path and the microservice guard's
//! oversized-file path both return a block reason; P1 hits push advisories.
use super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// Nano-file guard: mod.rs forbidden, depth <=7 below src/, new files <=100 LOC.
/// Severity-routed: `P0Block` returns a block reason; `P1Advisory` pushes context.
/// Uses `effective_content` (the WHOLE resulting file), not `ctx.content`: on an
/// Edit/Update, `ctx.content` is only the `new_string` fragment, so the LOC split
/// check would never fire on in-place edits — it must see the post-edit file
/// body, which `effective_content` holds (full file for Edit, content for Write).
pub(super) fn nano_file(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    if !ctx.is_rust || ctx.is_test {
        return None;
    }
    for v in kavach_patterns::nano_file_guard::detect(
        ctx.file_path,
        &ctx.effective_content,
        ctx.tool_name,
    ) {
        use kavach_patterns::nano_file_guard::NanoSeverity::{P0Block, P1Advisory};
        match v.severity {
            P0Block => return Some(format!("[NANO_FILE_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[NANO_FILE_P1] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// Microservice guard — promoted to P0 BLOCK per CLAUDE.md "max 100 lines" rule.
/// On an Edit, suppression comments (`// hub:` / `// split:`) in the fragment are
/// merged into the full file so you can add them despite the edit being blocked.
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
            "[MICROSERVICE_P0] {msg}\n\nSplit file before writing. Add `// split:` comment to suppress if intentional."
        )
    })
}
