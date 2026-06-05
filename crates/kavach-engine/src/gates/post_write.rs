//! Post-write umbrella gate: antiprod scan + quality + memory sync + recorders.
//! Runs after Write/Edit/NotebookEdit tools complete.
//!
//! `antiprod` is the P0 hard-block scan; `session` is the post-write bookkeeping;
//! `concept` upserts `// CONCEPT:` markers. The rest (event log, algo/arch
//! recorders, orphan/memory advisories) is orchestrated inline below.
mod antiprod;
mod concept;
mod session;

use crate::error::EngineError;
use crate::gates::post_write_checks::{read_written_content, run_content_quality_checks};
use kavach_types::HookInput;

/// Run the post-write pipeline. `Ok(())` always (uniform gate dispatch).
#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch")]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let file_path = input.get_string("file_path");
    let content = read_written_content(input);
    let mut context_parts: Vec<String> = Vec::new();

    // 1a. Anti-production pattern scan — P0 hard-blocks and returns early.
    if antiprod::check_antiprod(file_path, &content, &mut context_parts) {
        return Ok(());
    }
    // 1b. Content quality: assumptions + hallucinations + completion claims.
    run_content_quality_checks(&content, &mut context_parts);

    // 2. Session bookkeeping on the successful write.
    let mut sess = kavach_session::get_or_create_session();
    session::advance_session(&mut sess, file_path);

    // 4. Orphan detection — new files/exports without wiring.
    if let Some(orphan_ctx) = super::orphan_guard::check_orphan_risk(file_path, &content) {
        context_parts.push(orphan_ctx);
    }
    // 5. Memory-file write reminder — also persist to kavach-db.
    if file_path.contains("/memory/") || file_path.ends_with("MEMORY.md") {
        context_parts.push(
            "[MEMORY_DB_REMINDER]\n\
             You wrote to a memory file. Also persist to kavach-db:\n\
             kavach db write --project <slug> --category <cat> --key <key> --title <title>\n\
             Rule: kavach-db (SurrealDB) is the permanent store — MEMORY.md is session cache only"
                .into(),
        );
    }

    record_write(input, &sess, file_path, &content);
    concept::scan_concept_markers(&content);
    emit_context(file_path, &context_parts);
    Ok(())
}

/// Stages 5/5x/5a/5b: event log, bulk-sweep conformance, algo + arch recorders.
fn record_write(
    input: &HookInput,
    sess: &kavach_session::SessionState,
    file_path: &str,
    content: &str,
) {
    super::event_log::log_file_write(
        &sess.session_id,
        file_path,
        &input.tool_name,
        &sess.project,
        content,
    );
    // Bulk-mode conformance: fire-and-forget manifest increment (daemon-down no-ops).
    if let Ok(sweep_id) = std::env::var("KAVACH_BULK_SWEEP_ID")
        && !sweep_id.is_empty()
    {
        super::bulk_event::emit_apply(&sweep_id);
    }
    let turn = sess.turn_count.into();
    super::post_tool_algo_recorder::record(file_path, content, &sess.project, turn);
    super::post_tool_arch_recorder::record(file_path, content, &sess.project, turn);
}

/// Emit the `[POST_WRITE]` block only when an actionable advisory exists.
/// CONTEXT-ROT: noise in mid-context degrades every later output — filter at
/// source. SOURCE: research.trychroma.com/context-rot.
fn emit_context(file_path: &str, context_parts: &[String]) {
    if context_parts.is_empty() {
        return;
    }
    let mut full = format!("[POST_WRITE]\nfile: {file_path}\n\n");
    full.push_str(&context_parts.join("\n\n"));
    drop(kavach_hook::exit_post_tool_context(&full));
}
