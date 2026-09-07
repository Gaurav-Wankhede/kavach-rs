mod field;

use field::{FieldEntryView, extract_field};
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::turso::entries::EntryStore;

/// A single-key get is a DEPTH request → full content by default; `--snippet`
/// opts back into the short body, but `--full` always wins (it implies content).
#[must_use]
pub(crate) const fn want_full_content(full: bool, snippet: bool) -> bool {
    full || !snippet
}

#[expect(
    clippy::too_many_lines,
    reason = "multi-stage fallback from RPC to direct DB access with error handling"
)]
pub(super) fn run(
    project_slug: &str,
    category: &str,
    key: &str,
    full: bool,
    snippet: bool,
    field: Option<&str>,
    compact: bool,
) -> i32 {
    let fetch_content = want_full_content(full, snippet)
        || field.is_some_and(|f| f.eq_ignore_ascii_case("content"));
    match super::rpc_client::get(
        project_slug,
        category,
        key,
        fetch_content,
    ) {
        Ok(result) if result.found => {
            if let Some(entry) = result.entry {
                if let Some(f) = field {
                    let view = FieldEntryView {
                        key: &entry.key,
                        category: &entry.category,
                        title: &entry.title,
                        status: &entry.status,
                        content: entry.content.as_deref(),
                        access_count: entry.access_count,
                        exec_prompt: entry.exec_prompt.as_deref(),
                        priority: entry.priority,
                        lane: entry.lane.as_deref(),
                        tags: entry.tags.as_deref(),
                    };
                    match extract_field(&view, f, compact) {
                        Ok(val) => {
                            if let Err(io_err) = print_or_exit(&val) {
                                return into_exit_code(io_err);
                            }
                            return 0;
                        }
                        Err(err_msg) => {
                            if let Err(io_err) = ewrite_or_exit(&format!("error: {err_msg}")) {
                                return into_exit_code(io_err);
                            }
                            return 1;
                        }
                    }
                }
                let head = format!(
                    "[{}] {} — {} (status: {})",
                    entry.category, entry.key, entry.title, entry.status
                );
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                if let Some(content) = entry.content {
                    let final_content = if compact {
                        kavach_toon::compact::compress(&content, kavach_toon::compact::Level::Lite)
                    } else {
                        content
                    };
                    let body = format!("---\n{final_content}");
                    if let Err(io_err) = print_or_exit(&body) {
                        return into_exit_code(io_err);
                    }
                }
            }
            return 0;
        }
        Ok(_) => {
            let msg = format!("not found: [{category}] {key}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            let msg = format!("rpc error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    runtime.block_on(async {
        // Phase 6: direct open via TursoStore (libSQL file — no lock contention).
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        match store.get_by_key(project_slug, category, key).await {
            Ok(Some(entry)) => {
                let cat_str = entry.category.as_deref().unwrap_or("unknown");
                let status_str = entry.entry_status.as_deref().unwrap_or("todo");
                if let Some(f) = field {
                    let view = FieldEntryView {
                        key: &entry.entry_key,
                        category: cat_str,
                        title: &entry.title,
                        status: status_str,
                        content: Some(&entry.content),
                        access_count: entry.access_count.unwrap_or(0),
                        exec_prompt: entry.exec_prompt.as_deref(),
                        priority: entry.priority,
                        lane: entry.lane.as_deref(),
                        tags: entry.tags.as_deref(),
                    };
                    match extract_field(&view, f, compact) {
                        Ok(val) => {
                            if let Err(io_err) = print_or_exit(&val) {
                                return into_exit_code(io_err);
                            }
                            return 0;
                        }
                        Err(err_msg) => {
                            if let Err(io_err) = ewrite_or_exit(&format!("error: {err_msg}")) {
                                return into_exit_code(io_err);
                            }
                            return 1;
                        }
                    }
                }
                let head = format!(
                    "[{}] {} — {}",
                    cat_str, entry.entry_key, entry.title
                );
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                let status_line = format!(
                    "status: {} | entry_status: {} | access: {}",
                    entry.status.as_deref().unwrap_or("todo"),
                    status_str,
                    entry.access_count.unwrap_or(0)
                );
                if let Err(io_err) = print_or_exit(&status_line) {
                    return into_exit_code(io_err);
                }
                if full {
                    if let Some(tags) = &entry.tags {
                        let tags_line = format!("tags: {}", tags.join(", "));
                        if let Err(io_err) = print_or_exit(&tags_line) {
                            return into_exit_code(io_err);
                        }
                    }
                    if let Some(decay) = entry.decay_score {
                        let decay_line = format!("decay: {decay:.2}");
                        if let Err(io_err) = print_or_exit(&decay_line) {
                            return into_exit_code(io_err);
                        }
                    }
                }
                if !entry.content.is_empty() {
                    if let Err(io_err) = print_or_exit("---") {
                        return into_exit_code(io_err);
                    }
                    let final_content = if compact {
                        kavach_toon::compact::compress(&entry.content, kavach_toon::compact::Level::Lite)
                    } else {
                        entry.content
                    };
                    if let Err(io_err) = print_or_exit(&final_content) {
                        return into_exit_code(io_err);
                    }
                }
                0
            }
            Ok(None) => {
                let msg = format!(
                    "error: entry not found: project={project_slug} category={category} key={key}"
                );
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::want_full_content;

    #[test]
    fn default_get_is_full() {
        assert!(want_full_content(false, false));
    }
    #[test]
    fn snippet_opts_into_short() {
        assert!(!want_full_content(false, true));
    }
    #[test]
    fn full_wins_over_snippet() {
        assert!(want_full_content(true, true));
    }
}
