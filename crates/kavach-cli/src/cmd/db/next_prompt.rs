// `kavach db next-prompt` — serve the top-priority todo card's exec_prompt to stdout.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_surreal::MemoryEntry;

#[cfg(test)]
#[path = "next_prompt_test.rs"]
mod tests;

/// Outcome of selecting a servable prompt from a priority-ordered roadmap list.
pub(crate) enum Pick<'a> {
    /// The top todo card carries a non-empty exec_prompt — serve it.
    Prompt(&'a str),
    /// The top todo card exists but has no exec_prompt yet.
    Missing(&'a str),
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
        _ => Pick::Missing(&top.entry_key),
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
            Pick::Missing(key) => render_err(&format!(
                "error: top todo card '{key}' has no exec_prompt; have Opus author one first"
            )),
            Pick::Empty => render_err(&format!("error: no todo card on the {project} board")),
        }
    })
}

fn render_err(msg: &str) -> i32 {
    match ewrite_or_exit(msg) {
        Ok(()) => 1,
        Err(io_err) => into_exit_code(io_err),
    }
}
