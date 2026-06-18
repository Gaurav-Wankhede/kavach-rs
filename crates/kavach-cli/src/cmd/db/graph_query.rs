// split: intentional - cohesive CLI graph-query command (named lookup + list mode)
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(entity_type: Option<&str>, name: Option<&str>, limit: usize) -> i32 {
    // RPC-first: route through the daemon so the CLI never opens a competing
    // RocksDB handle (single-writer invariant — decision `rocksdb-lock-fix`).
    // The daemon returns pre-rendered lines so output is byte-identical to
    // the direct-DB fallback path below.
    match super::rpc_client::graph_query(entity_type, name, limit) {
        Ok(r) if r.success => {
            for line in &r.lines {
                if let Err(io_err) = print_or_exit(line) {
                    return into_exit_code(io_err);
                }
            }
            // List mode reproduces the footer; named mode leaves it absent
            // (name.is_some() ⇒ no footer, matching the direct path).
            if name.is_none() {
                let footer = format!("--- {}/{} entities shown ---", r.shown, r.total);
                if let Err(io_err) = print_or_exit(&footer) {
                    return into_exit_code(io_err);
                }
            }
            return 0;
        }
        Ok(r) => {
            let msg = super::rpc_client::or_str(r.error, "unknown");
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
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("db open failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Some(n) = name {
            let etype = entity_type.unwrap_or("skill");
            return query_named(&db, etype, n).await;
        }
        query_list(&db, entity_type, limit).await
    })
}

async fn query_named(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    etype: &str,
    n: &str,
) -> i32 {
    let entity = match kavach_surreal::graph_find_entity(db, etype, n).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            let msg = format!("entity not found: {etype}/{n}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!("graph find failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let Some(entity_id) = entity.id else {
        if let Err(io_err) = ewrite_or_exit("entity has no id") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let head = format!("[{etype}] {n} (id={entity_id:?})");
    if let Err(io_err) = print_or_exit(&head) {
        return into_exit_code(io_err);
    }
    match kavach_surreal::graph_get_related(db, &entity_id, 200).await {
        Ok(rows) => {
            if rows.is_empty() {
                if let Err(io_err) = print_or_exit("  (no edges)") {
                    return into_exit_code(io_err);
                }
            } else {
                for r in &rows {
                    let edge = format!(
                        "  ──{}──► [{}] {}",
                        r.rel_type, r.target.entity_type, r.target.name
                    );
                    if let Err(io_err) = print_or_exit(&edge) {
                        return into_exit_code(io_err);
                    }
                }
            }
            0
        }
        Err(e) => {
            let msg = format!("graph query failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

async fn query_list(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    entity_type: Option<&str>,
    limit: usize,
) -> i32 {
    match kavach_surreal::graph_list_entities(db, entity_type).await {
        Ok(entities) => {
            for e in entities.iter().take(limit) {
                let id_str =
                    e.id.as_ref()
                        .map_or_else(|| "-".into(), |id| format!("{id:?}"));
                let line = match &e.properties {
                    Some(serde_json::Value::String(s)) if !s.is_empty() => {
                        format!("[{}] {} (id={id_str}) {s}", e.entity_type, e.name)
                    }
                    Some(p) => format!("[{}] {} (id={id_str}) {p}", e.entity_type, e.name),
                    None => format!("[{}] {} (id={id_str})", e.entity_type, e.name),
                };
                if let Err(io_err) = print_or_exit(&line) {
                    return into_exit_code(io_err);
                }
            }
            let total = entities.len();
            let shown = total.min(limit);
            let footer = format!("--- {shown}/{total} entities shown ---");
            if let Err(io_err) = print_or_exit(&footer) {
                return into_exit_code(io_err);
            }
            0
        }
        Err(e) => {
            let msg = format!("graph list failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}
