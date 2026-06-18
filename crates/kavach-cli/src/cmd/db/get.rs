use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

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
pub(super) fn run(project_slug: &str, category: &str, key: &str, full: bool, snippet: bool) -> i32 {
    match super::rpc_client::get(project_slug, category, key, want_full_content(full, snippet)) {
        Ok(result) if result.found => {
            if let Some(entry) = result.entry {
                let head = format!(
                    "[{}] {} — {} (status: {})",
                    entry.category, entry.key, entry.title, entry.status
                );
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                if let Some(content) = entry.content {
                    let body = format!("---\n{content}");
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
        // Resilient open — closes the daemon-restart TOCTOU
        // (`rca.db-event-daemon-restart-race`): retry the lock-acquiring open
        // (bounded) instead of trusting the socket proxy; a genuine stale
        // lock still surfaces after the backoff exhausts.
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open SurrealDB: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let msg = format!("error: project not found: {project_slug}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        let Some(project_id) = project.id else {
            if let Err(io_err) = ewrite_or_exit("error: project missing id") {
                return into_exit_code(io_err);
            }
            return 1;
        };

        match kavach_surreal::get_by_key(&db, category, &project_id, key).await {
            Ok(Some(entry)) => {
                let head = format!(
                    "[{}] {} — {}",
                    entry.category_str(),
                    entry.entry_key,
                    entry.title
                );
                if let Err(io_err) = print_or_exit(&head) {
                    return into_exit_code(io_err);
                }
                let status_line = format!(
                    "status: {} | entry_status: {} | access: {}",
                    entry.status_str(),
                    entry.entry_status_str(),
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
                    if let Err(io_err) = print_or_exit(&entry.content) {
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
