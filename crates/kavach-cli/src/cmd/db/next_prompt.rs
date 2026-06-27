// `kavach db next-prompt` — serve the top-priority todo card's exec_prompt to stdout.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_surreal::MemoryEntry;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;
mod author;
#[cfg(test)]
#[path = "next_prompt/next_prompt_test.rs"]
mod tests;
/// Outcome of selecting a servable prompt from a priority-ordered roadmap list.
pub(crate) enum Pick<'a> {
    /// The top todo card carries a non-empty exec_prompt — serve it.
    Prompt(&'a str),
    /// The top todo card exists but has no exec_prompt yet — kavach authors it.
    Missing(&'a MemoryEntry),
    /// No todo card on the board.
    Empty,
}
/// Pick the first `todo` roadmap card from a priority-ordered slice and classify
/// its exec_prompt. Pure — the IO wrapper below renders the outcome.
pub(crate) fn pick(rows: &[MemoryEntry]) -> Pick<'_> {
    let Some(top) = rows.iter().find(|r| r.entry_status_str() == "todo") else {
        return Pick::Empty;
    };
    match top.exec_prompt.as_deref() {
        Some(p) if !p.trim().is_empty() => Pick::Prompt(p),
        _ => Pick::Missing(top),
    }
}
/// `kavach db next-prompt --project X`: print the prompt to stdout, or a stderr
/// warning + non-zero exit when the top card has none / the board is empty.
pub(crate) fn run(project: &str) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return render_err(&format!("error: tokio runtime: {e}")),
    };
    runtime.block_on(async {
        let db = match kavach_surreal::open_default().await {
            Ok(d) => d,
            Err(e) => return render_err(&format!("error: open SurrealDB: {e}")),
        };
        let project_id = match kavach_surreal::project_get_by_slug(&db, project).await {
            Ok(Some(p)) => match p.id {
                Some(id) => id,
                None => return render_err("error: project has no id"),
            },
            Ok(None) => return render_err(&format!("error: project not found: {project}")),
            Err(e) => return render_err(&format!("error: {e}")),
        };
        let rows = match kavach_surreal::list_by_project(&db, "roadmap", &project_id).await {
            Ok(rs) => rs,
            Err(e) => return render_err(&format!("error: list lookup: {e}")),
        };
        match pick(&rows) {
            Pick::Prompt(p) => match print_or_exit(p) {
                Ok(()) => 0,
                Err(io_err) => into_exit_code(io_err),
            },
            Pick::Missing(card) => author_and_serve(&db, &project_id, project, card).await,
            Pick::Empty => render_err(&format!("error: no todo card on the {project} board")),
        }
    })
}
/// Author the missing exec_prompt via Haiku, write it back to the SAME card (no
/// skip), then serve it. Fail-soft: any author/write error falls back to the
/// strict missing-prompt error so the harness never crashes.
async fn author_and_serve(
    db: &Surreal<Db>,
    project_id: &RecordId,
    project: &str,
    card: &MemoryEntry,
) -> i32 {
    let prompt = author::authoring_prompt(project, &card.entry_key, &card.title, &card.content);
    let authored = match kavach_advisor::ask(&prompt, 4) {
        Ok(text) if !text.trim().is_empty() => text,
        _ => {
            return render_err(&format!(
                "error: top todo card '{}' has no exec_prompt and auto-authoring failed \
                 (set ANTHROPIC_API_KEY); author one with --exec-prompt",
                card.entry_key
            ));
        }
    };
    let qname = format!("{project}/roadmap/{}", card.entry_key);
    let written = kavach_surreal::upsert_entry_full()
        .db(db)
        .category("roadmap")
        .project_id(project_id)
        .entry_key(&card.entry_key)
        .title(&card.title)
        .content(&card.content)
        .event_source("next-prompt-autoauthor")
        .qualified_name(&qname)
        .references(&[])
        .maybe_exec_prompt(Some(authored.as_str()))
        .build_for_call()
        .await;
    if let Err(e) = written {
        return render_err(&format!(
            "error: authored prompt but write-back failed: {e}"
        ));
    }
    match print_or_exit(&authored) {
        Ok(()) => 0,
        Err(io_err) => into_exit_code(io_err),
    }
}
fn render_err(msg: &str) -> i32 {
    match ewrite_or_exit(msg) {
        Ok(()) => 1,
        Err(io_err) => into_exit_code(io_err),
    }
}
