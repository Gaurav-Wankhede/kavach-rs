//! `kavach db citation {add,list}` — official-docs context pinned to a
//! (project, entry-key) for research-consumption guard and citation graph.
//! SOURCE: <https://surrealdb.com/docs/surrealql/statements/define/table>
use super::rpc_client;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::turso::entries::EntryStore;

/// Pin an official-docs citation to a (project, entry-key).
pub(super) fn add(project: &str, entry_key: &str, name: &str, slug: &str, url: &str) -> i32 {
    // RPC-first
    let metadata = vec![kavach_db::CitationMeta::new(
        slug.to_owned(),
        url.to_owned(),
    )];
    match rpc_client::citation_add(project, entry_key, name, metadata) {
        Ok(_) => {
            if let Err(io_err) = print_or_exit(&format!(
                "ok — citation '{name}' pinned to {project}/{entry_key}"
            )) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Err(e) if rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            if let Err(io_err) = ewrite_or_exit(&format!("error: {e}")) {
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
        let store = match rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        if let Err(e) = store
            .upsert_citation(project, entry_key, name, slug, url)
            .await
        {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        if let Err(io_err) = print_or_exit(&format!(
            "ok — citation '{name}' pinned to {project}/{entry_key}"
        )) {
            return into_exit_code(io_err);
        }
        0
    })
}

/// List every citation for a project (newest-updated first).
pub(super) fn list(project: &str) -> i32 {
    // RPC-first
    match rpc_client::citation_list(project) {
        Ok(items) if items.is_empty() => {
            if let Err(io_err) = print_or_exit(&format!("(no citations for {project})")) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(items) => {
            for item in items {
                let meta = item.metadata.first();
                let slug = meta.map_or("", |m| m.slug.as_str());
                let url = meta.map_or("", |m| m.url.as_str());
                println!(
                    "{:<20} {:<12} {:<30} {}",
                    item.entry_key, slug, item.name, url
                );
            }
            return 0;
        }
        Err(e) if rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            if let Err(io_err) = ewrite_or_exit(&format!("error: {e}")) {
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
        let store = match rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        let rows = match store.list_citations(project).await {
            Ok(rows) => rows,
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        if rows.is_empty() {
            if let Err(io_err) = print_or_exit(&format!("(no citations for {project})")) {
                return into_exit_code(io_err);
            }
            return 0;
        }

        for row in rows {
            println!(
                "{:<20} {:<12} {:<30} {}",
                row.entry_key, row.slug, row.name, row.url
            );
        }
        0
    })
}
