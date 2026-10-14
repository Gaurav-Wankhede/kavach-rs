//! `kavach db concept {add,link,search}` — global L0 concept graph (cross-project).
//! L0 concepts are canon; L1 entities bridge to them via `bridge-create`.
//! SOURCE: <https://database.com/docs/databaseql/statements/define/table>
use super::rpc_client;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::db::graph::GraphStore;

/// Insert or update a global L0 concept.
pub(super) fn add(
    name: &str,
    display: &str,
    desc: &str,
    tags: Option<&str>,
    sources: Option<&str>,
) -> i32 {
    let tags_vec: Vec<String> = tags
        .map(|t| t.split(',').map(str::trim).map(str::to_owned).collect())
        .unwrap_or_default();
    let sources_vec: Vec<String> = sources
        .map(|s| s.split(',').map(str::trim).map(str::to_owned).collect())
        .unwrap_or_default();
    let tags_vec_for_rpc = tags_vec.clone();
    let sources_vec_for_rpc = sources_vec.clone();
    match rpc_client::concept_add(name, display, desc, tags_vec_for_rpc, sources_vec_for_rpc) {
        Ok(_) => {
            if let Err(io_err) = print_or_exit(&format!("ok — concept '{name}' upserted")) {
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
        // Phase 6: direct open via DatabaseStore (libSQL file — no lock contention).
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

        let properties = format!(
            "{{\"desc\": \"{desc}\", \"tags\": {tags_vec:?}, \"sources\": {sources_vec:?}}}"
        );
        let entity = kavach_db::db::graph::GraphEntity::new(
            name,
            "concept",
            display,
            Some(properties),
            None,
            None,
        );
        if let Err(e) = store.upsert_entity(&entity).await {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        if let Err(io_err) = print_or_exit(&format!("ok — concept '{name}' upserted")) {
            return into_exit_code(io_err);
        }
        0
    })
}

/// Relate two L0 concepts via an ontology edge.
pub(super) fn link(from: &str, edge: &str, to: &str) -> i32 {
    match rpc_client::concept_link(from, edge, to) {
        Ok(_) => {
            if let Err(io_err) =
                print_or_exit(&format!("ok — edge '{edge}' from '{from}' to '{to}'"))
            {
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
        // Phase 6: direct open via DatabaseStore (libSQL file — no lock contention).
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

        if let Err(e) = store.relate_dynamic(from, to, edge, 1.0).await {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        if let Err(io_err) = print_or_exit(&format!("ok — edge '{edge}' from '{from}' to '{to}'"))
        {
            return into_exit_code(io_err);
        }
        0
    })
}

/// Search haystack for the direct-DB fallback: the parsed text fields when the
/// properties JSON has the canonical shape, else the raw string (recall-safe:
/// never match on nothing just because the shape drifted).
fn fallback_search_text(props_json: Option<&str>) -> String {
    let mut text = String::new();
    let mut push = |part: &str| {
        if !part.is_empty() {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(part);
        }
    };
    match props_json.and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok()) {
        Some(v) if v.is_object() => {
            if let Some(desc) = v.get("desc").and_then(|d| d.as_str()) {
                push(desc);
            }
            for key in ["tags", "sources"] {
                if let Some(items) = v.get(key).and_then(|a| a.as_array()) {
                    for item in items.iter().filter_map(|i| i.as_str()) {
                        push(item);
                    }
                } else if let Some(single) = v.get(key).and_then(|s| s.as_str()) {
                    push(single);
                }
            }
        }
        _ => {
            if let Some(raw) = props_json {
                push(raw);
            }
        }
    }
    text
}

/// Full-text (BM25) search over concept descriptions.
pub(super) fn search(query: &str, limit: usize) -> i32 {
    match rpc_client::concept_search(query, limit) {
        Ok(items) if items.is_empty() => {
            if let Err(io_err) = print_or_exit(&format!("(no concepts match '{query}')")) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(items) => {
            for item in items {
                let desc = item
                    .properties
                    .as_ref()
                    .and_then(|v| v.get("desc"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                println!("{:<24}  {}", item.name, desc);
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
        // Phase 6: direct open via DatabaseStore (libSQL file — no lock contention).
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

        let entities = match store.list_entities(Some("concept")).await {
            Ok(entities) => entities,
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        // Whole-word match (shared matcher with the RPC path): the query is
        // lexed once, and every row matches on token spans — no per-row
        // lowercased copies, no substring hits ("auth" vs "author").
        let words = kavach_db::graph_concept_query_words(query);
        if words.is_empty() {
            if let Err(io_err) = print_or_exit(&format!("(no concepts match '{query}')")) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        let filtered: Vec<_> = entities
            .into_iter()
            .filter(|e| {
                kavach_db::graph_concept_text_matches(
                    &e.name,
                    &fallback_search_text(e.properties.as_deref()),
                    &words,
                )
            })
            .take(limit)
            .collect();

        if filtered.is_empty() {
            if let Err(io_err) = print_or_exit(&format!("(no concepts match '{query}')")) {
                return into_exit_code(io_err);
            }
            return 0;
        }

        for entity in filtered {
            let desc = entity
                .properties
                .as_ref()
                .and_then(|p| {
                    serde_json::from_str::<serde_json::Value>(p)
                        .ok()
                        .and_then(|v| v.get("desc").and_then(|d| d.as_str().map(str::to_owned)))
                })
                .unwrap_or_default();
            println!("{:<24}  {}", entity.name, desc);
        }
        0
    })
}
