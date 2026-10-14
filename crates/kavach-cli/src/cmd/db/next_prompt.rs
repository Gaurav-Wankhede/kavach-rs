// `kavach db next-prompt` — serve the top-priority todo card's exec_prompt to stdout.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::dual_write::MemoryEntry;
use kavach_db::db::entries::EntryStore;
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
///
/// Lane match is typed (`effective_status_typed`): legacy `status`-only rows
/// read through the same default as every other board, so a `done` row can
/// never be served as the next todo.
pub(crate) fn pick(rows: &[MemoryEntry]) -> Pick<'_> {
    let Some(top) = rows
        .iter()
        .find(|r| r.effective_status_typed() == Some(kavach_types::MemoryStatus::Todo))
    else {
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
        // Phase 6: direct open via DatabaseStore (libSQL file — no lock contention).
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => return render_err(&format!("error: open kavach.db: {e}")),
        };
        let rows = match store.list_by_category(project, "roadmap").await {
            Ok(rs) => rs,
            Err(e) => return render_err(&format!("error: list lookup: {e}")),
        };
        match pick(&rows) {
            Pick::Prompt(p) => match print_or_exit(p) {
                Ok(()) => 0,
                Err(io_err) => into_exit_code(io_err),
            },
            Pick::Missing(card) => author_and_serve(&store, project, card).await,
            Pick::Empty => render_err(&format!("error: no todo card on the {project} board")),
        }
    })
}
/// Author the missing exec_prompt via Haiku, write it back to the SAME card (no
/// skip), then serve it. Fail-soft: any author/write error falls back to the
/// strict missing-prompt error so the harness never crashes.
async fn author_and_serve(
    store: &(impl EntryStore + Sync),
    _project: &str,
    card: &MemoryEntry,
) -> i32 {
        let authored = match card.exec_prompt.as_deref() {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => {
            card.title.clone()
        }
    };
    // Update the entry with the authored exec_prompt
    let mut updated = card.clone();
    updated.exec_prompt = Some(authored.clone());
    if let Err(e) = store.upsert_entry(&updated).await {
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
